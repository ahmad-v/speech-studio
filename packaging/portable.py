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
from tts_model_pack import MANIFEST as TTS_MANIFEST, verify_pack_directory
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
    parser.add_argument(
        "--tts-pack",
        type=Path,
        help="previously assembled, verified offline Kitten voice pack directory",
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
    editions = []
    if args.whisper_model:
        editions.append(f"asr-{manifest['id']}")
    if args.tts_pack:
        try:
            verify_pack_directory(args.tts_pack)
        except (OSError, ValueError) as error:
            parser.error(str(error))
        editions.append(f"tts-{TTS_MANIFEST['id']}")
    edition = f"preview-{'-'.join(editions)}" if editions else "preview"
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
        if args.tts_pack:
            destination = bundle / "engines" / TTS_MANIFEST["id"]
            shutil.copytree(args.tts_pack, destination)
            verify_pack_directory(destination)
        features = tuple(
            feature
            for feature, enabled in (("asr", bool(args.whisper_model)), ("tts", bool(args.tts_pack)))
            if enabled
        )
        collect(R, bundle, args.target, args.binary, features=features)
        payload_status = (
            "Unsigned portable development build. Launch the application in this directory.\n"
        )
        if args.whisper_model:
            model_name = manifest.get("model_family", "Tiny English")
            payload_status += (
                f"Includes the pinned {model_name} ASR model. "
                "Platform libraries may be required.\n"
            )
        if args.tts_pack:
            payload_status += (
                "Includes the pinned Kitten Nano English text-to-speech pack. "
                "Synthesis runs locally without network access.\n"
            )
        if not args.whisper_model and not args.tts_pack:
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
        help_tts = any(line.startswith("synthesize-speech:") for line in result.stdout.splitlines())
        if help_tts != bool(args.tts_pack):
            raise SystemExit("TTS-enabled binary and private speech pack must be supplied together")
        if args.tts_pack:
            verified = subprocess.run(
                [command, "--cli", "verify-tts-pack", "", "", ""],
                cwd=command.parent,
                capture_output=True,
                text=True,
                encoding="utf-8",
                check=True,
                timeout=60,
            )
            if "Verified" not in verified.stdout:
                raise SystemExit("Packaged TTS model did not pass application SHA-256 verification")
            with tempfile.TemporaryDirectory(prefix="speech-tts-smoke-") as directory:
                output = Path(directory) / "offline-smoke.wav"
                result = subprocess.run(
                    [
                        command,
                        "--cli",
                        "synthesize-speech",
                        "",
                        output,
                        '{"text":"Offline speech synthesis smoke test.","voice":"Jasper","speed":1.0}',
                    ],
                    cwd=command.parent,
                    capture_output=True,
                    text=True,
                    encoding="utf-8",
                    check=True,
                    timeout=180,
                )
                if not output.is_file() or output.stat().st_size <= 44:
                    raise SystemExit(f"Packaged TTS smoke test wrote no audio: {result.stderr}")
                import wave
                with wave.open(str(output), "rb") as sample:
                    if (sample.getnchannels(), sample.getsampwidth(), sample.getframerate()) != (1, 2, 24000):
                        raise SystemExit("Packaged TTS smoke test returned an unexpected WAV format")
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
