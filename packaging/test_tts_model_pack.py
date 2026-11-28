"""Deterministic licensing, provenance, lexicon and archive-safety tests."""
import hashlib
import importlib.util
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

MODULE_SPEC = importlib.util.spec_from_file_location(
    "tts_model_pack", Path(__file__).with_name("tts_model_pack.py")
)
PACKER = importlib.util.module_from_spec(MODULE_SPEC)
MODULE_SPEC.loader.exec_module(PACKER)


class TtsModelPackTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.dictionary = self.root / "cmudict.dict"
        self.dictionary.write_text("hello HH AH0 L OW1\nworld W ER1 L D\n", encoding="ascii")
        self.dictionary_license = self.root / "CMUDICT-LICENSE"
        self.dictionary_license.write_bytes(b"BSD fixture license")
        self.model_card = self.root / "MODEL_CARD.md"
        self.model_card.write_bytes(b"Apache fixture model card")
        lexicon = self.root / "fixture-lexicon.txt"
        PACKER.create_lexicon(self.dictionary, lexicon)
        phones = sorted({p for line in lexicon.read_text().splitlines() for p in line.split("||", 1)[1].split()})
        PACKER.CMUDICT_SHA256 = PACKER._sha256(self.dictionary)
        PACKER.CMUDICT_LICENSE_SHA256 = PACKER._sha256(self.dictionary_license)
        self.archive = self.root / "kitten.tar.bz2"
        self.assets = {
            "model.int8.onnx": b"model fixture",
            "voices.bin": b"voices fixture",
            "tokens.txt": "\n".join(f"{phone} {index}" for index, phone in enumerate(phones)).encode(),
            "LICENSE": b"Apache fixture license",
        }
        with tarfile.open(self.archive, "w:bz2") as output:
            for name, data in self.assets.items():
                info = tarfile.TarInfo(f"{PACKER.MODEL_PREFIX}{name}")
                info.size = len(data)
                output.addfile(info, io.BytesIO(data))
        lexicon = self.root / "manifest-lexicon.txt"
        PACKER.create_lexicon(self.dictionary, lexicon)
        self.manifest = {
            "id": "tts-fixture",
            "source_archive_size_bytes": self.archive.stat().st_size,
            "source_archive_sha256": PACKER._sha256(self.archive),
            "files": [
                {"name": name, "size_bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
                for name, data in self.assets.items()
            ] + [
                {"name": "lexicon.txt", "size_bytes": lexicon.stat().st_size, "sha256": PACKER._sha256(lexicon)},
                {"name": "CMUDICT-LICENSE", "size_bytes": self.dictionary_license.stat().st_size, "sha256": PACKER._sha256(self.dictionary_license)},
                {"name": "MODEL_CARD.md", "size_bytes": self.model_card.stat().st_size, "sha256": PACKER._sha256(self.model_card)},
            ],
        }

    def tearDown(self):
        self.temp.cleanup()

    def test_arpabet_conversion_targets_kitten_ipa_tokens(self):
        lexicon = self.root / "lexicon-test.txt"
        PACKER.create_lexicon(self.dictionary, lexicon)
        self.assertEqual(lexicon.read_text().splitlines(), [
            "hello || h ə l ˈ o ʊ", "world || w ˈ ɝ l d",
        ])
        self.assertIn("ˈ", set(lexicon.read_text().replace("||", "").split()))

    def test_extracts_only_pinned_model_and_license_assets(self):
        destination = self.root / "unpacked"
        PACKER.extract_assets(
            self.archive, self.dictionary, self.dictionary_license, destination,
            self.manifest, self.model_card,
        )
        expected = {item["name"] for item in self.manifest["files"]} | {"manifest.json"}
        self.assertEqual({path.name for path in destination.iterdir()}, expected)
        self.assertEqual((destination / "CMUDICT-LICENSE").read_bytes(), self.dictionary_license.read_bytes())
        self.assertEqual(json.loads((destination / "manifest.json").read_text()), self.manifest)

    def test_creates_verified_installable_directory_layout(self):
        output = PACKER.create_pack(
            self.archive, self.dictionary, self.dictionary_license,
            self.root / "new-output", self.manifest, self.model_card,
        )
        with PACKER.zipfile.ZipFile(output) as bundle:
            self.assertIn("engines/tts-fixture/CMUDICT-LICENSE", bundle.namelist())
            self.assertIn("engines/tts-fixture/lexicon.txt", bundle.namelist())

    def test_rejects_archive_hash_mismatch(self):
        changed = dict(self.manifest, source_archive_sha256="0" * 64)
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            PACKER.verify_sources(self.archive, self.dictionary, self.dictionary_license, self.model_card, changed)

    def test_rejects_bad_dictionary_and_model_card_hashes(self):
        self.dictionary.write_text("changed\n", encoding="ascii")
        with self.assertRaisesRegex(ValueError, "CMUdict source"):
            PACKER.verify_sources(self.archive, self.dictionary, self.dictionary_license, self.model_card, self.manifest)
        self.dictionary.write_text("hello HH AH0 L OW1\nworld W ER1 L D\n", encoding="ascii")
        self.model_card.write_bytes(b"altered")
        with self.assertRaisesRegex(ValueError, "model card"):
            PACKER.verify_sources(self.archive, self.dictionary, self.dictionary_license, self.model_card, self.manifest)

    def test_rejects_unsupported_phone_instead_of_emitting_bad_lexicon(self):
        self.dictionary.write_text("example XX1\n", encoding="ascii")
        with self.assertRaisesRegex(ValueError, "Unsupported CMUdict phoneme"):
            PACKER.create_lexicon(self.dictionary, self.root / "bad-lexicon.txt")


if __name__ == "__main__":
    unittest.main()
