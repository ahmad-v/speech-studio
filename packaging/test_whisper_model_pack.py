import hashlib
import io
import json
import socket
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import whisper_model_pack as packer
import create_whisper_model_packs as downloader


class WhisperModelPackTests(unittest.TestCase):
    def test_multilingual_manifest_pins_source_weights_and_license(self):
        manifest, _ = packer.load_manifest("whisper-tiny-multilingual")
        license_bytes = (packer.SOURCE / "LICENSE-MIT").read_bytes()
        self.assertEqual(manifest["bytes"], 77_691_713)
        self.assertEqual(
            manifest["sha256"],
            "be07e048e1e599ad46341c8d2a135645097a538221678b7acdd1b1919c6e1b21",
        )
        self.assertTrue(manifest["source"].endswith("/ggml-tiny.bin"))
        self.assertEqual(
            manifest["source_revision"],
            "5359861c739e955e79d9a303bcbc70fb988958b1",
        )
        self.assertEqual(manifest["license"], "MIT")
        self.assertEqual(
            hashlib.sha256(license_bytes).hexdigest(), manifest["license_sha256"]
        )
        self.assertEqual(manifest["language_mode"], "auto")
        self.assertEqual(manifest["model_parameters"], 39_000_000)
        self.assertEqual(manifest["upstream_vram_mib_estimate"], 1024)
        self.assertEqual(manifest["cpu_threads"], 2)
        self.assertFalse(manifest["gpu_required"])
        self.assertIsNone(manifest["runtime_ram_bytes"])

    def test_copy_verified_accepts_only_the_exact_declared_payload(self):
        payload = b"small synthetic model payload"
        manifest = {"bytes": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "source.bin"
            source.write_bytes(payload)
            with (Path(directory) / "valid.bin").open("wb") as output:
                packer.copy_verified(source, output, manifest)
            self.assertEqual((Path(directory) / "valid.bin").read_bytes(), payload)

            source.write_bytes(payload + b"extra")
            with (Path(directory) / "oversized.bin").open("wb") as output:
                with self.assertRaisesRegex(ValueError, "grew beyond"):
                    packer.copy_verified(source, output, manifest)

            source.write_bytes(b"different payload of same length")
            wrong_manifest = {"bytes": len(source.read_bytes()), "sha256": "0" * 64}
            with (Path(directory) / "wrong-hash.bin").open("wb") as output:
                with self.assertRaisesRegex(ValueError, "SHA-256"):
                    packer.copy_verified(source, output, wrong_manifest)

    def test_packager_accepts_model_variants_without_network_configuration(self):
        english, _ = packer.load_manifest("whisper-tiny-en")
        multilingual, _ = packer.load_manifest("whisper-tiny-multilingual")
        self.assertEqual(english["id"], "whisper-tiny-en")
        self.assertEqual(multilingual["id"], "whisper-tiny-multilingual")
        self.assertEqual(multilingual["language_mode"], "auto")
        with self.assertRaisesRegex(ValueError, "unsupported model ID"):
            packer.load_manifest("unknown-model")

    def test_build_pack_works_offline_from_a_local_model_file(self):
        payload = b"pinned test model"
        manifest = {
            "schema_version": 1,
            "id": "test-model",
            "file": "model.bin",
            "bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
        }
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest_path = root / "manifest.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            model_path = root / "model.bin"
            model_path.write_bytes(payload)
            archive_path = root / "offline-pack.zip"
            packer.MANIFESTS["test-model"] = manifest_path
            try:
                with mock.patch.object(
                    socket, "socket", side_effect=AssertionError("network access")
                ):
                    packer.build_pack(model_path, archive_path, "test-model")
            finally:
                del packer.MANIFESTS["test-model"]

            with zipfile.ZipFile(archive_path) as archive:
                self.assertEqual(archive.read("engines/test-model/model.bin"), payload)
                self.assertEqual(
                    json.loads(archive.read("engines/test-model/manifest.json")),
                    manifest,
                )
                self.assertIn("engines/test-model/LICENSE-MIT", archive.namelist())

    def test_download_verifies_pinned_https_source_before_promoting_file(self):
        payload = b"verified model bytes"
        manifest = {
            "source": "https://huggingface.co/example/model/resolve/abc123/model.bin",
            "source_revision": "abc123",
            "bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
        }
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "model.bin"
            with mock.patch.object(
                downloader, "load_manifest", return_value=(manifest, Path("manifest.json"))
            ):
                result = downloader.download_verified(
                    "test-model", destination, opener=lambda request, timeout: io.BytesIO(payload)
                )
            self.assertEqual(result.read_bytes(), payload)
            self.assertFalse(destination.with_name("model.bin.partial").exists())

    def test_download_rejects_unpinned_source_and_corrupt_payload(self):
        payload = b"not the model"
        manifest = {
            "source": "https://example.com/model.bin",
            "source_revision": "abc123",
            "bytes": len(payload),
            "sha256": hashlib.sha256(payload).hexdigest(),
        }
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "model.bin"
            with mock.patch.object(
                downloader, "load_manifest", return_value=(manifest, Path("manifest.json"))
            ):
                with self.assertRaisesRegex(ValueError, "pinned HTTPS"):
                    downloader.download_verified("test-model", destination)
                self.assertFalse(destination.exists())

                manifest["source"] = (
                    "https://huggingface.co/example/model/resolve/abc123/model.bin"
                )
                manifest["sha256"] = "0" * 64
                with self.assertRaisesRegex(ValueError, "size and SHA-256"):
                    downloader.download_verified(
                        "test-model",
                        destination,
                        opener=lambda request, timeout: io.BytesIO(payload),
                    )
            self.assertFalse(destination.exists())
            self.assertFalse(destination.with_name("model.bin.partial").exists())


if __name__ == "__main__":
    unittest.main()
