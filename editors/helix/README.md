# Recite for Helix

This directory is a small, source-based Helix language configuration. It
registers `.recite` files, supplies the syntax-only Recite Tree-sitter grammar,
and starts the shared `recite-lsp` server over stdio. It does not parse,
validate, or compile Recite in Helix.

## Install from a checkout

The shipped `languages.toml` is a standalone configuration: its
`use-grammars = { only = ["recite"] }` setting scopes grammar fetch/build to
the Recite entry for this isolated file. Do not copy it over an existing
Helix configuration, because that would discard unrelated language servers,
grammars, and user policy.

For an absent user configuration, this guarded command installs the language
file and Recite-specific query:

```sh
config_dir="$HOME/.config/helix"
config_file="$config_dir/languages.toml"
query_file="$config_dir/runtime/queries/recite/highlights.scm"
if [ -e "$config_file" ] || [ -L "$config_file" ]; then
  echo "Refusing to overwrite $config_file; merge the Recite entries manually." >&2
  exit 1
fi
if [ -e "$query_file" ] || [ -L "$query_file" ]; then
  echo "Refusing to overwrite $query_file; inspect the existing Recite query first." >&2
  exit 1
fi
mkdir -p "$(dirname "$query_file")"
cp /absolute/path/to/recite/editors/helix/languages.toml "$config_file"
cp /absolute/path/to/recite/editors/helix/runtime/queries/recite/highlights.scm "$query_file"
```

If `languages.toml` already exists, stop and merge these three tables into it:
`[language-server.recite-lsp]`, the `[[language]]` entry, and the `[[grammar]]`
entry. Preserve the existing `use-grammars` policy; if it uses an `only` list,
add `recite` to that list rather than replacing the list with only `recite`.
The query is Recite-specific, but inspect an existing destination before
replacing it. For a project-local setup, make the same guarded merge in
`.helix/languages.toml` and set `HELIX_RUNTIME` to a runtime directory
containing the query.

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
