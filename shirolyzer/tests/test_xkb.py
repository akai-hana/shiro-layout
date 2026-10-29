import tempfile
import unittest
from pathlib import Path

from shirolyzer.tests.fixtures import CURRENT_LAYOUT
from shirolyzer.utils import paths
from shirolyzer.utils import xkb

LAYOUT = CURRENT_LAYOUT


class SymbolsTests(unittest.TestCase):
    def test_generated_body_matches_shipped(self):
        generated = xkb.symbols(LAYOUT, "mine")
        body = "\n".join(line for line in generated.splitlines()
                         if not line.startswith("//"))
        shipped = (paths.XKB_DIR / "mine").read_text(encoding="utf-8")
        shipped_body = "\n".join(line for line in shipped.splitlines()
                                 if not line.startswith("//"))
        self.assertEqual(body, shipped_body)

    def test_comments_include_name_and_grid(self):
        text = xkb.symbols(LAYOUT, "shiro")
        self.assertIn("shiro layout", text)
        self.assertIn("// top:  ; n t g f  j o e h k", text)
        self.assertIn("// home: r p m s c  u a y d i", text)
        self.assertIn("// bot:  l b q v w  x z , . /", text)

    def test_punctuation_and_iso_keys(self):
        text = xkb.symbols(LAYOUT, "shiro")
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
            xkb.symbols(LAYOUT[:-1] + "a", "bad")
        with self.assertRaisesRegex(ValueError, "unsupported"):
            xkb.symbols(LAYOUT[:-1] + "!", "bad")

    def test_safe_name_is_not_mine(self):
        name = xkb.safe_name(LAYOUT)
        self.assertNotEqual(name, "mine")
        self.assertTrue(name.startswith("shiro_"))
        self.assertEqual(name, xkb.safe_name(LAYOUT))

    def test_write_named_uses_xkb_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = xkb.write_named(LAYOUT, "shiro", directory=tmp)
            self.assertEqual(Path(path).name, "shiro")
            self.assertIn("shiro layout", Path(path).read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
