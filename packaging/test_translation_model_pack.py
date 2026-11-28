import hashlib
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path
import zipfile

SPEC = importlib.util.spec_from_file_location(
    "translation_model_pack", Path(__file__).with_name("translation_model_pack.py")
)
PACK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACK)
PINNED_MANIFEST = PACK.expected_manifest()


class TranslationPackTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.assets = {name: data for name, data in {
            "pytorch_model.bin": b"fixture-model",
            "source.spm": b"fixture-source-tokenizer",
            "target.spm": b"fixture-target-tokenizer",
            "MODEL_CARD.md": b"license: apache-2.0\nopus-mt-en-fr\n",
        }.items()}
        PACK.ASSETS = {name: (len(data), hashlib.sha256(data).hexdigest()) for name, data in self.assets.items()}
        for name, data in self.assets.items():
            (self.root / name).write_bytes(data)

    def test_verified_pack_installs_exclusively_and_preserves_assets(self):
        pack = PACK.create_pack(self.root, self.root / "translation.zip")
        PACK.verify_pack(pack)
        app = self.root / "app"
        installed = PACK.install_pack(pack, app)
        for name, content in self.assets.items():
            self.assertEqual((installed / name).read_bytes(), content)
        with self.assertRaises(FileExistsError):
            PACK.install_pack(pack, app)

    def test_checked_in_manifest_matches_upstream_pins(self):
        manifest_path = Path(__file__).parent / "translation-model" / "manifest.json"
        self.assertEqual(json.loads(manifest_path.read_text(encoding="utf-8")), PINNED_MANIFEST)

    def test_pack_rejects_changed_input_and_unsafe_archive_entries(self):
        (self.root / "source.spm").write_bytes(b"modified")
        with self.assertRaises(ValueError):
            PACK.verify_assets(self.root)

        pack = self.root / "hostile.zip"
        with zipfile.ZipFile(pack, "w") as archive:
            archive.writestr("../outside", b"no")
        with self.assertRaises(ValueError):
            PACK.verify_pack(pack)


if __name__ == "__main__":
    unittest.main()
