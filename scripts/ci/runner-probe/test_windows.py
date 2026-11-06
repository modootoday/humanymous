"""Coverage accounting negatives; never provisions local accounts."""
import json
from pathlib import Path
import tempfile
import unittest
from windows import manifest_from_messages, verify_results


class CoverageTests(unittest.TestCase):
    def test_manifest_ignores_non_test_artifacts_and_deduplicates(self):
        item = {"reason": "compiler-artifact", "profile": {"test": True}, "executable": "a.exe", "package_id": "pkg", "target": {"name": "a", "kind": ["bin"]}}
        rows = manifest_from_messages('\n'.join(map(json.dumps, [item, item, {"reason": "build-finished", "success": True}])))
        self.assertEqual(len(rows), 1)
        with self.assertRaises(RuntimeError):
            manifest_from_messages('{"reason":"build-finished"}')

    def check(self, listing, summary, manifest=None, rows=None):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            (directory / "harness-0-list.log").write_text(listing, encoding="utf-8")
            (directory / "harness-0-run.log").write_text(summary, encoding="utf-8")
            return verify_results(manifest or [{"executable": "a"}], {"harnesses": rows or [{"executable": "a", "list_exit": 0, "run_exit": 0}]}, directory)

    def test_complete_required_tests(self):
        result = self.check('one: test\ntwo: test\n', 'test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out')
        self.assertEqual(result['passed_tests'], 2)

    def test_ignored_filtered_empty_or_missing_summary_fail(self):
        for summary in ['test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out', 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out', 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out', '']:
            with self.subTest(summary=summary), self.assertRaises(RuntimeError):
                self.check('one: test\n', summary)
        with self.assertRaises(RuntimeError):
            self.check('', 'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out')

    def test_missing_manifest_harness_fails(self):
        with self.assertRaises(RuntimeError):
            self.check('', '', manifest=[{"executable": "a"}, {"executable": "b"}])


if __name__ == '__main__':
    unittest.main()
