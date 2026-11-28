#!/usr/bin/env python3
"""Build and smoke-test an unsigned portable development bundle."""
import argparse
import hashlib
import shutil
import subprocess
import tarfile
import tempfile
import zipfile
from pathlib import Path

from dependency_notices import collect
from package import CFG, R, TARGETS, check_binary, payload, validate_host
from whisper_model_pack import DEFAULT_MODEL, MANIFESTS, load_manifest, stage_model


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", choices=TARGETS, required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument(
        "--whisper-model",
        type=Path,
        help="previously obtained local model for an ASR-enabled build",
    )
    parser.add_argument(
        "--whisper-model-id",
        choices=tuple(MANIFESTS),
        default=DEFAULT_MODEL,
        help="pinned model variant to package with --whisper-model",
    )
    args = parser.parse_args()
    if args.whisper_model_id != DEFAULT_MODEL and not args.whisper_model:
        parser.error("--whisper-model-id requires --whisper-model")

    validate_host(args.target)
    name = CFG["name"]
    windows = "windows" in args.target
    executable = name + (".exe" if windows else "")
    if args.binary.name != executable or not args.binary.is_file():
        parser.error(f"--binary must point to the built {executable}")
    check_binary(args.binary, args.target)

    manifest, _ = load_manifest(args.whisper_model_id)
    edition = f"preview-asr-{manifest['id']}" if args.whisper_model else "preview"
    stem = f"{name}-{CFG['package_version']}-{args.target}-{edition}"
    with tempfile.TemporaryDirectory(prefix="studio-portable-") as directory:
        temp = Path(directory)
        bundle = temp / name
        payload(bundle, args.binary)
        if args.whisper_model:
            try:
                _, manifest = stage_model(args.whisper_model, bundle, args.whisper_model_id)
            except (OSError, ValueError) as error:
                parser.error(str(error))
        collect(R, bundle, args.target, args.binary,
                features=("asr",) if args.whisper_model else ())
        payload_status = (
            "Unsigned portable development build. Launch the application in this directory.\n"
        )
        if args.whisper_model:
            model_name = manifest.get("model_family", "Tiny English")
            payload_status += (
                f"Includes the pinned {model_name} ASR model. "
                "Platform libraries may be required.\n"
            )
        else:
            payload_status += (
                "This bundle contains no private engines or speech model. "
                "Platform libraries may be required.\n"
            )
        (bundle / "PAYLOAD_STATUS.txt").write_text(payload_status, encoding="utf-8")
        extracted = temp / "extracted preview"
        if windows:
            archive = temp / f"{stem}.zip"
            with zipfile.ZipFile(archive, "x", zipfile.ZIP_DEFLATED, strict_timestamps=False) as output:
                for path in sorted(bundle.rglob("*")):
                    if path.is_file():
                        output.write(path, path.relative_to(temp).as_posix())
            with zipfile.ZipFile(archive) as output:
                output.extractall(extracted)
        else:
            archive = temp / f"{stem}.tar.gz"
            with tarfile.open(archive, "x:gz") as output:
                output.add(bundle, arcname=name)
            with tarfile.open(archive) as output:
                output.extractall(extracted, filter="data")

        command = (extracted / name / executable).resolve()
        check_binary(command, args.target)
        result = subprocess.run(
            [command, "--cli", "--help"],
            cwd=extracted,
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=True,
            timeout=30,
        )
        if "Usage: --cli OP INPUT OUTPUT PARAMETER" not in result.stdout:
            raise SystemExit("Packaged CLI help did not return the expected usage text")

        has_asr = any(line.startswith("transcribe:") for line in result.stdout.splitlines())
        if has_asr != bool(args.whisper_model):
            raise SystemExit(
                "ASR-enabled binary and private speech model must be supplied together"
            )
        if args.whisper_model:
            installed = extracted / name / "engines" / manifest["id"] / manifest["file"]
            with installed.open("rb") as model:
                if hashlib.file_digest(model, "sha256").hexdigest() != manifest["sha256"]:
                    raise SystemExit("Packaged model failed SHA-256 verification")

        with archive.open("rb") as source:
            digest = hashlib.file_digest(source, "sha256").hexdigest()
        args.output.mkdir(parents=True, exist_ok=False)
        shutil.copy2(archive, args.output / archive.name)
        (args.output / "SHA256SUMS").write_text(
            f"{digest}  {archive.name}\n", encoding="ascii"
        )


if __name__ == "__main__":
    main()
