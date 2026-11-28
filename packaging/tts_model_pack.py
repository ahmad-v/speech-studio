"""Pinned asset rules and safe pack assembly for the offline English TTS voice."""
import hashlib
import argparse
import json
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "packaging" / "tts-model" / "manifest.json"
MANIFEST = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
MAX_ARCHIVE_BYTES = 32 * 1024 * 1024
MAX_EXTRACTED_BYTES = 24 * 1024 * 1024


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_archive(path: Path, manifest=MANIFEST) -> None:
    if not path.is_file() or path.stat().st_size != manifest["source_archive_size_bytes"]:
        raise ValueError("TTS model archive size does not match the pinned manifest")
    if _sha256(path) != manifest["source_archive_sha256"]:
        raise ValueError("TTS model archive SHA-256 does not match the pinned manifest")


def extract_assets(path: Path, destination: Path, manifest=MANIFEST) -> None:
    """Extract only four regular, hash-pinned files from the expected archive."""
    verify_archive(path, manifest)
    expected = {item["name"]: item for item in manifest["files"]}
    prefix = "vits-piper-en_US-ljspeech-medium-int8/"
    total = 0
    destination.mkdir(parents=True, exist_ok=False)
    try:
        with tarfile.open(path, mode="r:bz2") as archive:
            seen = set()
            for member in archive.getmembers():
                if not member.name.startswith(prefix):
                    continue
                name = member.name[len(prefix):]
                if name not in expected:
                    continue
                if name in seen or not member.isfile() or member.size > MAX_EXTRACTED_BYTES:
                    raise ValueError("TTS model archive contains an unsafe or duplicate asset")
                source = archive.extractfile(member)
                if source is None:
                    raise ValueError("Could not read a TTS model archive asset")
                target = destination / name
                with source, target.open("xb") as output:
                    digest = hashlib.sha256()
                    size = 0
                    while True:
                        chunk = source.read(1024 * 1024)
                        if not chunk:
                            break
                        size += len(chunk)
                        total += len(chunk)
                        if size > expected[name]["size_bytes"] or total > MAX_EXTRACTED_BYTES:
                            raise ValueError("TTS model archive exceeds its declared asset bounds")
                        digest.update(chunk)
                        output.write(chunk)
                if size != expected[name]["size_bytes"]:
                    raise ValueError(f"Unexpected size for TTS model asset {name}")
                if digest.hexdigest() != expected[name]["sha256"]:
                    raise ValueError(f"SHA-256 mismatch for TTS model asset {name}")
                seen.add(name)
            if seen != set(expected):
                missing = ", ".join(sorted(set(expected) - seen))
                raise ValueError(f"TTS model archive omitted required assets: {missing}")
        (destination / "manifest.json").write_text(
            json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    except Exception:
        for child in destination.iterdir():
            if child.is_file() or child.is_symlink():
                child.unlink()
        destination.rmdir()
        raise


def create_pack(archive: Path, output_dir: Path, manifest=MANIFEST) -> Path:
    output_dir.mkdir(parents=True, exist_ok=False)
    pack_root = output_dir / "engines" / manifest["id"]
    pack_root.parent.mkdir(parents=True, exist_ok=True)
    try:
        extract_assets(archive, pack_root, manifest)
        output = output_dir / f"{manifest['id']}.zip"
        with zipfile.ZipFile(output, "x", zipfile.ZIP_DEFLATED, compresslevel=6) as bundle:
            for path in sorted(output_dir.rglob("*")):
                if path.is_file() and path != output:
                    bundle.write(path, path.relative_to(output_dir).as_posix())
        with zipfile.ZipFile(output) as bundle:
            names = set(bundle.namelist())
            expected_names = {
                f"engines/{manifest['id']}/{item['name']}" for item in manifest["files"]
            } | {f"engines/{manifest['id']}/manifest.json"}
            if names != expected_names:
                raise ValueError("Assembled TTS model pack has unexpected entries")
            for item in manifest["files"]:
                name = f"engines/{manifest['id']}/{item['name']}"
                data = bundle.read(name)
                if len(data) != item["size_bytes"] or hashlib.sha256(data).hexdigest() != item["sha256"]:
                    raise ValueError(f"Packaged TTS asset failed verification: {name}")
        return output
    except Exception:
        import shutil

        shutil.rmtree(output_dir, ignore_errors=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path, help="pinned sherpa-onnx voice archive")
    parser.add_argument("output_dir", type=Path, help="new output directory")
    arguments = parser.parse_args()
    try:
        output = create_pack(arguments.archive, arguments.output_dir)
    except (OSError, ValueError, tarfile.TarError) as error:
        parser.error(str(error))
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
