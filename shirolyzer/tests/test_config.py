import copy
import tempfile
import unittest
from pathlib import Path

import yaml

from shirolyzer.utils import config as CFG
from shirolyzer.utils import paths


class ConfigLoaderTests(unittest.TestCase):
    def setUp(self):
        self.raw = yaml.safe_load(paths.CONFIG_FILE.read_text(encoding="utf-8"))

    def write(self, data):
        tmp = tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False)
        yaml.safe_dump(data, tmp)
        tmp.close()
        self.addCleanup(Path(tmp.name).unlink)
        return Path(tmp.name)

    def test_default_loads(self):
        cfg = CFG.load()
        self.assertEqual(len(cfg.finger_tokens), 30)
        self.assertEqual(cfg.fingermap["A"], "P")
        self.assertEqual(cfg.model_variant, "A")
        self.assertEqual(sum(cfg.flat_share.values()), 1.0)

    def test_explicit_path_matches_default(self):
        a, b = CFG.load(), CFG.load(paths.CONFIG_FILE)
        self.assertEqual(a.fingermap, b.fingermap)
        self.assertEqual(a.conditions, b.conditions)
        self.assertEqual(a.priority_list, b.priority_list)

    def test_scalar_corpus_is_normalised(self):
        data = copy.deepcopy(self.raw)
        data["general"]["primary_corpus"] = "shai"
        cfg = CFG.load(self.write(data))
        self.assertEqual(cfg.primary_corpus[0]["name"], "shai")
        self.assertEqual(cfg.primary_corpus[0]["weight"], 1.0)

    def test_invalid_fingermap_key_is_actionable(self):
        data = copy.deepcopy(self.raw)
        data["fingermap"] = data["fingermap"].replace("QR", "XR", 1)
        with self.assertRaisesRegex(CFG.ConfigError, "fingermap\\[0\\]"):
            CFG.load(self.write(data))

    def test_invalid_priority_is_actionable(self):
        data = copy.deepcopy(self.raw)
        data["priority_list"][0] = "unknown-stat"
        with self.assertRaisesRegex(CFG.ConfigError, "priority_list"):
            CFG.load(self.write(data))

    def test_missing_theta_is_actionable(self):
        data = copy.deepcopy(self.raw)
        data["model"]["theta_file"] = "model/nope.json"
        with self.assertRaisesRegex(CFG.ConfigError, "theta file not found"):
            CFG.load(self.write(data))

    def test_alt_sequence_accepts_arbitrary_length(self):
        data = copy.deepcopy(self.raw)
        data["alt_fingers"].append({"sequence": "fwxa", "fingers": ["LM", "LI", "LM", "LP"]})
        cfg = CFG.load(self.write(data))
        sequence = [e for e in cfg.alt_fingers if "sequence" in e][0]
        self.assertEqual(sequence["sequence"], "FWXA")

    def test_alt_sequence_rejects_wrong_hand(self):
        data = copy.deepcopy(self.raw)
        data["alt_fingers"].append({"sequence": "fj", "fingers": ["LM", "LM"]})
        with self.assertRaisesRegex(CFG.ConfigError, "does not match the hand"):
            CFG.load(self.write(data))


if __name__ == "__main__":
    unittest.main()


class PriorityTests(unittest.TestCase):
    def setUp(self):
        self.raw = yaml.safe_load(paths.CONFIG_FILE.read_text(encoding="utf-8"))

    def write(self, data):
        tmp = tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False)
        yaml.safe_dump(data, tmp)
        tmp.close()
        self.addCleanup(Path(tmp.name).unlink)
        return Path(tmp.name)

    def test_default_priority_scales_are_neutral(self):
        cfg = CFG.load()
        self.assertEqual(set(cfg.priority_multipliers().values()), {1.0})

    def test_reordered_priority_changes_scale(self):
        data = copy.deepcopy(self.raw)
        data["priority_list"] = [
            ["finger_balance"],
            ["rolls", "alternation"],
            ["scissors", "redirects"],
            ["sfbs-sfts"],
        ]
        cfg = CFG.load(self.write(data))
        scales = cfg.priority_multipliers()
        self.assertGreater(scales["finger_balance"], scales["sfbs-sfts"])
        self.assertEqual(scales["rolls"], scales["alternation"])

    def test_reordered_priority_applies_to_search_weights(self):
        data = copy.deepcopy(self.raw)
        data["priority_list"] = [
            ["finger_balance"],
            ["rolls", "alternation"],
            ["scissors", "redirects"],
            ["sfbs-sfts"],
        ]
        path = self.write(data)
        try:
            CFG.load_and_apply(path)
            from shirolyzer.utils import comfort as cg
            from shirolyzer.utils import search as sh
            self.assertGreater(sh.W_BALANCE, 0.4)
            self.assertGreater(cg.W_ALT_SCALE, 1.0)
            self.assertGreater(cg.W_SFB_SCALE, 0.0)
        finally:
            CFG.load_and_apply(paths.CONFIG_FILE)
