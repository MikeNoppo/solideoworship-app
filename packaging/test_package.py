import unittest

from package import msi_version


class Versions(unittest.TestCase):
    def test_msi_orders_betas_candidates_and_stable_within_three_fields(self):
        versions = ["0.1.0-beta.4", "0.1.0-beta.10", "0.1.0-rc.1", "0.1.0", "0.1.1-beta.1", "0.1.1", "0.2.0-beta.1"]
        numeric = [tuple(map(int, msi_version(version).split("."))) for version in versions]
        self.assertEqual(numeric, sorted(set(numeric)))

    def test_msi_rejects_versions_that_would_collide_or_overflow(self):
        for version in ["0.1.0-beta.0", "0.1.0-beta.500", "0.1.0-rc.499", "256.0.0", "0.256.0", "0.1.66", "0.1.0-alpha.1", "0.1.0+build"]:
            with self.subTest(version=version), self.assertRaises(ValueError):
                msi_version(version)


if __name__ == "__main__":
    unittest.main()
