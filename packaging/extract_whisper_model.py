#!/usr/bin/env python3
"""Extract one verified model file from a pack for offline bundle assembly."""

import argparse
import hashlib
import os
import tempfile
import zipfile
from pathlib import Path

from whisper_model_pack import MANIFESTS, SOURCE, load_manifest


def extract_model(pack_path, destination, model_id):
    manifest, manifest_path = load_manifest(model_id)
    destination = Path(destination)
    expected = f"engines/{manifest['id']}/{manifest['file']}"
    manifest_member = f"engines/{manifest['id']}/manifest.json"
    license_member = f"engines/{manifest['id']}/LICENSE-MIT"
    with zipfile.ZipFile(pack_path) as archive:
        infos = archive.infolist()
        if len(infos) != 3 or {info.filename for info in infos} != {
            expected,
            manifest_member,
            license_member,
        }:
            raise ValueError("offline model pack does not match the selected variant")
        if archive.read(manifest_member) != manifest_path.read_bytes():
            raise ValueError("offline model pack manifest differs from the pinned copy")
        if archive.read(license_member) != (SOURCE / "LICENSE-MIT").read_bytes():
            raise ValueError("offline model pack license differs from the pinned copy")
        model_info = archive.getinfo(expected)
        if model_info.file_size != manifest["bytes"]:
            raise ValueError("offline model pack size differs from its manifest")
        digest = hashlib.sha256()
        total = 0
        destination.parent.mkdir(parents=True, exist_ok=True)
        if destination.exists():
            raise FileExistsError(destination)
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(
                mode="wb", prefix=destination.name + ".", suffix=".partial",
                dir=destination.parent, delete=False
            ) as output:
                temporary = Path(output.name)
                with archive.open(model_info) as source:
                    while chunk := source.read(1024 * 1024):
                        total += len(chunk)
                        if total > manifest["bytes"]:
                            raise ValueError("offline model pack exceeds its pinned size")
                        digest.update(chunk)
                        output.write(chunk)
            if total != manifest["bytes"] or digest.hexdigest() != manifest["sha256"]:
                raise ValueError("offline model pack SHA-256 differs from its manifest")
            os.link(temporary, destination)
            temporary.unlink()
            return destination
        except BaseException:
            if temporary is not None:
                temporary.unlink(missing_ok=True)
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack", type=Path)
    parser.add_argument("destination", type=Path)
    parser.add_argument("--model-id", choices=tuple(MANIFESTS), required=True)
    args = parser.parse_args()
    try:
        result = extract_model(args.pack, args.destination, args.model_id)
    except (OSError, ValueError, zipfile.BadZipFile) as error:
        parser.error(str(error))
    print(result)


if __name__ == "__main__":
    main()
