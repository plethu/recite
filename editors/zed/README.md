# Recite for Zed

The Recite Zed extension registers `.recite` files, reuses the pinned Recite Tree-sitter grammar,
projects the shared highlights query, and starts a separately installed `recite-lsp` through Zed's
native LSP host.

`extension.toml` owns the grammar revision; its highlight query is checked against the canonical
Tree-sitter query. Highlighting remains syntax-only.

## Local source development

This is a source/development installation path, not a published gallery installation. From a Recite
checkout, open Zed's command palette and run `zed: install dev extension` (or use Extensions →
Install Dev Extension), then select the repository's `editors/zed` directory. Zed builds the
extension and its pinned grammar from that directory. Rust with the `wasm32-wasip2` target and the
grammar build prerequisites must be available to the host; the repository gate uses host checks when
that target is unavailable.

Run `scripts/run-editor-host-check.sh zed` to exercise development-extension installation,
rendering, LSP features, versioned code actions and rename, static tasks, keyboard diagnostics, and
process cleanup in a private Linux compositor. The run writes a log and result under
`target/editor-host-evidence/`. The UTF-16 probe checks a request position after a non-BMP marker;
it does not establish response-range rendering behavior. macOS and Windows host smoke and gallery
installation remain unverified.

See Zed's [extension development guide](https://zed.dev/docs/extensions/developing-extensions) for
the host-side development-extension workflow.

## Language server

Install `recite-lsp` separately and make it available on PATH. The extension does not download,
bundle, or start a network service. Zed supplies the LSP root URI and workspace folders. Configure
the binary only when the default PATH lookup is not suitable:

```json
{
  "lsp": {
    "recite-lsp": {
      "binary": {
        "path": "/path/to/recite-lsp",
        "arguments": [],
        "env": {
          "RECITE_CONFIG": "/path/to/recite/config.toml"
        }
      }
    }
  }
}
```

Configured arguments and environment variables are passed through in stable key order. The extension
does not duplicate project-root or configuration discovery. If no configured path exists and
`recite-lsp` is not on PATH, Zed receives an actionable error naming both installation and
configuration options.

The LSP binary override applies only to the language server. It does not set the executable used by
static tasks: `recite` must separately be available on the task process PATH (or be wrapped by an
explicit project task).

## Static tasks

The language package provides only tasks whose inputs can be derived from Zed without guessing
project semantics:

Each task declares `save: "current"`, so launching validate, extract, compile, or watch saves the
current buffer without saving unrelated open buffers. Subsequent watch rebuilds still depend on the
editor's normal save events.

- `validate` and `extract` use the current `$ZED_FILE`;
- `compile` writes to the explicit sibling path `$ZED_DIRNAME/$ZED_STEM.recitec`. This is an
  explicit, user-invoked output location and may replace an existing generated asset; and
- `watch` uses `$ZED_WORKTREE_ROOT`.

Every task opts into `--output-format structured`. Zed's task terminal does not parse those records
into diagnostics, and this extension does not add a human output matcher or a second diagnostic
controller; LSP diagnostics remain the diagnostic authority. `run` and `trace` tasks are
intentionally absent because their required compiled asset, block, and fixture cannot be safely
inferred from the current buffer. Add an explicit project task when those inputs are known.

Zed owns terminal task lifecycle, including Ctrl-C. These tasks do not supply a structured watch
controller. The Linux probe exercises named keyboard actions and clean process stopping; it does not
establish complete accessibility, screen-reader or other-platform behavior.

## Evidence and limits

`scripts/check-zed.sh` checks the manifest, package inventory, grammar pin and query drift, task
argv contract, launcher unit tests, and a real `recite-lsp` stdio parity test.
`scripts/check-zed-host.sh` exercises the installed Linux development-extension path in isolated
Cage/WLR state. Package, LSP transport and installed-host assertions are distinct. The
[parity fixture](../../fixtures/editor-parity/contract.json) records supported observations and
remaining gaps, including task-record parsing, native cancellation and non-Linux hosts.

The extension is dual-licensed under MIT OR Apache-2.0. See `LICENSE-MIT` and `LICENSE-APACHE`.
