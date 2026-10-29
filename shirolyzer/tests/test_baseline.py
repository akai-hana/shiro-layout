import subprocess
import sys
import unittest

from shirolyzer.utils import paths


class BaselineRegressionTests(unittest.TestCase):
    def test_default_config_reproduces_k12_baseline(self):
        result = subprocess.run(
            [sys.executable, str(paths.SHIROLYZER_ROOT / "check_baseline.py")],
            cwd=paths.REPO_ROOT, capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("baseline OK", result.stdout)


if __name__ == "__main__":
    unittest.main()
