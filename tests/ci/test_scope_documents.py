"""Documentation and shared tooling CI selection."""

import unittest

from test_scope import scope, selected


class DocumentScopeTests(unittest.TestCase):
    def test_documentation_avoids_builds(self):
        for path in (
            "docs/recite-production-spec.md",
            "README.md",
            "apps/writer/acceptance.md",
            "apps/writer/packaging.md",
            "docs/LICENSE-POLICY.md",
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"docs", "maintainability"})

    def test_site_and_shared_javascript_do_not_select_native_builds(self):
        for path in (
            "docs-site/src/content/docs/index.md",
            "docs-site/src/content/docs/reference/index.md",
            "docs-site/justfile",
        ):
            self.assertEqual(selected(path), {"docs", "site", "maintainability"})
        self.assertEqual(selected("docs-site/README.md"), {"docs", "maintainability"})
        for path in ("docs-site/astro.config.mjs", "docs-site/check-browser.sh"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"docs", "site", "maintainability"})
        for path in ("package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"):
            with self.subTest(path=path):
                self.assertEqual(selected(path), scope.JS | {"maintainability"})
        self.assertEqual(
            selected("scripts/install-js-dependencies.sh"), scope.JS | {"maintainability"}
        )
        for path in (
            "scripts/check-docs.sh",
            "scripts/check-schema-manifest.mjs",
            "scripts/check-site-links.py",
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"docs", "site", "maintainability"})
        self.assertEqual(selected(".gitignore"), {"rust", "maintainability"})
        for path, lanes in {
            "apps/writer/justfile": {"writer", "maintainability"},
            "editors/justfile": {"rust", "editor-native", "maintainability"},
            "editors/zed/justfile": {"editor-native", "maintainability"},
            "engines.just": {"hosts", "maintainability"},
            "stress.just": scope.JUST,
        }.items():
            self.assertEqual(selected(path), lanes)
        for path in ("scripts/check-project-gates.sh", "scripts/check-ffi-header.sh"):
            self.assertEqual(selected(path), scope.RUST)
        self.assertEqual(selected("scripts/check-zed.sh"), {"editor-native", "maintainability"})
        self.assertEqual(selected("justfile"), scope.JUST)
        self.assertEqual(
            selected("docs-site/src/content/docs/index.md", "justfile", "apps/writer/justfile"),
            {"docs", "site", "writer", "maintainability"},
        )
        self.assertIn("site", selected("crates/recite-runtime/src/lib.rs"))
        self.assertIn("docs", selected("crates/recite-runtime/src/lib.rs"))
        for path in (
            "Cargo.toml",
            "Cargo.lock",
            "crates/recite-core/Cargo.toml",
            "apps/writer/Cargo.lock",
            ".cargo/config.toml",
            ".cargo/mutants.toml",
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), scope.RUST_BUILD)

    def test_executable_documentation_selects_its_rust_test_owner(self):
        for path in (
            "docs-site/src/content/docs/getting-started/first-scene.md",
            "docs-site/src/content/docs/migration/index.md",
            "docs-site/src/content/docs/migration/new-tool.md",
        ):
            with self.subTest(path=path):
                self.assertEqual(selected(path), {"docs", "site", "rust", "maintainability"})
