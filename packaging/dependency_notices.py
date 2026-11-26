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

        asset_review = []
        icon_font = package_root / "fonts/Iced-Icons.ttf"
        if package["name"] == "iced_graphics" and icon_font.is_file():
            asset_review.append({
                "path": "fonts/Iced-Icons.ttf",
                "sha256": sha256(icon_font),
                "status": "Original glyph licenses and notices require review",
            })
        entries.append({
            "name": package["name"],
            "version": package["version"],
            "source": package["source"],
            "repository": package["repository"],
            "declared_license": package["license"],
            "notice_files": copied,
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
