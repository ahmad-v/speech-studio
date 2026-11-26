#!/usr/bin/env python3
"""Assemble the pinned speech model from an already-resolved local file."""

import argparse
import hashlib
import json
import shutil
import tempfile
import zipfile
from pathlib import Path


SOURCE = Path(__file__).resolve().parent / "whisper-model"
MANIFEST = json.loads((SOURCE / "manifest.json").read_text(encoding="utf-8"))
FOLDER = Path("engines") / MANIFEST["id"]


def copy_verified(model_path, output):
    digest = hashlib.sha256()
    count = 0
    with model_path.open("rb") as model:
        while chunk := model.read(1024 * 1024):
            count += len(chunk)
            if count > MANIFEST["bytes"]:
                raise ValueError("local model grew beyond the pinned length")
            digest.update(chunk)
            output.write(chunk)
    if count != MANIFEST["bytes"] or digest.hexdigest() != MANIFEST["sha256"]:
        raise ValueError("local model does not match the pinned SHA-256")


def stage_model(model, bundle):
    folder = bundle / FOLDER
    folder.mkdir(parents=True)
    try:
        with (folder / MANIFEST["file"]).open("xb") as output:
            copy_verified(model, output)
        for name in ("manifest.json", "LICENSE-MIT"):
            shutil.copy2(SOURCE / name, folder / name)
    except (OSError, ValueError):
        shutil.rmtree(folder)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("model", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if not args.model.is_file() or args.model.stat().st_size != MANIFEST["bytes"]:
        parser.error("local model file does not match the pinned length")

    # Hash the bytes as they enter the archive so the check covers the very
    # same payload that would be distributed.
    with tempfile.TemporaryFile() as staged:
        with zipfile.ZipFile(staged, "w", compression=zipfile.ZIP_STORED,
                             strict_timestamps=False) as pack:
            for name in ("manifest.json", "LICENSE-MIT"):
                pack.write(SOURCE / name, (FOLDER / name).as_posix())
            try:
                with pack.open((FOLDER / MANIFEST["file"]).as_posix(), "w") as member:
                    copy_verified(args.model, member)
            except ValueError as error:
                parser.error(str(error))

        staged.seek(0)
        with args.output.open("xb") as output:
            shutil.copyfileobj(staged, output)


if __name__ == "__main__":
    main()
