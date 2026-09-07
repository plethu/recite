# Ki editor evaluation

Checked 2026-09-07 against the live `ki-editor/ki-editor` source at
[`b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd`](https://github.com/ki-editor/ki-editor/commit/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd).
This is a source evaluation, not an integration or support record.

## Snapshot

| Item | Finding |
| --- | --- |
| Upstream revision | `b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd`, committed 2026-09-04; `feat(languages): add WIT (.wit) language support (#1698)` |
| Version | `0.1.0` in both [`VERSION`](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/VERSION) and the [root Cargo package](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/Cargo.toml) |
| Status | Upstream's installation page says Ki is “under heavy development” and directs users to the latest GitHub build |
| Licence | Upstream is MPL-2.0. Recite copies no Ki code and adds no Ki dependency. |

The pinned source is the `master` line, rather than a Recite-specific package or
adapter. The version and status above are upstream statements, not a judgement
about whether Ki is ready for Recite.

## LSP surface observed in the pinned source

Ki starts a configured language-server process with piped stdin, stdout, and
stderr. It sends `initialize`, retains the server's returned capabilities, and
sends `initialized`; request handlers check the advertised capability before
sending a request. The adapter contains response handling for:

- diagnostics, completion and completion-item resolve;
- hover, definition, declaration, type definition, and implementation;
- references, prepare-rename, rename/workspace edits, and code actions;
- signature help, document symbols, and workspace symbols;
- workspace commands, call hierarchy, progress, and `workspace/applyEdit`.

This is the protocol surface implemented by Ki's client. It is not evidence
that a particular language server implements every operation, or that Ki has a
Recite language definition.

The relevant implementation is [`src/lsp/process.rs`](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/src/lsp/process.rs).

## Tests present at the pin

The dedicated LSP scenarios in [`src/test_lsp.rs`](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/src/test_lsp.rs) are:

- `rust_lsp_auto_import_from_completion_item`: diagnostics, completion,
  completion resolve, and an additional import edit;
- `typescript_lsp_workspace_symbols`: workspace-symbol lookup;
- `typescript_lsp_restart`: server restart, document reopen, hover, and
  workspace-symbol lookup after restart;
- `typescript_lsp_references`: reference lookup and quickfix-list presentation.

[`src/lsp/process.rs`](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/src/lsp/process.rs)
also has a process-level test for stopping after repeated invalid LSP input.
Other [`src/test_app.rs`](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/src/test_app.rs)
tests exercise LSP notification handling with injected values. These tests are
useful evidence about Ki's client code, but they are not Recite tests and do
not establish an installed Ki plus `recite-lsp` run.

## Configuration and plugin boundary

Language configuration names the file extensions and LSP language ID, then
provides an `lsp_command` with a command, arguments, optional initialization
options, and optional environment variables. The configuration model is in
[`shared/src/language.rs`](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/shared/src/language.rs).

Ki reads global configuration and a workspace `.ki/config.json`; workspace
configuration overrides global configuration. Its custom scripts are a
separate boundary: a script reads editor-state JSON on stdin and returns Ki
dispatches on stdout. That scripting interface does not register an LSP
language. See the upstream [`configuration.mdx`](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/docs/docs/configuration.mdx).

## What could be probed later

An external probe could add a temporary `recite` language entry to an isolated
Ki configuration: associate `.recite` files with language ID `recite`, and
point `lsp_command` at the already-built `recite-lsp` executable. This keeps
the Recite server and semantic authority outside Ki; no Ki plugin or source
patch is needed for the first probe.

Reopen this evaluation when one of these conditions is true:

1. an upstream change alters Ki's version, language configuration shape, or LSP
   process boundary;
2. an available Ki executable **and** an autonomous, isolated, fail-closed
   observation harness (or equivalent) are available for probing Recite; or
3. a maintainer explicitly schedules a Ki probe.

Acceptance evidence for that later probe should record the Ki revision and
version, `recite-lsp` executable digest, exact isolated configuration and
command arguments, then show that the actual Ki process sends `initialize` and
`textDocument/didOpen`, receives the expected Recite diagnostic code, severity,
and range for the malformed fixture, and completes at least one supported
semantic request such as completion or hover on the valid fixture. The record
must also show a clean Ki/server shutdown with no leaked probe process. Until
that evidence exists, source capability inspection is not an integration
claim.

## Current Recite status

This checkout contains no Ki configuration, extension, or Ki-specific tests.
Recite has no Ki support and no installed-host evidence. This note does not
change v1 scope.

## Sources

- [Ki repository](https://github.com/ki-editor/ki-editor)
- [Pinned README](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/README.md)
- [Pinned installation status](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/docs/docs/installation.md)
- [Pinned MPL-2.0 licence](https://github.com/ki-editor/ki-editor/blob/b3c7c4bbb79fd60c7d0dce845ea83740bceeebcd/LICENSE)
