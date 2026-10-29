import tempfile
import unittest
from pathlib import Path

from shirolyzer.utils import paths
from shirolyzer.utils import xkb

CFGE = ";aiojvlnrwhfpeygtcmsbxquzkd,./"


class SymbolsTests(unittest.TestCase):
    def test_cfge_matches_shipped_body(self):
        generated = xkb.symbols(CFGE, "cfgE")
        body = "\n".join(line for line in generated.splitlines()
                         if not line.startswith("//"))
        shipped = (paths.XKB_DIR / "mine").read_text(encoding="utf-8")
        shipped_body = "\n".join(line for line in shipped.splitlines()
                                 if not line.startswith("//"))
        self.assertEqual(body, shipped_body)

    def test_comments_include_name_and_grid(self):
        text = xkb.symbols(CFGE, "cfgE")
        self.assertIn("cfgE layout", text)
        self.assertIn("// top:  ; a i o j  v l n r w", text)
        self.assertIn("// home: h f p e y  g t c m s", text)
        self.assertIn("// bot:  b x q u z  k d , . /", text)

    def test_punctuation_and_iso_keys(self):
        text = xkb.symbols(CFGE, "cfgE")
        self.assertIn("key <AD01> { [ semicolon,   colon", text)
        self.assertIn("key <AB08> { [ comma,       less", text)
        self.assertIn("key <AB09> { [ period,      greater", text)
        self.assertIn("key <AB10> { [ slash,       question", text)
        self.assertIn("key <AC11> { [ apostrophe,  quotedbl", text)
        self.assertIn("key <LSGT> { [ apostrophe,  quotedbl", text)

    def test_rejects_bad_layouts(self):
        with self.assertRaisesRegex(ValueError, "30 characters"):
            xkb.symbols("abc", "bad")
        with self.assertRaisesRegex(ValueError, "duplicate"):
            xkb.symbols(CFGE[:-1] + "a", "bad")
        with self.assertRaisesRegex(ValueError, "unsupported"):
            xkb.symbols(CFGE[:-1] + "!", "bad")

    def test_safe_name_is_not_mine(self):
        name = xkb.safe_name(CFGE)
        self.assertNotEqual(name, "mine")
        self.assertTrue(name.startswith("shiro_"))
        self.assertEqual(name, xkb.safe_name(CFGE))

    def test_write_named_uses_xkb_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = xkb.write_named(CFGE, "cfgE", directory=tmp)
            self.assertEqual(Path(path).name, "cfgE")
            self.assertIn("cfgE layout", Path(path).read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
