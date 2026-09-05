#!/usr/bin/env python3
"""Apply one isolated mutation to the editor-parity fixture contract."""

import json
import os
import shutil
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 3:
        raise SystemExit("usage: mutate_fixture.py CONTRACT MUTATION")
    path = Path(sys.argv[1])
    mutation = sys.argv[2]
    with path.open(encoding="utf-8") as handle:
        contract = json.load(handle)
    fixture_repo = path.parents[2]

    if mutation == "traversal":
        contract["artifacts"][0]["path"] = "../../outside/claimed.vsix"
    elif mutation == "client":
        client = record(contract, "clients", "vscode")
        client["status"] = "implemented"
        client["platform_status"] = {"linux": "implemented", "macos": "partial", "windows": "partial"}
        record(contract, "artifacts", "vscode-vsix")["status"] = "planned"
    elif mutation == "distribution":
        record(contract, "distributions", "vs-marketplace")["status"] = "implemented"
    elif mutation == "capability-platform":
        record(contract, "capabilities", "lsp.completion")["platform_status"]["linux"] = "implemented"
    elif mutation == "capability-evidence":
        record(contract, "capabilities", "lsp.completion")["expected_evidence"]["status"] = "implemented"
    elif mutation == "duplicate":
        contract["capabilities"].append(dict(contract["capabilities"][0]))
    elif mutation == "malformed":
        evidence = record(contract, "capabilities", "lsp.completion")["expected_evidence"]
        evidence.pop("commands", None)
        evidence["command"] = "not a cargo test command"
    elif mutation == "stale-evidence":
        evidence = record(contract, "capabilities", "lsp.completion")["expected_evidence"]
        evidence.pop("commands", None)
        evidence["command"] = "cargo test --locked -p recite-lsp --test editor_parity no_such_test"
    elif mutation == "stale-module-evidence":
        evidence = record(contract, "capabilities", "command.structured.results")["expected_evidence"]
        evidence.pop("commands", None)
        evidence["command"] = (
            "cargo test --locked -p recite-compiler --test authoring_build "
            "invented::projects_every_lifecycle_state_with_stable_fields"
        )
    elif mutation == "preserved-mtime-disconnected-module":
        root = fixture_repo / "crates/recite-compiler/tests/authoring_build.rs"
        source = root.read_text(encoding="utf-8")
        declaration = '#[path = "authoring_build/status_projection.rs"]\nmod status_projection;\n'
        if declaration not in source:
            raise SystemExit("status projection module declaration was not present")
        restore_mtime(root, source.replace(declaration, "", 1))
    elif mutation == "block-commented-stale-test":
        root = fixture_repo / "crates/recite-compiler/tests/authoring_catalog_summary.rs"
        source = root.read_text(encoding="utf-8")
        original = "#[test]\nfn checked_in_locale_fallback_catalogue_resolves_deterministically() {}"
        replacement = "/* #[test]\nfn checked_in_locale_fallback_catalogue_resolves_deterministically() {} */"
        if original not in source:
            raise SystemExit("catalogue test was not present")
        restore_mtime(root, source.replace(original, replacement, 1))
    elif mutation == "block-commented-include-test":
        set_module_shapes_command(contract)
        root = fixture_repo / "crates/recite-lsp/tests/module_tests.inc"
        source = root.read_text(encoding="utf-8")
        original = "        #[test]\n        fn nested_test() {}"
        replacement = "        /* #[test]\n        fn nested_test() {} */"
        if original not in source:
            raise SystemExit("included nested test was not present")
        restore_mtime(root, source.replace(original, replacement, 1))
    elif mutation == "build-input":
        set_module_shapes_command(contract)
        root = fixture_repo / "crates/recite-lsp/build.rs"
        source = root.read_text(encoding="utf-8")
        original = 'include!("../../shared-build.inc");'
        if original not in source:
            raise SystemExit("build input include was not present")
        restore_mtime(root, source.replace(original, f"/* {original} */", 1))
    elif mutation == "shared-build-input":
        set_module_shapes_command(contract)
        root = fixture_repo / "shared-build.inc"
        source = root.read_text(encoding="utf-8")
        original = 'pub const BUILD_SHARED: &str = "build input";'
        if original not in source:
            raise SystemExit("shared build input was not present")
        restore_mtime(root, source.replace(original, f"/* {original} */", 1))
    elif mutation == "shared-workspace-input":
        set_module_shapes_command(contract)
        root = fixture_repo / "shared_workspace.rs"
        source = root.read_text(encoding="utf-8")
        original = 'pub const WORKSPACE_SHARED: &str = "workspace input";'
        if original not in source:
            raise SystemExit("shared workspace input was not present")
        restore_mtime(root, source.replace(original, f"/* {original} */", 1))
    elif mutation == "compiler-diagnostic":
        set_module_shapes_command(contract)
        root = fixture_repo / "crates/recite-lsp/tests/module_shapes.rs"
        source = root.read_text(encoding="utf-8")
        marker = 'include!("module_tests.inc");'
        if marker not in source:
            raise SystemExit("module-shapes include was not present")
        detail = "editor parity compiler diagnostic fixture " + ("x" * 8000)
        replacement = f'compile_error!("{detail}");\n\n{marker}'
        restore_mtime(root, source.replace(marker, replacement, 1))
    elif mutation == "contained-file-link":
        create_digest_symlink_fixture(fixture_repo, "contained-file-link", False)
    elif mutation == "escaping-file-link":
        create_digest_symlink_fixture(fixture_repo, "escaping-file-link", True)
    elif mutation == "contained-directory-link":
        create_digest_symlink_fixture(fixture_repo, "contained-directory-link", False, directory=True)
    elif mutation == "symlink-cycle":
        digest_root = fixture_repo / "digest-inputs"
        first = digest_root / "cycle-a"
        second = digest_root / "cycle-b"
        first.mkdir(parents=True)
        second.mkdir()
        (first / "to-b").symlink_to(second)
        (second / "to-a").symlink_to(first)
    elif mutation == "module-shapes":
        set_module_shapes_command(contract)
    elif mutation == "evidence-traversal":
        evidence = record(contract, "capabilities", "lsp.completion")["expected_evidence"]
        evidence.pop("commands", None)
        evidence["command"] = (
            "cargo test --locked -p ../../outside --test editor_parity "
            "project_root_discovers_canonical_multi_file_overlays_for_navigation"
        )
    elif mutation == "orphan-utf16":
        scenario = record(contract, "scenarios", "utf16-crlf-non-bmp")
        orphan = dict(scenario)
        orphan["id"] = "orphan-utf16-crlf-non-bmp"
        contract["scenarios"].append(orphan)
    elif mutation == "neovim-client-topology":
        record(contract, "clients", "neovim")["artifact"] = "tree-sitter-grammar"
    elif mutation == "zed-tree-sitter-claim":
        record(contract, "clients", "zed")["artifacts"] = ["zed-extension", "tree-sitter-grammar"]
        record(contract, "artifacts", "tree-sitter-grammar")["clients"].append("zed")
    elif mutation == "client-platform-shape":
        record(contract, "clients", "neovim")["platform_status"] = ["linux", "macos", "windows"]
    elif mutation == "implemented-client-platform-shape":
        client = record(contract, "clients", "neovim")
        client["status"] = "implemented"
        client["platform_status"] = ["linux", "macos", "windows"]
    elif mutation == "neovim-evidence-shape":
        record(contract, "capabilities", "editor.neovim.syntax-projection")["expected_evidence"] = ["broken evidence shape"]
    elif mutation == "implementation-status-shape":
        record(contract, "capabilities", "lsp.completion")["implementation_status"] = ["partial"]
    elif mutation == "status-values-shape":
        contract["status_values"] = [{"invalid": "shape"}]
    elif mutation == "client-artifacts-shape":
        record(contract, "clients", "neovim")["artifacts"] = {"invalid": "shape"}
    elif mutation == "distribution-artifacts-shape":
        record(contract, "distributions", "neovim-distribution")["artifacts"] = {"invalid": "shape"}
    elif mutation == "evidence-artifacts-shape":
        evidence = record(contract, "capabilities", "lsp.completion")["expected_evidence"]
        evidence.pop("artifact", None)
        evidence["artifacts"] = {"invalid": "shape"}
    elif mutation == "follow-up-shape":
        record(contract, "capabilities", "lsp.completion")["follow_up"] = []
    elif mutation == "keyboard-follow-up":
        record(contract, "capabilities", "editor.keyboard.workflow")["follow_up"] = "#192"
    elif mutation == "keyboard-follow-up-missing":
        record(contract, "capabilities", "editor.keyboard.workflow").pop("follow_up")
    elif mutation == "keyboard-scenario-status":
        record(contract, "scenarios", "keyboard-workflow")["status"] = "implemented"
    elif mutation == "keyboard-executable-evidence":
        evidence = record(contract, "capabilities", "editor.keyboard.workflow")["expected_evidence"]
        evidence["commands"] = ["scripts/check-vscode.sh"]
    elif mutation == "keyboard-evidence-boundary":
        capability = record(contract, "capabilities", "editor.keyboard.workflow")
        capability["known_limitation"] = capability["known_limitation"].replace("headless", "protocol")
    elif mutation == "keyboard-document-wording":
        document = fixture_repo / "docs/editor-parity-contract.md"
        marker = "broader Milestone 5 accessibility proof"
        source = document.read_text(encoding="utf-8")
        if marker not in source:
            raise SystemExit("keyboard documentation wording was not present")
        document.write_text(source.replace(marker, "accessibility proof"), encoding="utf-8")
    elif mutation == "neovim-stale-filetype":
        capability = record(contract, "capabilities", "editor.filetype.registration")
        capability["known_limitation"] = "No client package or activation registration exists."
        capability["platform_status"]["linux"] = "planned"
    elif mutation == "disconnected-module":
        root = fixture_repo / "crates/recite-compiler/tests/authoring_build.rs"
        source = root.read_text(encoding="utf-8")
        declaration = '#[path = "authoring_build/status_projection.rs"]\nmod status_projection;\n'
        if declaration not in source:
            raise SystemExit("status projection module declaration was not present")
        root.write_text(source.replace(declaration, "", 1), encoding="utf-8")
    elif mutation == "reciprocity":
        record(contract, "artifacts", "vscode-vsix")["clients"].remove("vscode")
    elif mutation == "topology":
        record(contract, "clients", "vscodium")["artifact"] = "tree-sitter-grammar"
    elif mutation == "wrong-primary":
        record(contract, "distributions", "neovim-distribution")["artifact"] = "tree-sitter-grammar"
    elif mutation == "missing-grammar-support":
        record(contract, "distributions", "neovim-distribution")["artifacts"].remove("tree-sitter-grammar")
    elif mutation == "unknown-supporting-artifact":
        record(contract, "distributions", "neovim-distribution")["artifacts"].append("unknown-editor-artifact")
    elif mutation == "symlink":
        outside = fixture_repo.parent / "outside-editor-parity.recite"
        outside.write_text("outside\n", encoding="utf-8")
        canonical = fixture_repo / "fixtures/recite/valid/core_language_spike.recite"
        canonical.unlink()
        canonical.symlink_to(outside)
    elif mutation == "symlink-component":
        valid = fixture_repo / "fixtures/recite/valid"
        internal = fixture_repo / "fixtures/recite/internal-valid"
        shutil.copytree(valid, internal, symlinks=True)
        shutil.rmtree(valid)
        valid.symlink_to(internal, target_is_directory=True)
    elif mutation == "symlink-artifact-component":
        alias = fixture_repo / "fixtures/editor-parity/artifact-alias"
        alias.symlink_to(alias.parent, target_is_directory=True)
        artifact = record(contract, "artifacts", "vscode-vsix")
        artifact["status"] = "implemented"
        artifact["path"] = "fixtures/editor-parity/artifact-alias/contract.json"
    elif mutation == "symlink-contract-control":
        outside = fixture_repo.parent / "outside-editor-parity-contract.json"
        outside.write_text(path.read_text(encoding="utf-8"), encoding="utf-8")
        path.unlink()
        path.symlink_to(outside)
    elif mutation == "symlink-document-control":
        document = fixture_repo / "docs/editor-parity-contract.md"
        outside = fixture_repo.parent / "outside-editor-parity-document.md"
        outside.write_text(document.read_text(encoding="utf-8"), encoding="utf-8")
        document.unlink()
        document.symlink_to(outside)
    else:
        raise SystemExit(f"unknown mutation: {mutation}")

    with path.open("w", encoding="utf-8") as handle:
        json.dump(contract, handle, indent=2)
        handle.write("\n")
    return 0


def record(contract: dict, collection: str, identifier: str) -> dict:
    return next(record for record in contract[collection] if record["id"] == identifier)


def restore_mtime(path: Path, content: str) -> None:
    original_stat = path.stat()
    path.write_text(content, encoding="utf-8")
    os.utime(path, ns=(original_stat.st_atime_ns, original_stat.st_mtime_ns))


def set_module_shapes_command(contract: dict) -> None:
    record(contract, "capabilities", "lsp.completion")["expected_evidence"]["commands"][0] = (
        "cargo test --locked -p recite-lsp --test module_shapes inline::nested::nested_test"
    )


def create_digest_symlink_fixture(fixture_repo: Path, name: str, escaping: bool, directory: bool = False) -> None:
    digest_root = fixture_repo / "digest-inputs"
    digest_root.mkdir(exist_ok=True)
    if directory:
        target = digest_root / "contained-directory"
        target.mkdir()
        target.joinpath("input.txt").write_text("input\n", encoding="utf-8")
    else:
        target = digest_root / "contained-input.txt"
        target.write_text("input\n", encoding="utf-8")
    link = digest_root / name
    link.symlink_to(fixture_repo.parent / "outside-input" if escaping else target)


if __name__ == "__main__":
    raise SystemExit(main())
