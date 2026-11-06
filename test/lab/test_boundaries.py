"""Security boundaries of host target selection and orphan cleanup."""
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "deployments/lab/clients"))
from probe import private_address
from cleanup import eligible


class Boundaries(unittest.TestCase):
    def test_targets_refuse_non_docker_address_classes(self):
        for target in ["https://example.com", "1.1.1.1", "127.0.0.1", "169.254.169.254", "0.0.0.0", "192.0.2.1", "::1", "::ffff:172.20.0.2", "origin"]:
            with self.subTest(target=target), self.assertRaises(ValueError):
                private_address(target)
        self.assertEqual(private_address("172.20.0.2"), "172.20.0.2")

    def test_janitor_requires_owner_project_and_expiry(self):
        labels = {"org.humanymous.owner": "rust-product-lab", "com.docker.compose.project": "hm-rust-lab-0123456789ab", "org.humanymous.expires": "100"}
        self.assertTrue(eligible(labels, 101))
        for key, value in [("org.humanymous.owner", "another-session"), ("com.docker.compose.project", "production"), ("org.humanymous.expires", "0"), ("org.humanymous.expires", "102"), ("org.humanymous.expires", "garbage")]:
            self.assertFalse(eligible({**labels, key: value}, 101))


if __name__ == "__main__":
    unittest.main()
