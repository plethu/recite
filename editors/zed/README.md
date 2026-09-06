# Recite for Zed

This is the first Zed extension slice for Recite. It registers `.recite`
files, reuses the pinned Recite Tree-sitter grammar, projects the shared
highlights query, and starts a separately installed `recite-lsp` through Zed's
native LSP host.

The grammar is loaded from the Recite repository at commit
`209ea23195f674a18be0b8f87e037273fb3296bd`. The checked-in
`languages/recite/highlights.scm` is an exact projection of
`editors/recite-tree-sitter/queries/highlights.scm`; it adds no semantic
validation. Stable IDs, references, diagnostics, project discovery, and
configuration remain owned by Recite's parser/compiler/LSP and `recite-config`.

## Local source development

This is a source/development installation path, not a published gallery
installation. From a Recite checkout, open Zed's command palette and run
`zed: install dev extension` (or use Extensions → Install Dev Extension), then
select the repository's `editors/zed` directory. Zed builds the extension and
its pinned grammar from that directory. Rust with the `wasm32-wasip2` target
and the grammar build prerequisites must be available to the host; the
repository gate uses host checks when that target is unavailable.

The installed Linux host path was exercised on 2026-09-06 with Arch Zed
1.18.1 under a private headless Cage/WLR compositor. The probe installed and
rendered this development extension, started the configured `recite-lsp`, and
sent real keyboard-driven diagnostic, completion, hover, definition,
references, prepare-rename, task, and shutdown actions. It never used the
caller's display or desktop.

That host run received a real `textDocument/codeAction` request for a missing
ID, but Recite returned an empty result and no edit was applied. Code actions
therefore remain an explicit host limitation; the probe fails closed if a
future result is non-empty until its evidence and documentation are reviewed.
The run reached `textDocument/prepareRename`, but did not apply a replacement
name or capture a `textDocument/rename` edit. The shared LSP tests still prove
the canonical code-action edit and rename semantics outside this host lane.

macOS and Windows host smoke, gallery publication, and gallery installation
remain residuals.

See Zed's [extension development guide](https://zed.dev/docs/extensions/developing-extensions)
for the host-side development-extension workflow.

## Language server

Install `recite-lsp` separately and make it available on PATH. The extension
does not download, bundle, or start a network service. Zed supplies the LSP
root URI and workspace folders. Configure the binary only when the default
PATH lookup is not suitable:

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

Configured arguments and environment variables are passed through in stable
key order. The extension does not duplicate project-root or configuration
discovery. If no configured path exists and `recite-lsp` is not on PATH, Zed
receives an actionable error naming both installation and configuration
options.

The LSP binary override applies only to the language server. It does not set
the executable used by static tasks: `recite` must separately be available on
the task process PATH (or be wrapped by an explicit project task).

## Static tasks

The language package provides only tasks whose inputs can be derived from Zed
without guessing project semantics:

Compile output is explicit but not non-destructive: invoking the task may
replace `$ZED_DIRNAME/$ZED_STEM.recitec`.

Each task declares `save: "current"`, so launching validate, extract, compile,
or watch saves the current buffer without saving unrelated open buffers.
Subsequent watch rebuilds still depend on the editor's normal save events.

- `validate` and `extract` use the current `$ZED_FILE`;
- `compile` writes to the explicit sibling path
  `$ZED_DIRNAME/$ZED_STEM.recitec`. This is an explicit, user-invoked output
  location and may replace an existing generated asset; and
- `watch` uses `$ZED_WORKTREE_ROOT`.

Every task opts into `--output-format structured`. Zed's task terminal does not
parse those records into diagnostics, and this extension does not add a human
output matcher or a second diagnostic controller; LSP diagnostics remain the
diagnostic authority. `run` and `trace` tasks are intentionally absent because
their required compiled asset, block, and fixture cannot be safely inferred
from the current buffer. Add an explicit project task when those inputs are
known.

Zed owns task process lifecycle. These static tasks do not implement a fake
stdin cancellation transport or claim parsed watch recovery; stopping a task
uses the host's normal terminal/process controls. `recite` remains the owner of
structured watch records. The isolated Linux probe observed validation failure,
watch termination through the task terminal's Ctrl-C action, and clean private
process shutdown.

The isolated probe demonstrates keyboard reachability for the tested actions,
not complete Zed accessibility conformance. Screen-reader, focus,
high-contrast, and non-colour behavior remain host/LSP surfaces and are not
claimed here. The package adds no color protocol or terminal-color parser.

## Evidence and limits

`scripts/check-zed.sh` checks the manifest, package inventory, grammar pin and
query drift, task argv contract, launcher unit tests, and a real `recite-lsp`
stdio parity test. `scripts/check-zed-host.sh` exercises the installed Linux
development-extension path in isolated Cage/WLR state. Its transport log
asserts the real Zed requests and canonical diagnostics/navigation results; the
current host's code-action response is explicitly recorded as empty and
unsupported, and prepare-rename is the furthest rename boundary proved.
Task-terminal structured-record parsing, a native watch-cancellation API,
screen-reader/high-contrast behavior, macOS/Windows host smoke, gallery
publication, and gallery installation remain residuals.

The extension is dual-licensed under MIT OR Apache-2.0. See
`LICENSE-MIT` and `LICENSE-APACHE`.
