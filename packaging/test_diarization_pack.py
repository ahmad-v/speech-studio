"""Hermetic packaging tests for the offline speaker diarization pack."""

from __future__ import annotations

import copy
import hashlib
import importlib.util
import io
import json
import tarfile
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).with_name("diarization_pack.py")
SPEC = importlib.util.spec_from_file_location("diarization_pack", MODULE_PATH)
PACKER = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(PACKER)


def _item(name: str, data: bytes) -> dict:
    return {"name": name, "size_bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


class DiarizationPackTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.embedding = self.root / "embedding.onnx"
        self.embedding.write_bytes(b"speaker embedding model fixture")
        self.archive = self.root / "segmentation.tar.bz2"
        self.segmentation = b"segmentation model fixture"
        self.mit = b"MIT model notice fixture"
        with tarfile.open(self.archive, "w:bz2") as output:
            for name, data in (("model.int8.onnx", self.segmentation), ("LICENSE", self.mit)):
                member = tarfile.TarInfo(PACKER.SEGMENTATION_PREFIX + name)
                member.size = len(data)
                output.addfile(member, io.BytesIO(data))
        PACKER.SEGMENTATION_ARCHIVE_BYTES = self.archive.stat().st_size
        PACKER.SEGMENTATION_ARCHIVE_SHA256 = PACKER.sha256(self.archive)
        PACKER.EMBEDDING_SOURCE_BYTES = self.embedding.stat().st_size
        PACKER.EMBEDDING_SOURCE_SHA256 = PACKER.sha256(self.embedding)
        self.manifest = copy.deepcopy(PACKER.MANIFEST)
        files = [
            _item("segmentation.int8.onnx", self.segmentation),
            _item("speaker-embedding.onnx", self.embedding.read_bytes()),
            _item("LICENSE-pyannote-MIT", self.mit),
        ]
        for name in ("LICENSE-3dspeaker-Apache-2.0", "MODEL-CARD.md"):
            files.append(_item(name, (PACKER.MODEL_DIR / name).read_bytes()))
        self.manifest["files"] = files
        self.manifest["id"] = "diarization-fixture"

    def tearDown(self):
        self.temp.cleanup()

    def test_extracts_only_verified_assets_and_includes_both_model_licenses(self):
        destination = self.root / "pack"
        PACKER.extract_pack(self.archive, self.embedding, destination, self.manifest)
        names = {path.name for path in destination.iterdir()}
        self.assertEqual(names, {item["name"] for item in self.manifest["files"]} | {"manifest.json"})
        self.assertEqual((destination / "segmentation.int8.onnx").read_bytes(), self.segmentation)
        self.assertEqual((destination / "speaker-embedding.onnx").read_bytes(), self.embedding.read_bytes())
        self.assertEqual(json.loads((destination / "manifest.json").read_text()), self.manifest)

    def test_rejects_changed_source_archive_before_unpacking(self):
        PACKER.SEGMENTATION_ARCHIVE_SHA256 = "0" * 64
        with self.assertRaisesRegex(ValueError, "SHA-256"):
            PACKER.extract_pack(self.archive, self.embedding, self.root / "bad", self.manifest)

    def test_rejects_tampered_embedding_weights(self):
        PACKER.EMBEDDING_SOURCE_SHA256 = "0" * 64
        with self.assertRaisesRegex(ValueError, "embedding model SHA-256"):
            PACKER.extract_pack(self.archive, self.embedding, self.root / "bad", self.manifest)

    def test_rejects_unsafe_required_archive_member(self):
        with tarfile.open(self.archive, "w:bz2") as output:
            member = tarfile.TarInfo(PACKER.SEGMENTATION_PREFIX + "model.int8.onnx")
            member.type = tarfile.SYMTYPE
            member.linkname = "/etc/passwd"
            output.addfile(member)
        PACKER.SEGMENTATION_ARCHIVE_BYTES = self.archive.stat().st_size
        PACKER.SEGMENTATION_ARCHIVE_SHA256 = PACKER.sha256(self.archive)
        with self.assertRaisesRegex(ValueError, "unsafe or duplicate"):
            PACKER.extract_pack(self.archive, self.embedding, self.root / "bad", self.manifest)


if __name__ == "__main__":
    unittest.main()
