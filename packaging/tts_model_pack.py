"""Assemble a hash-pinned, offline Kitten TTS pack with BSD CMUdict lexicon."""
import argparse
import hashlib
import json
import tarfile
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "packaging" / "tts-model" / "manifest.json"
MANIFEST = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
MODEL_PREFIX = "kitten-nano-en-v0_8-int8/"
MAX_ARCHIVE_BYTES = 40 * 1024 * 1024
MAX_EXTRACTED_BYTES = 40 * 1024 * 1024
CMUDICT_COMMIT = "74790861f652b15e4ac49015a90074ad62a27690"
CMUDICT_SHA256 = "81917843c7f44ce2b094ac63873c2c7a4cf802040792c455ba3ca406891c3d22"
CMUDICT_LICENSE_SHA256 = "bd4ce8e44170a5f9f481310ca85c51de3c4f851a65e679b40e603b143bd3542a"

# CMUdict ARPABET to the single-phone/IPA token vocabulary shipped with Kitten.
# Diphthongs are deliberately split into model tokens; stress is emitted as a
# separate token so it matches Kitten's tokens.txt rather than an ARPABET tag.
VOWELS = {
    "AA": ("ɑ", "ɑ"), "AE": ("æ", "æ"), "AH": ("ə", "ʌ"),
    "AO": ("ɔ", "ɔ"), "EH": ("ɛ", "ɛ"), "ER": ("ɚ", "ɝ"),
    "IH": ("ɪ", "ɪ"), "IY": ("i", "i"), "UH": ("ʊ", "ʊ"),
    "UW": ("u", "u"),
}
CONSONANTS = {
    "B": ("b",), "CH": ("ʧ",), "D": ("d",), "DH": ("ð",),
    "F": ("f",), "G": ("ɡ",), "HH": ("h",), "JH": ("ʤ",),
    "K": ("k",), "L": ("l",), "M": ("m",), "N": ("n",),
    "NG": ("ŋ",), "P": ("p",), "R": ("ɹ",), "S": ("s",),
    "SH": ("ʃ",), "T": ("t",), "TH": ("θ",), "V": ("v",),
    "W": ("w",), "Y": ("j",), "Z": ("z",), "ZH": ("ʒ",),
    "DX": ("ɾ",), "EL": ("ə", "l"), "EM": ("ə", "m"),
    "EN": ("ə", "n"), "AX": ("ə",), "AXR": ("ɚ",),
    "IX": ("ɪ",), "UX": ("u",),
}
DIPHTHONGS = {
    "AW": ("a", "ʊ"), "AY": ("a", "ɪ"), "EY": ("e", "ɪ"),
    "OW": ("o", "ʊ"), "OY": ("ɔ", "ɪ"),
}


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_sources(archive: Path, dictionary: Path, dictionary_license: Path, model_card: Path, manifest=MANIFEST) -> None:
    if not archive.is_file() or archive.stat().st_size != manifest["source_archive_size_bytes"]:
        raise ValueError("Kitten TTS archive size does not match the pinned manifest")
    if _sha256(archive) != manifest["source_archive_sha256"]:
        raise ValueError("Kitten TTS archive SHA-256 does not match the pinned manifest")
    if not dictionary.is_file() or dictionary.stat().st_size > MAX_ARCHIVE_BYTES:
        raise ValueError("CMUdict source is missing or exceeds its size limit")
    if _sha256(dictionary) != CMUDICT_SHA256:
        raise ValueError("CMUdict source does not match the pinned BSD-licensed revision")
    if not dictionary_license.is_file() or _sha256(dictionary_license) != CMUDICT_LICENSE_SHA256:
        raise ValueError("CMUdict BSD license notice does not match the pinned revision")
    card = next(item for item in manifest["files"] if item["name"] == "MODEL_CARD.md")
    if model_card.stat().st_size != card["size_bytes"] or _sha256(model_card) != card["sha256"]:
        raise ValueError("Kitten model card does not match the pinned Apache source revision")


def _phonemes(arpabet: str) -> list[str]:
    result = []
    for item in arpabet.split():
        phone = item.rstrip("012")
        stress = item[len(phone):]
        if phone in VOWELS:
            if stress == "1":
                result.append("ˈ")
            elif stress == "2":
                result.append("ˌ")
            result.append(VOWELS[phone][0 if stress == "0" else 1])
        elif phone in DIPHTHONGS:
            if stress == "1":
                result.append("ˈ")
            elif stress == "2":
                result.append("ˌ")
            result.extend(DIPHTHONGS[phone])
        elif phone in CONSONANTS:
            result.extend(CONSONANTS[phone])
        else:
            raise ValueError(f"Unsupported CMUdict phoneme: {phone}")
    return result


def create_lexicon(dictionary: Path, destination: Path) -> None:
    """Create a deterministic sherpa lexicon using the pinned BSD CMUdict."""
    seen = set()
    with dictionary.open("r", encoding="ascii") as source, destination.open("x", encoding="utf-8", newline="\n") as output:
        for line in source:
            line = line.strip()
            if not line or line.startswith(";;; "):
                continue
            fields = line.split(" #", 1)[0].split()
            if len(fields) < 2:
                continue
            word = fields[0]
            # Use the first CMUdict pronunciation and ignore alternate-number
            # suffixes; the first is stable and avoids duplicate lexicon rows.
            word = __import__("re").sub(r"\(\d+\)$", "", word).lower()
            if word in seen:
                continue
            phones = _phonemes(" ".join(fields[1:]))
            if not phones:
                continue
            output.write(f"{word} || {' '.join(phones)}\n")
            seen.add(word)
    if not seen:
        destination.unlink(missing_ok=True)
        raise ValueError("CMUdict produced an empty TTS lexicon")


def extract_assets(archive: Path, dictionary: Path, dictionary_license: Path, destination: Path, manifest=MANIFEST, model_card: Path | None = None) -> None:
    """Extract an explicit allowlist and generate the licensed lexicon."""
    model_card = model_card or ROOT / "packaging" / "tts-model" / "MODEL_CARD.md"
    verify_sources(archive, dictionary, dictionary_license, model_card, manifest)
    all_expected = {item["name"]: item for item in manifest["files"]}
    expected = {name: item for name, item in all_expected.items() if name not in {"lexicon.txt", "CMUDICT-LICENSE", "MODEL_CARD.md"}}
    destination.mkdir(parents=True, exist_ok=False)
    try:
        total = 0
        seen = set()
        with tarfile.open(archive, mode="r:bz2") as source:
            for member in source.getmembers():
                if not member.name.startswith(MODEL_PREFIX):
                    continue
                name = member.name[len(MODEL_PREFIX):]
                if name not in expected:
                    continue
                if name in seen or not member.isfile() or member.size > MAX_EXTRACTED_BYTES:
                    raise ValueError("TTS archive contains an unsafe or duplicate asset")
                stream = source.extractfile(member)
                if stream is None:
                    raise ValueError("Could not read a TTS archive asset")
                target = destination / name
                with stream, target.open("xb") as output:
                    digest = hashlib.sha256()
                    size = 0
                    while chunk := stream.read(1024 * 1024):
                        size += len(chunk)
                        total += len(chunk)
                        if size > expected[name]["size_bytes"] or total > MAX_EXTRACTED_BYTES:
                            raise ValueError("TTS archive exceeds its declared asset bounds")
                        digest.update(chunk)
                        output.write(chunk)
                item = expected[name]
                if size != item["size_bytes"] or digest.hexdigest() != item["sha256"]:
                    raise ValueError(f"TTS asset failed pinned verification: {name}")
                seen.add(name)
        if seen != set(expected):
            raise ValueError("TTS archive omitted a required pinned asset")
        create_lexicon(dictionary, destination / "lexicon.txt")
        tokens = {
            line.rsplit(" ", 1)[0]
            for line in (destination / "tokens.txt").read_text(encoding="utf-8").splitlines()
        }
        for line in (destination / "lexicon.txt").read_text(encoding="utf-8").splitlines():
            _, phones = line.split("||", 1)
            unknown = set(phones.split()) - tokens
            if unknown:
                raise ValueError(f"Generated lexicon uses phones absent from Kitten tokens: {sorted(unknown)}")
        for name in ("lexicon.txt", "CMUDICT-LICENSE", "MODEL_CARD.md"):
            item = all_expected[name]
            path = destination / name
            if name == "CMUDICT-LICENSE":
                path.write_bytes(dictionary_license.read_bytes())
            elif name == "MODEL_CARD.md":
                path.write_bytes(model_card.read_bytes())
            if path.stat().st_size != item["size_bytes"] or _sha256(path) != item["sha256"]:
                raise ValueError(f"Generated TTS asset does not match the pinned digest: {name}")
        (destination / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    except Exception:
        import shutil
        shutil.rmtree(destination, ignore_errors=True)
        raise


def create_pack(archive: Path, dictionary: Path, dictionary_license: Path, output_dir: Path, manifest=MANIFEST, model_card: Path | None = None) -> Path:
    output_dir.mkdir(parents=True, exist_ok=False)
    pack_root = output_dir / "engines" / manifest["id"]
    pack_root.parent.mkdir(parents=True, exist_ok=True)
    try:
        extract_assets(archive, dictionary, dictionary_license, pack_root, manifest, model_card)
        output = output_dir / f"{manifest['id']}.zip"
        with zipfile.ZipFile(output, "x", zipfile.ZIP_DEFLATED, compresslevel=6) as bundle:
            for path in sorted(output_dir.rglob("*")):
                if path.is_file() and path != output:
                    bundle.write(path, path.relative_to(output_dir).as_posix())
        with zipfile.ZipFile(output) as bundle:
            names = set(bundle.namelist())
            expected = {f"engines/{manifest['id']}/{item['name']}" for item in manifest["files"]}
            expected.add(f"engines/{manifest['id']}/manifest.json")
            if names != expected:
                raise ValueError("TTS pack contains unexpected entries")
        return output
    except Exception:
        import shutil
        shutil.rmtree(output_dir, ignore_errors=True)
        raise


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path, help="pinned sherpa Kitten Nano 0.8 archive")
    parser.add_argument("cmudict", type=Path, help="cmudict.dict from the pinned CMUdict commit")
    parser.add_argument("cmudict_license", type=Path, help="LICENSE from the same pinned CMUdict commit")
    parser.add_argument("output_dir", type=Path, help="new output directory")
    args = parser.parse_args()
    try:
        output = create_pack(args.archive, args.cmudict, args.cmudict_license, args.output_dir)
    except (OSError, ValueError, tarfile.TarError) as error:
        parser.error(str(error))
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
