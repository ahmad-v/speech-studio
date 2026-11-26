"""Collect Cargo dependency license evidence for a default-feature preview."""
import hashlib
import json
import os
import shutil
import subprocess
from pathlib import Path


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def collect(repo, bundle, target, binary):
    metadata = json.loads(subprocess.check_output(
        [os.environ.get("CARGO", "cargo"), "metadata", "--locked",
         "--filter-platform", target, "--format-version", "1"],
        cwd=repo, encoding="utf-8",
    ))
    resolve = metadata["resolve"]
    root_id = resolve["root"]
    if root_id is None:
        raise SystemExit("Expected an application root in Cargo metadata")
    nodes = {node["id"]: node for node in resolve["nodes"]}
    packages = {package["id"]: package for package in metadata["packages"]}
    pending = [root_id]
    visited = set()
    while pending:
        node_id = pending.pop()
        if node_id in visited:
            continue
        visited.add(node_id)
        pending.extend(
            edge["pkg"] for edge in nodes[node_id]["deps"]
            if any(kind["kind"] != "dev" for kind in edge["dep_kinds"])
        )

    supplemental_dir = repo / "packaging/upstream-licenses"
    supplemental = {
        (entry["name"], entry["version"]): entry
        for entry in json.loads((supplemental_dir / "sources.json").read_text(encoding="utf-8"))
    }
    notices = bundle / "third-party"
    notices.mkdir()
    entries = []
    for package_id in sorted(visited - {root_id}):
        package = packages[package_id]
        package_root = Path(package["manifest_path"]).parent.resolve()
        prefix = f'{package["name"]}-{package["version"]}'
        source_id = hashlib.sha256(package_id.encode()).hexdigest()[:12]
        copy_root = notices / f"{prefix}-{source_id}"
        files = {
            path for path in package_root.iterdir()
            if path.is_file() and path.name.upper().startswith(
                ("LICENSE", "LICENCE", "COPYING", "COPYRIGHT", "NOTICE", "UNLICENSE")
            )
        }
        if package["license_file"]:
            path = package_root / package["license_file"]
            if path.is_file():
                files.add(path)
        copied = []
        for path in sorted(files):
            if not path.resolve().is_relative_to(package_root):
                continue
            copy_root.mkdir(exist_ok=True)
            dest = copy_root / path.name
            if dest.exists():
                raise SystemExit(f"Duplicate notice name in {prefix}: {path.name}")
            shutil.copy2(path, dest)
            copied.append(dest.relative_to(bundle).as_posix())

        source_urls = []
        if not copied and (entry := supplemental.get((package["name"], package["version"]))):
            source = package["source"] or ""
            metadata_file = package_root / ".cargo_vcs_info.json"
            if source.startswith("git+https://github.com/ahmad-v/localwright?"):
                revision = source.rpartition("#")[2]
            elif source.startswith("registry+") and metadata_file.is_file():
                revision = json.loads(metadata_file.read_text(encoding="utf-8"))["git"]["sha1"]
            else:
                revision = None
            if revision == entry["revision"]:
                copy_root.mkdir(exist_ok=True)
                for notice in entry["files"]:
                    dest = copy_root / notice["name"]
                    shutil.copy2(supplemental_dir / notice["name"], dest)
                    copied.append(dest.relative_to(bundle).as_posix())
                    source_urls.append(notice["url"])

        asset_review = []
        icon_font = package_root / "fonts/Iced-Icons.ttf"
        if package["name"] == "iced_graphics" and icon_font.is_file():
            icon_digest = sha256(icon_font)
            icon_evidence = {
                "path": "fonts/Iced-Icons.ttf",
                "sha256": icon_digest,
                "status": "Review required for this font version",
            }
            if icon_digest == "89ce457409339061b0333d2ed374e4e3666a3c673304505963078d67b61efb77":
                icon_evidence["status"] = "Fontello and Font Awesome glyph attribution recorded"
                copy_root.mkdir(exist_ok=True)
                icon_evidence["notice_files"] = []
                for name in ("ICED-ICON-NOTICE.txt", "OFL-1.1.txt"):
                    dest = copy_root / name
                    shutil.copy2(supplemental_dir / name, dest)
                    icon_evidence["notice_files"].append(dest.relative_to(bundle).as_posix())
            asset_review.append(icon_evidence)
        entries.append({
            "name": package["name"],
            "version": package["version"],
            "source": package["source"],
            "repository": package["repository"],
            "declared_license": package["license"],
            "notice_files": copied,
            "supplemental_notice_sources": source_urls,
            "asset_review": asset_review,
        })

    (bundle / "DEPENDENCY_LICENSES.json").write_text(
        json.dumps({
            "schema_version": 1,
            "target": target,
            "configuration": "Cargo default features with normal and build dependency edges",
            "cargo_lock_sha256": sha256(repo / "Cargo.lock"),
            "executable_sha256": sha256(binary),
            "packages": entries,
        }, indent=2) + "\n",
        encoding="utf-8",
    )
