import unittest

from shirolyzer.tests.fixtures import CURRENT_LAYOUT
from shirolyzer.utils import diversity


A = CURRENT_LAYOUT
B = A[::-1]
C = A[10:] + A[:10]


def swapped(layout):
    chars = list(layout)
    chars[0], chars[1] = chars[1], chars[0]
    return "".join(chars)


class DiversityTests(unittest.TestCase):
    def test_layout_distance(self):
        self.assertEqual(diversity.layout_distance("abc", "abc"), 0)
        self.assertEqual(diversity.layout_distance("abc", "axc"), 1)
        self.assertEqual(diversity.layout_distance("abc", "xyz"), 3)
        with self.assertRaises(ValueError):
            diversity.layout_distance("abc", "ab")

    def test_merge_results_keeps_best_scores(self):
        rows = [
            {"start": "a", "layout": "abc", "primary": 3.5, "secondary": None},
            {"start": "b", "layout": "abc", "primary": 3.4, "secondary": None},
            {"start": "c", "layout": "abc", "primary": 3.45, "secondary": 3.6},
        ]
        merged = diversity.merge_results(rows)
        self.assertEqual(len(merged), 1)
        self.assertEqual(merged[0]["primary"], 3.4)
        self.assertEqual(merged[0]["secondary"], 3.6)
        self.assertEqual(merged[0]["final"], 3.6)

    def test_comparable_uses_both_tolerances(self):
        ref = {"primary": 3.50, "secondary": 3.60}
        near = {"primary": 3.501, "secondary": 3.604}
        better = {"primary": 3.49, "secondary": 3.59}
        bad_primary = {"primary": 3.51, "secondary": 3.60}
        bad_secondary = {"primary": 3.50, "secondary": 3.62}
        missing_secondary = {"primary": 3.50, "secondary": None}
        self.assertTrue(diversity.comparable(near, ref, 0.002, 0.005))
        self.assertTrue(diversity.comparable(better, ref, 0.002, 0.005))
        self.assertFalse(diversity.comparable(bad_primary, ref, 0.002, 0.005))
        self.assertFalse(diversity.comparable(bad_secondary, ref, 0.002, 0.005))
        self.assertFalse(diversity.comparable(missing_secondary, ref, 0.002, 0.005))

    def test_comparable_ignores_secondary_without_reference(self):
        ref = {"primary": 3.50, "secondary": None}
        row = {"primary": 3.501, "secondary": 3.90}
        self.assertTrue(diversity.comparable(row, ref, 0.002, 0.005))

    def test_clear_winner(self):
        ref = {"start": "ref", "layout": A, "primary": 3.50, "secondary": 3.60}
        rows = [ref,
                {"start": "x", "layout": B, "primary": 3.53, "secondary": 3.60}]
        selected, truncated = diversity.select_valid_layouts(rows, ref)
        self.assertEqual([row["layout"] for row in selected], [A])
        self.assertFalse(truncated)

    def test_near_duplicate_excluded(self):
        ref = {"start": "ref", "layout": A, "primary": 3.50, "secondary": 3.60}
        near = {"start": "near", "layout": swapped(A), "primary": 3.501,
                "secondary": 3.601}
        selected, truncated = diversity.select_valid_layouts(
            [ref, near], ref, min_distance=4)
        self.assertEqual([row["layout"] for row in selected], [A])
        self.assertFalse(truncated)

    def test_multiple_valid_kept(self):
        ref = {"start": "ref", "layout": A, "primary": 3.50, "secondary": 3.60}
        alt = {"start": "alt", "layout": B, "primary": 3.501,
               "secondary": 3.601}
        selected, truncated = diversity.select_valid_layouts(
            [ref, alt], ref, min_distance=4)
        self.assertEqual([row["layout"] for row in selected], [A, B])
        self.assertFalse(truncated)

    def test_better_secondary_is_valid(self):
        ref = {"start": "ref", "layout": A, "primary": 3.50, "secondary": 3.70}
        alt = {"start": "alt", "layout": B, "primary": 3.501,
               "secondary": 3.60}
        selected, truncated = diversity.select_valid_layouts(
            [ref, alt], ref, min_distance=4)
        self.assertEqual([row["layout"] for row in selected], [A, B])
        self.assertFalse(truncated)

    def test_reference_is_always_included(self):
        ref = {"start": "ref", "layout": A, "primary": 3.50, "secondary": 3.60}
        selected, _ = diversity.select_valid_layouts([], ref)
        self.assertEqual([row["layout"] for row in selected], [A])

    def test_truncation(self):
        ref = {"start": "ref", "layout": A, "primary": 3.50, "secondary": 3.60}
        rows = [ref,
                {"start": "b", "layout": B, "primary": 3.50, "secondary": 3.60},
                {"start": "c", "layout": C, "primary": 3.50, "secondary": 3.60}]
        selected, truncated = diversity.select_valid_layouts(
            rows, ref, min_distance=4, max_valid=2)
        self.assertEqual(len(selected), 2)
        self.assertTrue(truncated)

    def test_format_valid_layouts(self):
        rows = [{"layout": A, "primary": 3.5, "secondary": 3.6}]
        lines = diversity.format_valid_layouts(rows, "clear_winner")
        self.assertIn("valid_layouts=1 status=clear_winner", lines[0])
        self.assertIn(A, lines[1])

    def test_valid_payload(self):
        rows = [{"start": "ref", "layout": A, "primary": 3.5,
                 "secondary": 3.6, "reference_name": "pick"}]
        payload = diversity.valid_payload(rows)
        self.assertEqual(payload[0]["name"], "pick")
        self.assertEqual(payload[0]["layout"], A)
        self.assertEqual(payload[0]["start"], "ref")


if __name__ == "__main__":
    unittest.main()
