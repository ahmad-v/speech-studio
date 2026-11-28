"""Build a bounded, hash-pinned offline speaker diarization pack."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODEL_DIR = ROOT / "packaging" / "diarization"
MANIFEST_PATH = MODEL_DIR / "manifest.json"
MANIFEST = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
SEGMENTATION_PREFIX = "sherpa-onnx-pyannote-segmentation-3-0/"
SEGMENTATION_ARCHIVE_BYTES = 6_958_444
SEGMENTATION_ARCHIVE_SHA256 = "24615ee884c897d9d2ba09bb4d30da6bb1b15e685065962db5b02e76e4996488"
EMBEDDING_SOURCE_BYTES = 39_593_762
EMBEDDING_SOURCE_SHA256 = "f1ee61be04000b50392a63442e5dd9feaea7e81dddd494d3647cf1c3d88a1498"
MAX_ARCHIVE_BYTES = 16 * 1024 * 1024
MAX_UNPACKED_BYTES = 64 * 1024 * 1024


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_source_files(archive: Path, embedding: Path) -> None:
    if not archive.is_file() or archive.stat().st_size != SEGMENTATION_ARCHIVE_BYTES:
        raise ValueError("Pinned pyannote segmentation archive is missing or has wrong size")
    if sha256(archive) != SEGMENTATION_ARCHIVE_SHA256:
        raise ValueError("Pinned pyannote segmentation archive SHA-256 mismatch")
    if not embedding.is_file() or embedding.stat().st_size != EMBEDDING_SOURCE_BYTES:
        raise ValueError("Pinned 3D-Speaker embedding model is missing or has wrong size")
    if sha256(embedding) != EMBEDDING_SOURCE_SHA256:
        raise ValueError("Pinned 3D-Speaker embedding model SHA-256 mismatch")


def extract_pack(archive: Path, embedding: Path, destination: Path, manifest=MANIFEST) -> None:
    """Extract only the selected int8 model and copy separately licensed notices."""
    verify_source_files(archive, embedding)
    destination.mkdir(parents=True, exist_ok=False)
    expected = {item["name"]: item for item in manifest["files"]}
    copied = set()
    total = 0
    try:
        with tarfile.open(archive, mode="r:bz2") as source:
            for member in source.getmembers():
                if not member.name.startswith(SEGMENTATION_PREFIX):
                    continue
                relative = member.name[len(SEGMENTATION_PREFIX):]
                if relative == "model.int8.onnx":
                    target_name = "segmentation.int8.onnx"
                elif relative == "LICENSE":
                    target_name = "LICENSE-pyannote-MIT"
                else:
                    continue
                if target_name in copied or not member.isfile() or member.size > MAX_UNPACKED_BYTES:
                    raise ValueError("Segmentation archive contains an unsafe or duplicate required asset")
                stream = source.extractfile(member)
                if stream is None:
                    raise ValueError("Could not read a segmentation archive asset")
                output_path = destination / target_name
                digest = hashlib.sha256()
                size = 0
                with stream, output_path.open("xb") as output:
                    while chunk := stream.read(1024 * 1024):
                        size += len(chunk)
                        total += len(chunk)
                        if size > expected[target_name]["size_bytes"] or total > MAX_UNPACKED_BYTES:
                            raise ValueError("Segmentation archive exceeded its declared asset bounds")
                        digest.update(chunk)
                        output.write(chunk)
                item = expected[target_name]
                if size != item["size_bytes"] or digest.hexdigest() != item["sha256"]:
                    raise ValueError(f"Segmentation asset failed pinned verification: {target_name}")
                copied.add(target_name)

        if "segmentation.int8.onnx" not in copied or "LICENSE-pyannote-MIT" not in copied:
            raise ValueError("Segmentation archive omitted the pinned model or license")
        embedding_item = expected["speaker-embedding.onnx"]
        if embedding.stat().st_size != embedding_item["size_bytes"] or sha256(embedding) != embedding_item["sha256"]:
            raise ValueError("Speaker embedding asset failed pinned verification")
        shutil.copyfile(embedding, destination / "speaker-embedding.onnx")
        shutil.copyfile(MODEL_DIR / "LICENSE-3dspeaker-Apache-2.0", destination / "LICENSE-3dspeaker-Apache-2.0")
        shutil.copyfile(MODEL_DIR / "MODEL-CARD.md", destination / "MODEL-CARD.md")
        copied.update({"speaker-embedding.onnx", "LICENSE-3dspeaker-Apache-2.0", "MODEL-CARD.md"})
        for name in copied:
            item = expected[name]
            path = destination / name
            if path.stat().st_size != item["size_bytes"] or sha256(path) != item["sha256"]:
                raise ValueError(f"Diarization asset failed pinned verification: {name}")
        if copied != set(expected):
            raise ValueError("Diarization pack has an incomplete or unexpected asset set")
        (destination / "manifest.json").write_text(
            json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    except Exception:
        shutil.rmtree(destination, ignore_errors=True)
        raise


def create_pack(archive: Path, embedding: Path, output_dir: Path, manifest=MANIFEST) -> Path:
    output_dir.mkdir(parents=True, exist_ok=False)
    pack_dir = output_dir / "engines" / manifest["id"]
    pack_dir.parent.mkdir(parents=True, exist_ok=True)
    try:
        extract_pack(archive, embedding, pack_dir, manifest)
        output = output_dir / f"{manifest['id']}.zip"
        with zipfile.ZipFile(output, "x", compression=zipfile.ZIP_STORED) as bundle:
            for path in sorted(output_dir.rglob("*")):
                if path.is_file() and path != output:
                    bundle.write(path, path.relative_to(output_dir).as_posix())
        with zipfile.ZipFile(output) as bundle:
            names = set(bundle.namelist())
            expected = {f"engines/{manifest['id']}/{item['name']}" for item in manifest["files"]}
            expected.add(f"engines/{manifest['id']}/manifest.json")
            if names != expected:
                raise ValueError("Diarization bundle contains missing or unexpected entries")
        return output
    except Exception:
        shutil.rmtree(output_dir, ignore_errors=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("segmentation_archive", type=Path)
    parser.add_argument("embedding_model", type=Path)
    parser.add_argument("output_dir", type=Path)
    args = parser.parse_args()
    try:
        output = create_pack(args.segmentation_archive, args.embedding_model, args.output_dir)
    except (OSError, ValueError, tarfile.TarError, zipfile.BadZipFile) as error:
        parser.error(str(error))
    print(json.dumps({"pack": str(output), "sha256": sha256(output), "bytes": output.stat().st_size}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
