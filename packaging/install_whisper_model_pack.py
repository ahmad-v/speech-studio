#!/usr/bin/env python3
"""Verify and install an offline Whisper model ZIP beside Speech Studio."""

import argparse
import hashlib
import json
import os
import shutil
import stat
import tempfile
import zipfile
from pathlib import Path, PurePosixPath

from whisper_model_pack import (
    DEFAULT_MODEL,
    MANIFESTS,
    SOURCE,
    canonical_text_bytes,
    load_manifest,
)

MAX_PACK_METADATA_BYTES = 64 * 1024


def _safe_member(info, expected):
    name = PurePosixPath(info.filename)
    mode = info.external_attr >> 16
    return (
        name.as_posix() == expected
        and not name.is_absolute()
        and ".." not in name.parts
        and "\\" not in info.filename
        and not stat.S_ISLNK(mode)
        and not info.is_dir()
    )


def install_pack(pack_path, application_dir, manifest, manifest_bytes, license_bytes):
    """Validate a pack fully, then install without overwriting application data."""
    app_dir = Path(application_dir)
    if not app_dir.is_dir() or app_dir.is_symlink():
        raise ValueError("application directory must be an existing real directory")
    if not any((app_dir / name).is_file() for name in ("speech-studio", "speech-studio.exe")):
        raise ValueError("application directory does not contain Speech Studio")

    model_id = manifest["id"]
    file_name = manifest["file"]
    expected_bytes = manifest["bytes"]
    expected_hash = manifest["sha256"]
    if (
        not isinstance(model_id, str)
        or not model_id.startswith("whisper-")
        or Path(model_id).name != model_id
        or "\\" in model_id
        or not isinstance(file_name, str)
        or Path(file_name).name != file_name
        or "\\" in file_name
        or not isinstance(expected_bytes, int)
        or expected_bytes <= 0
        or not isinstance(expected_hash, str)
        or len(expected_hash) != 64
        or any(char not in "0123456789abcdefABCDEF" for char in expected_hash)
    ):
        raise ValueError("pinned model manifest has unsafe or invalid fields")

    if Path(pack_path).stat().st_size > expected_bytes + (2 * MAX_PACK_METADATA_BYTES):
        raise ValueError("model pack exceeds the pinned model size plus bounded metadata")

    root = PurePosixPath("engines") / model_id
    entries = {
        (root / "manifest.json").as_posix(): (manifest_bytes, MAX_PACK_METADATA_BYTES),
        (root / "LICENSE-MIT").as_posix(): (license_bytes, MAX_PACK_METADATA_BYTES),
        (root / file_name).as_posix(): (None, expected_bytes),
    }
    with zipfile.ZipFile(pack_path) as archive:
        infos = archive.infolist()
        names = [info.filename for info in infos]
        if len(infos) != len(entries) or len(set(names)) != len(names) or set(names) != set(entries):
            raise ValueError("model pack must contain exactly the pinned manifest, license, and model")
        by_name = {info.filename: info for info in infos}
        for name, (expected, limit) in entries.items():
            info = by_name[name]
            if not _safe_member(info, name) or info.file_size > limit:
                raise ValueError("model pack contains an unsafe or oversized entry")
            if expected is not None and archive.read(info) != expected:
                raise ValueError("model pack manifest or license does not match the pinned asset")
        model_info = by_name[(root / file_name).as_posix()]
        if model_info.file_size != expected_bytes:
            raise ValueError("model pack size does not match the pinned manifest")
        digest = hashlib.sha256()
        total = 0
        with archive.open(model_info) as source:
            while chunk := source.read(1024 * 1024):
                total += len(chunk)
                if total > expected_bytes:
                    raise ValueError("model pack exceeds the pinned model size")
                digest.update(chunk)
        if total != expected_bytes or digest.hexdigest() != expected_hash.lower():
            raise ValueError("model pack SHA-256 does not match the pinned manifest")

        engines_dir = app_dir / "engines"
        if engines_dir.is_symlink():
            raise ValueError("application engines directory must not redirect")
        engines_dir.mkdir(exist_ok=True)
        destination = engines_dir / model_id
        if destination.exists() or destination.is_symlink():
            raise FileExistsError(destination)
        staging_dir = Path(tempfile.mkdtemp(prefix=f".{model_id}.", dir=engines_dir))
        temporary_model = None
        try:
            (staging_dir / "manifest.json").write_bytes(manifest_bytes)
            (staging_dir / "LICENSE-MIT").write_bytes(license_bytes)
            with tempfile.NamedTemporaryFile(
                mode="wb", prefix="model.", suffix=".partial", dir=staging_dir, delete=False
            ) as output:
                temporary_model = Path(output.name)
                digest = hashlib.sha256()
                total = 0
                with archive.open(model_info) as source:
                    while chunk := source.read(1024 * 1024):
                        total += len(chunk)
                        digest.update(chunk)
                        output.write(chunk)
            if total != expected_bytes or digest.hexdigest() != expected_hash.lower():
                raise ValueError("model pack changed while it was being installed")
            os.chmod(temporary_model, 0o644)
            os.link(temporary_model, staging_dir / file_name)
            temporary_model.unlink()
            os.rename(staging_dir, destination)
            return destination
        except BaseException:
            if temporary_model is not None:
                temporary_model.unlink(missing_ok=True)
            shutil.rmtree(staging_dir, ignore_errors=True)
            raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack", type=Path, help="offline ZIP from the model-pack workflow")
    parser.add_argument("application_dir", type=Path, help="folder containing speech-studio executable")
    parser.add_argument("--model-id", choices=tuple(MANIFESTS), default=DEFAULT_MODEL)
    args = parser.parse_args()
    manifest, manifest_path = load_manifest(args.model_id)
    try:
        installed = install_pack(
            args.pack,
            args.application_dir,
            manifest,
            canonical_text_bytes(manifest_path),
            canonical_text_bytes(SOURCE / "LICENSE-MIT"),
        )
    except (OSError, ValueError, zipfile.BadZipFile, json.JSONDecodeError) as error:
        parser.error(str(error))
    print(f"Installed and SHA-256 verified {args.model_id} at {installed}")


if __name__ == "__main__":
    main()
