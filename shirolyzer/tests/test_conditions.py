import unittest

import numpy as np

from shirolyzer.utils import conditions as CD


class ConditionOrderTests(unittest.TestCase):
    def tearDown(self):
        CD.configure([], enabled=False)

    def test_specific_half_scissor_overrides_general_allow(self):
        conditions = [
            {"type": "scissors", "distance": "half", "fingers": ["LP", "RP"], "allow": False},
            {"type": "scissors", "distance": "half", "allow": True},
        ]
        CD.configure(conditions, enabled=True)
        mask = CD.penalty_mask(("A", "W"))
        self.assertTrue(np.any(mask > 0))

    def test_reversed_order_allows_all_half_scissors(self):
        conditions = [
            {"type": "scissors", "distance": "half", "allow": True},
            {"type": "scissors", "distance": "half", "fingers": ["LP", "RP"], "allow": False},
        ]
        CD.configure(conditions, enabled=True)
        mask = CD.penalty_mask(("A", "W"))
        self.assertFalse(np.any(mask > 0))


if __name__ == "__main__":
    unittest.main()
