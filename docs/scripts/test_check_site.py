"""Boundary checks for the read-only Pages validator."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('check_site', Path(__file__).with_name('check-site.py'))
check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(check)


class SourceBoundaryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.old = check.ROOT
        check.ROOT = Path(self.temp.name)
        self.page = check.ROOT / 'index.md'
        self.page.write_text('---\nlayout: doc\ntitle: Example\npermalink: /\nreviewed: 2026-11-07\n---\n'
                             '**Diátaxis quadrant:** Reference. **Audience:** reviewers.\n', encoding='utf-8')

    def tearDown(self):
        check.ROOT = self.old
        self.temp.cleanup()

    def test_missing_and_escaping_links_fail(self):
        self.page.write_text(self.page.read_text() + '[missing](absent.md) [escape](../outside.md)')
        self.assertEqual(len(check.source()), 2)

    def test_private_identifier_fails(self):
        self.page.write_text(self.page.read_text() + 'sot:root/private')
        self.assertTrue(any('private' in e or 'internal' in e for e in check.source()))

    def test_frontmatter_is_required(self):
        self.page.write_text('# Bare page')
        with self.assertRaisesRegex(ValueError, 'front matter'):
            check.source()

    def test_builtin_exclusion_and_no_writes(self):
        (check.ROOT / 'AGENTS.md').write_text('private instructions')
        before = self.page.read_bytes()
        self.assertEqual(check.source(), [])
        self.assertEqual(self.page.read_bytes(), before)


class OutputBoundaryTests(unittest.TestCase):
    setUp = SourceBoundaryTests.setUp
    tearDown = SourceBoundaryTests.tearDown
    def build(self, content):
        site = check.ROOT / '_site'
        site.mkdir(exist_ok=True)
        (site / 'index.html').write_text(content, encoding='utf-8')
        return site

    def test_canonical_and_subpath_escape_fail(self):
        site = self.build('<html lang="en"><link rel="canonical" href="https://humanymous.net/">'
                          '<main><h1>Example</h1><a href="/">escape</a></main></html>')
        errors = check.built(site, '/humanymous')
        self.assertTrue(any('canonical' in e for e in errors))
        self.assertTrue(any('baseurl' in e for e in errors))

    def test_missing_anchor_and_duplicate_id_fail(self):
        site = self.build('<html lang="en"><link rel="canonical" href="https://humanymous.net/">'
                          '<main><h1 id="same">Example</h1><p id="same">dup</p>'
                          '<a href="#absent">bad</a></main></html>')
        errors = check.built(site, '')
        self.assertTrue(any('duplicate id' in e for e in errors))
        self.assertTrue(any('anchor' in e for e in errors))

    def test_retired_output_fails(self):
        site = self.build('<html lang="en"><link rel="canonical" href="https://humanymous.net/">'
                          '<main><h1>Example</h1></main></html>')
        (site / 'reviews.json').write_text('{}')
        self.assertTrue(any('non-public' in e for e in check.built(site, '')))


if __name__ == '__main__':
    unittest.main()
