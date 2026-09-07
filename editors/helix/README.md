# Recite for Helix

This directory is a small, source-based Helix language configuration. It
registers `.recite` files, supplies the syntax-only Recite Tree-sitter grammar,
and starts the shared `recite-lsp` server over stdio. It does not parse,
validate, or compile Recite in Helix.

## Install from a checkout

Copy the language configuration and query into the Helix user configuration
directory:

```sh
mkdir -p ~/.config/helix/runtime/queries/recite
cp /absolute/path/to/recite/editors/helix/languages.toml ~/.config/helix/languages.toml
cp /absolute/path/to/recite/editors/helix/runtime/queries/recite/highlights.scm \
  ~/.config/helix/runtime/queries/recite/highlights.scm
```

For a project-local setup, copy `languages.toml` to `.helix/languages.toml`
instead and set `HELIX_RUNTIME` to a runtime directory containing the query.

The configured command is `recite-lsp`, resolved through `PATH`. For a
checkout-local development binary, change `command` to an absolute path such
as `/absolute/path/to/recite/target/debug/recite-lsp`. The server reads the
shared Recite user/project configuration; Helix does not merge or reproduce
those semantics.

## Tree-sitter grammar

The grammar is fetched from the Recite repository at the pinned revision in
`languages.toml`, with `subpath = "editors/recite-tree-sitter"` because the
grammar lives inside the main repository. `use-grammars = { only = ["recite"] }`
keeps Helix's grammar commands scoped to this Recite entry. After installing
Helix's grammar build prerequisites, run from the configuration containing this
file:

```sh
hx --grammar fetch
hx --grammar build
```

The installed command may be named `helix` rather than `hx`; use the command
provided by the package. The query projection is syntax-only and tolerant of
partial buffers. Parser/compiler/LSP diagnostics remain authoritative for IDs,
references, schema, conditions, effects, markup, and recovery.

## Project roots and limits

`recite.project.toml` is a Helix LSP root marker. Helix chooses the topmost
matching marker between the file and the workspace root; that is not a claim
of Recite's nearest-manifest discovery semantics. Open the intended project
root and verify the resulting workspace before relying on nested projects.

This integration intentionally provides no task definitions, watch controller,
structured CLI diagnostic matcher, cancellation implementation, or default
keymaps. Use the ordinary `recite` CLI explicitly for compile, validate,
extract, run, trace, and watch operations.

## Evidence boundary

`../recite-tree-sitter` and the shared LSP fixture tests provide the package
and protocol evidence. `scripts/check-helix.sh` checks the package without
requiring a running editor. Installed-host keyboard evidence is not claimed:
Helix 25.07.1 has no headless mode in its command-line interface, and a
future host lane must add a private PTY/compositor harness with inspectable
assertions before it can be accepted.
