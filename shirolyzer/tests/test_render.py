import unittest

from shirolyzer.tests.fixtures import CURRENT_LAYOUT
from shirolyzer.utils import config
from shirolyzer.utils import render

LAYOUT = CURRENT_LAYOUT


class RenderTests(unittest.TestCase):
    def test_plain_grid(self):
        self.assertEqual(
            render.render(LAYOUT),
            "; n t g f  j o e h k\n"
            " r p m s c  u a y d i\n"
            "  l b q v w  x z , . /",
        )

    def test_annotated_grid(self):
        fingers = config.load().finger_tokens
        self.assertEqual(
            render.render(LAYOUT, fingers),
            ";R nR tM gM fM  jM oM eM hR kR\n"
            " rP pR mM sI cI  uI aI yM dR iP\n"
            "  lP bR qI vI wI  xI zI ,R .R /P",
        )

    def test_rejects_wrong_lengths(self):
        with self.assertRaisesRegex(ValueError, "30 characters"):
            render.render("abc")
        with self.assertRaisesRegex(ValueError, "30 entries"):
            render.render(LAYOUT, ["P"] * 29)


if __name__ == "__main__":
    unittest.main()
