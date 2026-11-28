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


def create_packs(output_dir):
    output_dir = Path(output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="speech-models-") as temporary:
        work = Path(temporary)
        outputs = []
        for model_id in MANIFESTS:
            manifest, _ = load_manifest(model_id)
            model = download_verified(model_id, work / manifest["file"])
            archive = output_dir / f"{model_id}.zip"
            build_pack(model, archive, model_id)
            outputs.append(archive)
    return outputs


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output_dir", type=Path)
    args = parser.parse_args()
    try:
        for archive in create_packs(args.output_dir):
            print(archive)
    except (OSError, ValueError) as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
