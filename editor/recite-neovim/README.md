# Recite for Neovim

This directory is a small, plugin-manager-neutral Neovim integration for
Recite. It registers the `.recite` filetype, starts the shared
`recite-lsp` server, and registers the syntax-only Tree-sitter grammar from
[`../recite-tree-sitter`](../recite-tree-sitter/). It does not parse Recite,
validate a project, or provide a second compiler in Lua.

The integration uses Neovim's native Lua and LSP APIs. A plugin manager is
optional; the directory can be placed on `runtimepath` directly. The minimum
supported Neovim is 0.10.4 (the checked-in smoke lane currently runs 0.12.5).

## Install from a checkout

Add the integration to `runtimepath` from `init.lua`:

```lua
vim.opt.rtp:prepend("/absolute/path/to/recite/editor/recite-neovim")
```

The package's `plugin/recite.lua` calls `require("recite").setup()` with
defaults. For deterministic pre-load configuration, set
`vim.g.recite_options` before the package enters `runtimepath`; this works with
direct runtimepath use and lazy/packer-style managers:

```lua
vim.g.recite_options = {
  lsp = { cmd = { "/absolute/path/to/recite-lsp" } },
}
vim.opt.rtp:prepend("/absolute/path/to/recite/editor/recite-neovim")
```

It is also safe to call `setup` yourself after a manager loads the package:

```lua
require("recite").setup({
  lsp = {
    cmd = { "/absolute/path/to/recite-lsp" },
  },
})
```

The default command is `{ "recite-lsp" }`, resolved through Neovim's `PATH`.
For a checkout-local development binary, use
`/absolute/path/to/recite/target/debug/recite-lsp` or install it with
`cargo install --path crates/recite-lsp`. Do not put a project-relative path in
the configuration: an absolute command makes which server is running
inspectable and deterministic.

No default keymaps are installed. Neovim's normal LSP actions remain available
through its built-in commands and APIs. An `on_attach` callback and explicit
`capabilities` can be supplied under `lsp` when a user's configuration needs
them:

```lua
require("recite").setup({
  lsp = {
    on_attach = function(client, bufnr)
      -- Add personal mappings here; Recite does not impose a keymap.
    end,
  },
})
```

`lsp.root_dir` may be a fixed absolute path or a function receiving a buffer
number. With the defaults, the integration walks upward for the nearest
`recite.project.toml`; if there is no manifest, the buffer's containing
directory is used. This follows the shared discovery boundary: a nearer
manifest wins, while a manifest's `discovery.source_roots` and `excludes` stay
owned by `recite-config` and are not reimplemented in Lua.

The language server separately loads the shared user configuration. It honors
`$RECITE_CONFIG` first and then the platform configuration location described
in the [production spec](../../docs/recite-production-spec.md#137-play). The
Neovim integration does not read, merge, or write that file. Project semantics
belong in `recite.project.toml`, not in Neovim settings. An explicit schema
manifest can be passed to the server through `lsp.init_options` when needed:

```lua
require("recite").setup({
  lsp = {
    init_options = { schemaManifest = "/absolute/path/to/generated_manifest.json" },
  },
})
```

The server owns resolution of that path and all schema, localisation,
diagnostic, completion, navigation, rename, and code-action semantics.
Completion, hover, definition, references, and server-supported edits are
requested from that server without Lua-side semantic fallbacks. Rename and
code-action responses are inspected as structured workspace edits; a refused
operation is left unapplied. Re-running `setup` with changed LSP-owned options
stops and reattaches Recite clients while retaining caller-supplied
`capabilities`, `init_options`, `settings`, and `on_exit`. Unexpected exits are
retried for still-open Recite buffers with a bounded backoff; intentional
`require("recite").stop(client_id)` calls are not restarted.

## Tree-sitter highlighting

The checked-in grammar is source code plus generated parser sources. Neovim
needs a platform-specific dynamic parser library in `parser/`. Build it from
the grammar after installing the pinned Tree-sitter CLI:

```sh
cd /absolute/path/to/recite
mkdir -p editor/recite-neovim/parser
tree-sitter build editor/recite-tree-sitter \
  --output editor/recite-neovim/parser/recite.so
```

Use `recite.dll` on Windows. On macOS, `recite.so` is the usual Neovim
runtimepath name for this parser. The generated library is ignored by Git and
must be rebuilt for the host where Neovim runs. `mise run check-tree-sitter`
checks grammar generation, captures, recovery, and canonical fixture coverage.
`scripts/check-neovim.sh` builds and loads the ABI14 parser in Neovim and is the
authoritative integration gate; it does not install the Neovim parser library.

Highlighting starts automatically for `recite` buffers when the parser is
available. Without it, the filetype and LSP still work. The grammar only
provides tolerant visual captures; parser/compiler/LSP diagnostics remain the
authority for malformed syntax, IDs, references, schema, conditions, effects,
markup, and match exhaustiveness.

## Authoring commands

These commands operate on the project or source paths and are intentionally
ordinary CLI commands. They do not depend on Neovim or a plugin manager:

```sh
recite validate /path/to/project
recite extract /path/to/project -o /path/to/project/locales/recite.pot
recite watch /path/to/project
recite play /path/to/project/build/dialogue.recitec \
  --block which_way --ui plain
```

`recite watch` is the current human-oriented rebuild stream. Versioned
structured command and watch integration, process lifecycle, cancellation, and
editor task presentation belong to #53; this setup does not scrape CLI prose
or pretend that a watch process is an LSP feature. The `play --ui plain` form
is the predictable terminal preview path and is also suitable for screen
readers and pipes.

## Health and troubleshooting

Run `:checkhealth recite` to inspect filetype registration, the configured
`recite-lsp` executable, the Tree-sitter query, and the parser library.

- If `:set filetype?` is not `recite` for a `.recite` file, make sure the
  `editor/recite-neovim` directory is on `runtimepath` and run `:filetype on`.
- If the LSP is not attached, check `:checkhealth recite`, then set `lsp.cmd`
  to an absolute executable path. The server must be executable by Neovim and
  speak LSP over stdio.
- If highlighting is absent but the LSP works, build `parser/recite.so` from
  the Tree-sitter grammar and ensure the package directory is on runtimepath.
- If the server reports diagnostics for the wrong project, inspect the nearest
  `recite.project.toml` and either open the intended project root or provide a
  fixed `lsp.root_dir`. Do not duplicate discovery rules in Lua.
- If schema diagnostics are missing, configure the manifest's schema or pass
  `lsp.init_options.schemaManifest`; schema loading failures remain visible as
  server diagnostics.

The syntax query in `queries/recite/highlights.scm` is kept byte-for-byte equal
to the host-neutral query in `../recite-tree-sitter/queries/highlights.scm` by
the Neovim integration check. Changes to grammar captures belong to #98 and
must be reflected in both projections.
