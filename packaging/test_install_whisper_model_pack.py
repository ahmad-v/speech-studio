import hashlib
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
from install_whisper_model_pack import install_pack
import extract_whisper_model as extractor
from whisper_model_pack import canonical_text_bytes


class OfflineModelInstallTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.app = self.root / "Speech Studio"
        self.app.mkdir()
        (self.app / "speech-studio").write_bytes(b"test application")
        self.payload = b"small offline model fixture"
        self.manifest = {
            "schema_version": 1,
            "id": "whisper-test",
            "file": "model.bin",
            "bytes": len(self.payload),
            "sha256": hashlib.sha256(self.payload).hexdigest(),
        }
        self.manifest_bytes = json.dumps(self.manifest, sort_keys=True).encode() + b"\n"
        self.license_bytes = b"MIT test license\n"
        self.pack = self.root / "model.zip"
        self.write_pack()

    def tearDown(self):
        self.temporary.cleanup()

    def write_pack(self, payload=None, extra=None):
        payload = self.payload if payload is None else payload
        with zipfile.ZipFile(self.pack, "w", compression=zipfile.ZIP_STORED) as archive:
            prefix = "engines/whisper-test/"
            archive.writestr(prefix + "manifest.json", self.manifest_bytes)
            archive.writestr(prefix + "LICENSE-MIT", self.license_bytes)
            archive.writestr(prefix + "model.bin", payload)
            if extra:
                archive.writestr(extra[0], extra[1])

    def test_installs_verified_model_and_preserves_existing_files(self):
        manifest_path = self.root / "pinned.json"
        manifest_path.write_bytes(self.manifest_bytes.replace(b"\n", b"\r\n"))
        license_path = self.root / "pinned-license"
        license_path.write_bytes(self.license_bytes.replace(b"\n", b"\r\n"))
        installed = install_pack(
            self.pack,
            self.app,
            self.manifest,
            canonical_text_bytes(manifest_path),
            canonical_text_bytes(license_path),
        )
        self.assertEqual(installed, self.app / "engines" / "whisper-test")
        self.assertEqual((installed / "model.bin").read_bytes(), self.payload)
        self.assertEqual((installed / "manifest.json").read_bytes(), self.manifest_bytes)
        self.assertEqual((installed / "LICENSE-MIT").read_bytes(), self.license_bytes)
        with self.assertRaises(FileExistsError):
            install_pack(
                self.pack,
                self.app,
                self.manifest,
                self.manifest_bytes,
                self.license_bytes,
            )
        self.assertEqual((installed / "model.bin").read_bytes(), self.payload)

    def test_rejects_bad_hash_and_cleans_partial_install(self):
        self.write_pack(b"x" * len(self.payload))
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            install_pack(
                self.pack,
                self.app,
                self.manifest,
                self.manifest_bytes,
                self.license_bytes,
            )
        self.assertFalse((self.app / "engines" / "whisper-test").exists())

    def test_rejects_extra_traversal_entries(self):
        self.write_pack(extra=("../outside.txt", b"bad"))
        with self.assertRaisesRegex(ValueError, "exactly the pinned"):
            install_pack(
                self.pack,
                self.app,
                self.manifest,
                self.manifest_bytes,
                self.license_bytes,
            )
        self.assertFalse((self.root / "outside.txt").exists())

    def test_rejects_non_application_directory(self):
        empty = self.root / "empty"
        empty.mkdir()
        with self.assertRaisesRegex(ValueError, "does not contain Speech Studio"):
            install_pack(
                self.pack,
                empty,
                self.manifest,
                self.manifest_bytes,
                self.license_bytes,
            )

    def test_distribution_extractor_verifies_and_never_overwrites(self):
        manifest_path = self.root / "pinned.json"
        manifest_path.write_bytes(self.manifest_bytes.replace(b"\n", b"\r\n"))
        license_path = self.root / "LICENSE-MIT"
        license_path.write_bytes(self.license_bytes.replace(b"\n", b"\r\n"))
        destination = self.root / "bundle-model.bin"
        with mock.patch.object(
            extractor, "load_manifest", return_value=(self.manifest, manifest_path)
        ), mock.patch.object(extractor, "SOURCE", self.root):
            extracted = extractor.extract_model(self.pack, destination, "whisper-test")
            self.assertEqual(extracted.read_bytes(), self.payload)
            with self.assertRaises(FileExistsError):
                extractor.extract_model(self.pack, destination, "whisper-test")


if __name__ == "__main__":
    unittest.main()
