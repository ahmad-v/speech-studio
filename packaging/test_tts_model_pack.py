"""Small deterministic tests for the offline TTS packer's safety rules."""
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
        self.assets = {
            "en_US-ljspeech-medium.onnx": b"model fixture",
            "en_US-ljspeech-medium.onnx.json": b"config fixture",
            "tokens.txt": b"tokens fixture",
            "MODEL_CARD": b"public-domain dataset card fixture",
        }
        self.manifest = {
            "id": "voice-fixture",
            "source_archive_size_bytes": 0,
            "source_archive_sha256": "",
            "files": [
                {
                    "name": name,
                    "size_bytes": len(data),
                    "sha256": hashlib.sha256(data).hexdigest(),
                }
                for name, data in self.assets.items()
            ],
        }
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.archive = self.root / "voice.tar.bz2"
        with tarfile.open(self.archive, "w:bz2") as output:
            for name, data in self.assets.items():
                info = tarfile.TarInfo(f"vits-piper-en_US-ljspeech-medium-int8/{name}")
                info.size = len(data)
                output.addfile(info, io.BytesIO(data))
        self.manifest["source_archive_size_bytes"] = self.archive.stat().st_size
        self.manifest["source_archive_sha256"] = PACKER._sha256(self.archive)

    def tearDown(self):
        self.temp.cleanup()

    def test_extracts_only_hash_pinned_assets(self):
        destination = self.root / "unpacked"
        PACKER.extract_assets(self.archive, destination, self.manifest)
        self.assertEqual(
            {path.name for path in destination.iterdir()},
            set(self.assets) | {"manifest.json"},
        )
        self.assertEqual(
            json.loads((destination / "manifest.json").read_text()), self.manifest
        )

    def test_creates_verified_installable_directory_layout(self):
        output_dir = self.root / "new-output"
        archive = PACKER.create_pack(self.archive, output_dir, self.manifest)
        self.assertEqual(archive.name, "voice-fixture.zip")
        with PACKER.zipfile.ZipFile(archive) as bundle:
            self.assertIn(
                "engines/voice-fixture/MODEL_CARD", bundle.namelist()
            )

    def test_rejects_archive_hash_mismatch(self):
        changed = dict(self.manifest, source_archive_sha256="0" * 64)
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            PACKER.verify_archive(self.archive, changed)

    def test_rejects_unexpected_asset_digest_and_cleans_output(self):
        changed = json.loads(json.dumps(self.manifest))
        changed["files"][0]["sha256"] = "0" * 64
        destination = self.root / "bad-pack"
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            PACKER.extract_assets(self.archive, destination, changed)
        self.assertFalse(destination.exists())

    def test_rejects_archive_path_traversal(self):
        with tarfile.open(self.archive, "w:bz2") as output:
            info = tarfile.TarInfo(
                "vits-piper-en_US-ljspeech-medium-int8/../escape.txt"
            )
            info.size = len(b"escape")
            output.addfile(info, io.BytesIO(b"escape"))
        self.manifest["source_archive_size_bytes"] = self.archive.stat().st_size
        self.manifest["source_archive_sha256"] = PACKER._sha256(self.archive)
        destination = self.root / "traversal"
        with self.assertRaisesRegex(ValueError, "omitted required assets"):
            PACKER.extract_assets(self.archive, destination, self.manifest)
        self.assertFalse((self.root / "escape.txt").exists())
        self.assertFalse(destination.exists())


if __name__ == "__main__":
    unittest.main()
