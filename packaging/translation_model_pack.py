"""Build or install the pinned, optional offline English-to-French model pack.

Only small tooling/metadata is committed. The 301 MB weights are downloaded
from a fixed upstream Git revision for package jobs and verified before use.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import shutil
import tempfile
import urllib.parse
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PACK_ID = "opus-mt-en-fr-v1"
MODEL_REV = "dd7f6540a7a48a7f4db59e5c0b9c42c8eea67f18"
MODEL_REPO = "Helsinki-NLP/opus-mt-en-fr"
MODEL_LICENSE = "Apache-2.0 (model card metadata at the pinned upstream revision)"
TOKENIZER_REPO = MODEL_REPO
TORCH_VERSION = "2.5.1+cpu"
SAFETENSORS_VERSION = "0.6.2"
PACKAGING_VERSION = "25.0"
NUMPY_VERSION = "1.26.4"
CONVERSION_TOOLCHAIN = (
    f"PyTorch {TORCH_VERSION}; safetensors {SAFETENSORS_VERSION}; packaging {PACKAGING_VERSION}; "
    f"NumPy {NUMPY_VERSION}"
)
SOURCE_MODEL_FILE = "pytorch_model.bin"
SOURCE_MODEL_BYTES = 300_827_685
SOURCE_MODEL_SHA256 = "cc1de10b49342ad2f33e06bc4474ddd6eaca278474903c4a8636ce15680d64de"
SOURCE_ASSETS = {
    SOURCE_MODEL_FILE: (SOURCE_MODEL_BYTES, SOURCE_MODEL_SHA256),
    "config.json": (1_416, "df92c371911b45282caac6101fe7e7f5d5e9af7f9a99356067ffe306278ce72b"),
    "source.spm": (778_395, "173e9f493a668fe396d599e28d414a201193094e6ffd7a4678e5aab0f6d3d838"),
    "target.spm": (802_397, "78d0e717c77053f1c4b856d8661d9cb87c64f083a35418c087b9146300e4f585"),
    "vocab.json": (1_339_166, "945c604346ce15ce4aff9001001e7f925e336d942c4087017f191871162cbdc4"),
    "MODEL_CARD.md": (1_209, "695f64a07925db14e3e2f9d0f2074ef8ac86e4d43939451fae5603683dba90a6"),
}
PACK_ASSETS = {
    "model.safetensors": (300_803_144, "174014ba6a746e5d8d1f31470692131c55786dff4daf320cfb882593d674646b"),
    **{name: pin for name, pin in SOURCE_ASSETS.items() if name != "pytorch_model.bin"},
}
PACK_NOTICES = {
    "LICENSE-APACHE-2.0": (11_358, "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30"),
    "THIRD_PARTY_NOTICE.txt": (589, "0c013d0d76a8b41e7ee2341df5ee754fa485a929cf5ec9e41ca538653d99936b"),
}
UPSTREAM_FILES = {"MODEL_CARD.md": "README.md"}
MAX_ASSET_BYTES = 320 * 1024 * 1024
NOTICE = (
    "OPUS-MT Helsinki-NLP opus-mt-en-fr, pinned revision " + MODEL_REV + ".\n"
    "The model card declares Apache-2.0. The weights and SentencePiece vocabularies\n"
    "and model-aligned vocab.json are from that revision; see MODEL_CARD.md and\n"
    "LICENSE-APACHE-2.0. The runtime maps SentencePiece pieces through vocab.json.\n"
    f"The pinned legacy PyTorch state dictionary was converted losslessly to safetensors\n"
    f"with {CONVERSION_TOOLCHAIN}; see manifest conversion provenance.\n"
    "This pack is English-to-French only. Model accuracy is not guaranteed.\n"
)


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def expected_manifest() -> dict:
    return {
        "schema_version": 1,
        "id": PACK_ID,
        "version": "1.0.0",
        "model_repository": MODEL_REPO,
        "model_revision": MODEL_REV,
        "model_license": MODEL_LICENSE,
        "source_model_file": SOURCE_MODEL_FILE,
        "source_model_sha256": SOURCE_MODEL_SHA256,
        "conversion_toolchain": CONVERSION_TOOLCHAIN,
        "tokenizer_repository": TOKENIZER_REPO,
        "tokenizer_revision": MODEL_REV,
        "language_pair": "English (en) to French (fr)",
        "files": [
            {"name": name, "size_bytes": size, "sha256": sha}
            for name, (size, sha) in {**PACK_ASSETS, **PACK_NOTICES}.items()
        ],
    }


def verify_asset(path: Path, size: int, sha256: str) -> None:
    if path.is_symlink() or not path.is_file() or path.stat().st_size != size:
        raise ValueError(f"asset has an unexpected type or length: {path.name}")
    if size > MAX_ASSET_BYTES or digest(path) != sha256:
        raise ValueError(f"asset failed pinned SHA-256 validation: {path.name}")


def verify_source_assets(root: Path) -> None:
    for name, (size, sha) in SOURCE_ASSETS.items():
        verify_asset(root / name, size, sha)
    card = (root / "MODEL_CARD.md").read_text(encoding="utf-8")
    if "license: apache-2.0" not in card or "opus-mt-en-fr" not in card:
        raise ValueError("pinned model card does not document the selected model and license")


def convert_weights(source: Path, destination: Path) -> None:
    """Safely convert the pinned legacy PyTorch pickle to Candle-readable safetensors."""
    try:
        import torch
        import safetensors
        import packaging
        import numpy
        from safetensors.torch import save_file
    except ImportError as error:
        raise RuntimeError(
            f"offline translation conversion requires pinned {CONVERSION_TOOLCHAIN}"
        ) from error
    if (
        torch.__version__ != TORCH_VERSION
        or safetensors.__version__ != SAFETENSORS_VERSION
        or packaging.__version__ != PACKAGING_VERSION
        or numpy.__version__ != NUMPY_VERSION
    ):
        raise RuntimeError(
            f"expected {CONVERSION_TOOLCHAIN}, found PyTorch {torch.__version__}; "
            f"safetensors {safetensors.__version__}; packaging {packaging.__version__}; "
            f"NumPy {numpy.__version__}"
        )
    state = torch.load(source, map_location="cpu", weights_only=True)
    if not isinstance(state, dict) or not state or not all(
        isinstance(name, str) and isinstance(value, torch.Tensor)
        for name, value in state.items()
    ):
        raise ValueError("pinned model did not decode to a plain tensor state dictionary")
    shared = state.get("model.shared.weight")
    if shared is None:
        raise ValueError("pinned Marian state dictionary has no shared embedding tensor")
    # HF stores encoder/decoder embedding aliases in the pickle. Preserve one
    # canonical tied embedding tensor; Candle's Marian implementation uses it
    # to construct both embedding layers.
    for duplicate in ("model.encoder.embed_tokens.weight", "model.decoder.embed_tokens.weight"):
        alias = state.get(duplicate)
        if alias is not None:
            if not torch.equal(alias, shared):
                raise ValueError(f"model's tied embedding alias differs: {duplicate}")
            del state[duplicate]
    tensors = {name: value.detach().cpu().contiguous() for name, value in state.items()}
    if len(tensors) != len(state):
        raise ValueError("model state dictionary contains duplicate tensor names")
    save_file(tensors, str(destination), metadata={"format": "pt"})


def verify_pack_files(root: Path) -> None:
    for name, (size, sha) in {**PACK_ASSETS, **PACK_NOTICES}.items():
        verify_asset(root / name, size, sha)


def _download(url: str, destination: Path, expected_size: int, expected_sha: str) -> None:
    request = urllib.request.Request(url, headers={"User-Agent": "Speech-Studio-model-pack/1"})
    with urllib.request.urlopen(request, timeout=60) as response:
        final = urllib.parse.urlparse(response.geturl())
        host = final.hostname or ""
        approved_host = host in {"huggingface.co", "hf.co"} or host.endswith((".huggingface.co", ".hf.co"))
        if final.scheme != "https" or not approved_host:
            raise ValueError("model download redirected outside Hugging Face HTTPS hosts")
        total, h = 0, hashlib.sha256()
        with destination.open("xb") as output:
            while block := response.read(1024 * 1024):
                total += len(block)
                if total > expected_size or total > MAX_ASSET_BYTES:
                    raise ValueError(f"upstream asset exceeded the pinned size: {destination.name}")
                h.update(block)
                output.write(block)
    if total != expected_size or h.hexdigest() != expected_sha:
        destination.unlink(missing_ok=True)
        raise ValueError(f"download failed pinned size or SHA-256: {destination.name}")


def download_sources(destination: Path) -> None:
    destination.mkdir(parents=True, exist_ok=False)
    try:
        for name, (size, sha) in SOURCE_ASSETS.items():
            source_name = UPSTREAM_FILES.get(name, name)
            url = f"https://huggingface.co/{MODEL_REPO}/resolve/{MODEL_REV}/{source_name}?download=true"
            _download(url, destination / name, size, sha)
        verify_source_assets(destination)
    except Exception:
        shutil.rmtree(destination, ignore_errors=True)
        raise


def create_pack(asset_root: Path, output: Path) -> Path:
    verify_source_assets(asset_root)
    if output.exists():
        raise FileExistsError(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="speech-translation-", dir=output.parent) as temp:
        staging = Path(temp) / "engines" / PACK_ID
        staging.mkdir(parents=True)
        for name in PACK_ASSETS:
            if name != "model.safetensors":
                shutil.copyfile(asset_root / name, staging / name)
        convert_weights(asset_root / SOURCE_MODEL_FILE, staging / "model.safetensors")
        shutil.copyfile(ROOT / "LICENSE-APACHE", staging / "LICENSE-APACHE-2.0")
        (staging / "THIRD_PARTY_NOTICE.txt").write_text(NOTICE, encoding="utf-8", newline="\n")
        verify_pack_files(staging)
        (staging / "manifest.json").write_text(
            json.dumps(expected_manifest(), indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
            newline="\n",
        )
        verify_pack_files(staging)
        # Zip beside the app's `engines/` tree so contents have a stable prefix.
        with zipfile.ZipFile(output, "x", zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
            for path in sorted(Path(temp).rglob("*")):
                if path.is_file():
                    archive.write(path, path.relative_to(temp).as_posix())
        verify_pack(output)
    return output


def expected_entries() -> set[str]:
    base = f"engines/{PACK_ID}/"
    return {base + name for name in (*PACK_ASSETS, *PACK_NOTICES, "manifest.json")}


def verify_pack(pack_path: Path) -> None:
    with zipfile.ZipFile(pack_path) as archive:
        entries = archive.infolist()
        names = [entry.filename for entry in entries]
        if set(names) != expected_entries() or len(names) != len(set(names)):
            raise ValueError("translation pack contains missing, duplicate, or unexpected entries")
        for entry in entries:
            path = PurePosixPath(entry.filename)
            if path.is_absolute() or ".." in path.parts or entry.is_dir() or entry.file_size > MAX_ASSET_BYTES:
                raise ValueError("translation pack contains an unsafe archive entry")
            if entry.external_attr >> 16 & 0o170000 == 0o120000:
                raise ValueError("translation pack must not contain symlinks")
        manifest = json.loads(archive.read(f"engines/{PACK_ID}/manifest.json"))
        if manifest != expected_manifest():
            raise ValueError("translation pack manifest differs from pinned source metadata")
        for name, (size, sha) in {**PACK_ASSETS, **PACK_NOTICES}.items():
            digest = hashlib.sha256()
            total = 0
            with archive.open(f"engines/{PACK_ID}/{name}") as content:
                while block := content.read(1024 * 1024):
                    total += len(block)
                    if total > size:
                        raise ValueError(f"translation pack asset exceeds its pinned size: {name}")
                    digest.update(block)
            if total != size or digest.hexdigest() != sha:
                raise ValueError(f"translation pack asset verification failed: {name}")


def install_pack(pack_path: Path, app_directory: Path) -> Path:
    verify_pack(pack_path)
    destination = app_directory / "engines" / PACK_ID
    if destination.exists() or destination.is_symlink():
        raise FileExistsError(destination)
    engines = app_directory / "engines"
    engines.mkdir(parents=True, exist_ok=True)
    if engines.is_symlink():
        raise ValueError("application engines directory must not be a symlink")
    staging = Path(tempfile.mkdtemp(prefix=".translation-stage-", dir=engines))
    try:
        with zipfile.ZipFile(pack_path) as archive:
            for name in sorted(expected_entries()):
                relative = PurePosixPath(name).relative_to("engines")
                member = archive.getinfo(name)
                target = staging.joinpath(*relative.parts)
                target.parent.mkdir(parents=True, exist_ok=True)
                with archive.open(member) as source, target.open("xb") as output:
                    shutil.copyfileobj(source, output, 1024 * 1024)
        verify_pack_files(staging / PACK_ID)
        (staging / PACK_ID / "manifest.json").write_text(
            json.dumps(expected_manifest(), indent=2) + "\n", encoding="utf-8", newline="\n"
        )
        staging_pack = staging / PACK_ID
        staging_pack.rename(destination)
    except Exception:
        shutil.rmtree(staging, ignore_errors=True)
        raise
    staging.rmdir()
    return destination


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="action", required=True)
    assemble = sub.add_parser("assemble", help="Download pinned assets and create an offline ZIP")
    assemble.add_argument("output", type=Path)
    assemble.add_argument("--cache-dir", type=Path)
    install = sub.add_parser("install", help="Install an already-built verified ZIP beside the app")
    install.add_argument("pack", type=Path)
    install.add_argument("app_directory", type=Path, help="Directory containing the application executable")
    verify = sub.add_parser("verify", help="Verify a pack without installing it")
    verify.add_argument("pack", type=Path)
    args = parser.parse_args()
    if args.action == "assemble":
        if args.cache_dir:
            assets = args.cache_dir
            verify_source_assets(assets)
        else:
            with tempfile.TemporaryDirectory(prefix="speech-translation-assets-") as temp:
                assets = Path(temp) / "assets"
                download_sources(assets)
                create_pack(assets, args.output)
        if args.cache_dir:
            create_pack(assets, args.output)
        print(f"Created verified offline model pack: {args.output}")
    elif args.action == "install":
        print(f"Installed verified model pack at {install_pack(args.pack, args.app_directory)}")
    else:
        verify_pack(args.pack)
        print("Verified pinned English-to-French model pack")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
