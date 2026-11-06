"""Negative checks for shipping authorization and execution evidence."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import delivery


def package(name, deps=(), publish=None):
    return {"id": name, "name": name, "publish": publish,
            "dependencies": [{"name": dep, "path": f"crates/{dep}", "req": "^0.1.0"} for dep in deps]}


class GateTests(unittest.TestCase):
    def test_publish_false_is_a_blocker(self):
        with self.assertRaisesRegex(ValueError, "publish=false"):
            delivery.release_packages({"workspace_members": ["a"], "packages": [package("a", publish=[])]})

    def test_dependencies_and_dev_dependencies_precede_consumers(self):
        data = {"workspace_members": ["a", "b", "c"],
                "packages": [package("a", ("b", "c")), package("b", ("c",)), package("c")]}
        self.assertEqual([p["name"] for p in delivery.release_packages(data)], ["c", "b", "a"])
        self.assertEqual(delivery.package_waves(data), [["c"], ["b"], ["a"]])

    def test_unpublished_dependency_and_cycle_fail(self):
        for packages in ([package("a", ("b",)), package("b", publish=[])],
                         [package("a", ("b",)), package("b", ("a",))]):
            with self.assertRaises(ValueError):
                delivery.release_packages({"workspace_members": ["a", "b"], "packages": packages})

    def test_skipped_or_failed_job_cannot_authorize_release(self):
        for result in ("skipped", "cancelled", "failure", "timed_out", "neutral"):
            with self.assertRaises(ValueError):
                delivery.check_jobs(json.dumps({"docker": {"result": result}}))
        delivery.check_jobs(json.dumps({"docker": {"result": "success"}}))

    def test_private_attacker_and_credential_package_files_rejected(self):
        for file in (".spec.local/evidence.json", "bots/agent.js", "redteam/catalog.rs", "a/secret.key", "../leak", "/absolute"):
            with self.assertRaises(ValueError):
                delivery.safe_files([file])
        delivery.safe_files(["src/lib.rs", "tests/required.rs"], rust=True)

    def report(self, kind="docker"):
        return {"schema": 1, "source_sha": "a" * 40, "kind": kind, "status": "passed",
                "documentation_review_sha256": "d" * 64,
                "cases": [{"id": case, "status": "passed"} for case in sorted(delivery.CASES[kind])],
                "errors": 0, "skips": 0, "isolation": "docker", "runtime": "rust", "images": ["sha256:" + "b" * 64]}

    def test_stale_go_loopback_skipped_empty_and_failed_reports_rejected(self):
        delivery.validate_report(self.report(), "docker", "a" * 40, "d" * 64)
        for field, value in (("source_sha", "c" * 40), ("runtime", "go"), ("isolation", "host"),
                             ("documentation_review_sha256", "e" * 64),
                             ("skips", 1), ("errors", 1), ("cases", []), ("images", ["latest"])):
            report = self.report()
            report[field] = value
            with self.assertRaises(ValueError, msg=field):
                delivery.validate_report(report, "docker", "a" * 40, "d" * 64)
        report = self.report()
        report["cases"][0]["status"] = "skipped"
        with self.assertRaises(ValueError):
            delivery.validate_report(report, "docker", "a" * 40, "d" * 64)

    def test_required_case_and_duplicate_case_rejected(self):
        for mutation in (lambda r: r["cases"].pop(), lambda r: r["cases"].append(r["cases"][0])):
            report = self.report()
            mutation(report)
            with self.assertRaises(ValueError):
                delivery.validate_report(report, "docker", "a" * 40, "d" * 64)

    def test_policy_rejects_unpinned_actions_and_missing_docker_gate(self):
        import yaml
        original = yaml.safe_load((delivery.ROOT / ".github/workflows/rust-validation.yml").read_text(encoding="utf-8"))
        for kind in ("pin", "docker", "conditional", "tolerated"):
            doc = copy.deepcopy(original)
            if kind == "pin":
                doc["jobs"]["native"]["steps"][0]["uses"] = "actions/checkout@v4"
            elif kind == "docker":
                doc["jobs"]["ready"]["needs"].remove("docker")
            elif kind == "conditional":
                doc["jobs"]["docker"]["if"] = "false"
            else:
                doc["jobs"]["docker"]["continue-on-error"] = True
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                folder = root / ".github/workflows"
                folder.mkdir(parents=True)
                (folder / "rust-validation.yml").write_text(yaml.safe_dump(doc), encoding="utf-8")
                with patch.object(delivery, "ROOT", root), self.assertRaises(ValueError, msg=kind):
                    delivery.policy()

    def test_unit_only_parity_or_removed_baseline_rows_cannot_ship(self):
        for state, baseline in (("ported_unit_parity_not_runtime", "cmd/main.go\n"),
                                ("verified_runtime_parity", "cmd/main.go\ncmd/omitted.go\n")):
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                folder = root / ".spec/evidence/20261106194112-go-migration-source-inventory"
                folder.mkdir(parents=True)
                (folder / "go-inventory.json").write_text(json.dumps({
                    "baseline": "981d25cf71a5a54855d7e2b4359e832a9d5998e5",
                    "files": [{"path": "cmd/main.go", "parity_status": state}]}))
                with patch.object(delivery, "ROOT", root), patch.object(delivery, "run", return_value=baseline):
                    with self.assertRaises(ValueError):
                        delivery.migration_acceptance()

if __name__ == "__main__":
    unittest.main()
