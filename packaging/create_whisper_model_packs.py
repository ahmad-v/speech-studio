#!/usr/bin/env python3
"""Download only manifest-pinned Whisper weights and assemble offline packs."""

import argparse
import hashlib
import os
import tempfile
import urllib.parse
import urllib.request
from pathlib import Path

from whisper_model_pack import MANIFESTS, build_pack, load_manifest


def download_verified(model_id, destination, opener=urllib.request.urlopen):
    manifest, _ = load_manifest(model_id)
    source = urllib.parse.urlsplit(manifest["source"])
    if (
        source.scheme != "https"
        or source.hostname != "huggingface.co"
        or f"/resolve/{manifest['source_revision']}/" not in source.path
    ):
        raise ValueError("model source is not the pinned HTTPS Hugging Face revision")

    destination = Path(destination)
    if destination.exists():
        raise FileExistsError(destination)
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = None
    digest = hashlib.sha256()
    size = 0
    request = urllib.request.Request(
        manifest["source"], headers={"User-Agent": "SpeechStudioModelPack/1"}
    )
    try:
        with opener(request, timeout=120) as response:
            with tempfile.NamedTemporaryFile(
                mode="wb",
                prefix=destination.name + ".",
                suffix=".partial",
                dir=destination.parent,
                delete=False,
            ) as output:
                partial = Path(output.name)
                while chunk := response.read(1024 * 1024):
                    size += len(chunk)
                    if size > manifest["bytes"]:
                        raise ValueError("download exceeded the pinned model size")
                    digest.update(chunk)
                    output.write(chunk)
        if size != manifest["bytes"] or digest.hexdigest() != manifest["sha256"]:
            raise ValueError("download does not match the manifest size and SHA-256")
        os.link(partial, destination)
        partial.unlink()
    except BaseException:
        if partial is not None:
            partial.unlink(missing_ok=True)
        raise
    return destination


def create_packs(output_dir, model_ids=None):
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    model_ids = tuple(MANIFESTS) if model_ids is None else tuple(model_ids)
    if not model_ids or any(model_id not in MANIFESTS for model_id in model_ids):
        raise ValueError("select one or more supported model IDs")
    if len(set(model_ids)) != len(model_ids):
        raise ValueError("model IDs must be unique")
    with tempfile.TemporaryDirectory(prefix="speech-models-") as temporary:
        work = Path(temporary)
        outputs = []
        for model_id in model_ids:
            manifest, _ = load_manifest(model_id)
            model = download_verified(model_id, work / manifest["file"])
            archive = output_dir / f"{model_id}.zip"
            build_pack(model, archive, model_id)
            outputs.append(archive)
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path)
    parser.add_argument(
        "--model-id",
        action="append",
        choices=tuple(MANIFESTS),
        help="build only selected models; repeat to include multiple variants",
    )
    args = parser.parse_args()
    try:
        for archive in create_packs(args.output_dir, args.model_id):
            print(archive)
    except (OSError, ValueError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
