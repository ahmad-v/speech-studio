#!/usr/bin/env python3
"""Assemble a pinned speech model from an already-resolved local file.

This tool has no network access or download code. Obtain the model separately,
then choose the matching variant and pass the local file path.
"""

import argparse
import hashlib
import json
import shutil
import tempfile
import zipfile
from pathlib import Path


SOURCE = Path(__file__).resolve().parent / "whisper-model"
DEFAULT_MODEL = "whisper-tiny-en"
MANIFESTS = {
    "whisper-tiny-en": SOURCE / "manifest.json",
    "whisper-tiny-multilingual": SOURCE / "manifest-multilingual.json",
}


def load_manifest(model_id=DEFAULT_MODEL):
    try:
        path = MANIFESTS[model_id]
    except KeyError as error:
        raise ValueError(f"unsupported model ID: {model_id}") from error
    manifest = json.loads(path.read_text(encoding="utf-8"))
    return manifest, path


def copy_verified(model_path, output, manifest):
    digest = hashlib.sha256()
    count = 0
    with model_path.open("rb") as model:
        while chunk := model.read(1024 * 1024):
            count += len(chunk)
            if count > manifest["bytes"]:
                raise ValueError("local model grew beyond the pinned length")
            digest.update(chunk)
            output.write(chunk)
    if count != manifest["bytes"] or digest.hexdigest() != manifest["sha256"]:
        raise ValueError("local model does not match the pinned SHA-256")


def stage_model(model, bundle, model_id=DEFAULT_MODEL):
    manifest, manifest_path = load_manifest(model_id)
    folder = bundle / "engines" / manifest["id"]
    folder.mkdir(parents=True)
    try:
        with (folder / manifest["file"]).open("xb") as output:
            copy_verified(model, output, manifest)
        shutil.copy2(manifest_path, folder / "manifest.json")
        shutil.copy2(SOURCE / "LICENSE-MIT", folder / "LICENSE-MIT")
    except (OSError, ValueError):
        shutil.rmtree(folder)
        raise
    return folder, manifest


def build_pack(model, output, model_id=DEFAULT_MODEL):
    manifest, manifest_path = load_manifest(model_id)
    if not model.is_file() or model.stat().st_size != manifest["bytes"]:
        raise ValueError("local model file does not match the pinned length")

    folder = Path("engines") / manifest["id"]
    model_member = (folder / manifest["file"]).as_posix()
    with tempfile.TemporaryFile() as staged:
        with zipfile.ZipFile(
            staged, "w", compression=zipfile.ZIP_STORED, strict_timestamps=False
        ) as pack:
            pack.write(manifest_path, (folder / "manifest.json").as_posix())
            pack.write(SOURCE / "LICENSE-MIT", (folder / "LICENSE-MIT").as_posix())
            with pack.open(model_member, "w") as member:
                copy_verified(model, member, manifest)

        staged.seek(0)
        with output.open("xb") as destination:
            shutil.copyfileobj(staged, destination)
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("model", type=Path, help="previously obtained local model file")
    parser.add_argument("output", type=Path, help="new ZIP pack path (must not exist)")
    parser.add_argument(
        "--model-id",
        choices=tuple(MANIFESTS),
        default=DEFAULT_MODEL,
        help="select a pinned model; defaults to the existing English Tiny pack",
    )
    args = parser.parse_args()
    try:
        build_pack(args.model, args.output, args.model_id)
    except (OSError, ValueError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
