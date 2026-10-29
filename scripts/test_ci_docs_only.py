import unittest
from ci_docs_only import docs_only


class DocsOnlyTests(unittest.TestCase):
    def test_documentation_paths(self):
        self.assertTrue(docs_only(['README.md', 'AGENTS.md', 'docs/example.md']))

    def test_mixed_or_sensitive_paths_run_checks(self):
        for paths in ([], ['docs/example.md', 'Cargo.toml'], ['.github/workflows/ci.yml'], ['docs/code.rs'], ['crates/example.md']):
            with self.subTest(paths=paths):
                self.assertFalse(docs_only(paths))


if __name__ == '__main__':
    unittest.main()
