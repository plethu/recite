# Recite for Neovim

This directory is a small, plugin-manager-neutral Neovim integration for Recite. It registers the
`.recite` filetype, starts the shared `recite-lsp` server, and registers the syntax-only Tree-sitter
grammar from [`../recite-tree-sitter`](../recite-tree-sitter/). It does not parse Recite, validate a
project, or provide a second compiler in Lua.

The integration uses Neovim's native Lua and LSP APIs. A plugin manager is optional; the directory
can be placed on `runtimepath` directly. The minimum compatibility target is Neovim 0.10.4. The
installed-host evidence lane runs both 0.10.4 and the pinned current host, 0.12.5, on Linux x86_64.

## Install from a checkout

Add the integration to `runtimepath` from `init.lua`:

```lua
vim.opt.rtp:prepend("/absolute/path/to/recite/editors/recite-neovim")
```

The package's `plugin/recite.lua` calls `require("recite").setup()` with defaults. For deterministic
pre-load configuration, set `vim.g.recite_options` before the package enters `runtimepath`; this
works with direct runtimepath use and lazy/packer-style managers:

```lua
vim.g.recite_options = {
  lsp = { cmd = { "/absolute/path/to/recite-lsp" } },
}
vim.opt.rtp:prepend("/absolute/path/to/recite/editors/recite-neovim")
```

It is also safe to call `setup` yourself after a manager loads the package:

```lua
require("recite").setup({
  lsp = {
    cmd = { "/absolute/path/to/recite-lsp" },
  },
})
```

The default command is `{ "recite-lsp" }`, resolved through Neovim's `PATH`. For a checkout-local
development binary, use `/absolute/path/to/recite/target/debug/recite-lsp` or install it with `cargo
install --path crates/recite-lsp`. Do not put a project-relative path in the configuration: an
absolute command makes which server is running inspectable and deterministic.

No default keymaps are installed. Neovim's normal LSP actions remain available through its built-in
commands and APIs. An `on_attach` callback and explicit `capabilities` can be supplied under `lsp`
when a user's configuration needs them:

```lua
require("recite").setup({
  lsp = {
    on_attach = function(client, bufnr)
      -- Add personal mappings here; Recite does not impose a keymap.
    end,
  },
})
```

`lsp.root_dir` may be a fixed absolute path or a function receiving a buffer number. With the
defaults, the integration walks upward for the nearest `recite.project.toml`; if there is no
manifest, the buffer's containing directory is used. This follows the shared discovery boundary: a
nearer manifest wins, while a manifest's `discovery.source_roots` and `excludes` stay owned by
`recite-config` and are not reimplemented in Lua.

The language server separately loads the shared user configuration. It honors `$RECITE_CONFIG` first
and then the platform configuration location described in the
[production spec](../../docs/recite-production-spec.md#137-play). The Neovim integration does not
read, merge, or write that file. Project semantics belong in `recite.project.toml`, not in Neovim
settings. An explicit schema manifest can be passed to the server through `lsp.init_options` when
needed:

```lua
require("recite").setup({
  lsp = {
    init_options = { schemaManifest = "/absolute/path/to/generated_manifest.json" },
  },
})
```

The server owns schema resolution and semantic answers. Rename and code actions keep version
preconditions and are left unapplied when refused. Re-running `setup` with changed options
reattaches clients while preserving supplied callbacks and capabilities.

`lsp.flags.debounce_text_changes` defaults to 50 ms; override it to coalesce more typing. Unexpected
server exits are retried with bounded backoff for still-open Recite buffers. Exhausted recovery is
reported. An intentional `require("recite").stop(client_id)` is not restarted, and `autostart =
false` remains respected after reconfiguration. Each distinct configuration retains its own recovery
state; callback errors are reported without preventing cleanup.

## Tree-sitter highlighting

The checked-in grammar is source code plus generated parser sources. Neovim needs a
platform-specific dynamic parser library in `parser/`. Build it from the grammar after installing
the pinned Tree-sitter CLI:

```sh
cd /absolute/path/to/recite
mkdir -p editors/recite-neovim/parser
tree-sitter build editors/recite-tree-sitter \
  --output editors/recite-neovim/parser/recite.so
```

Use `recite.dll` on Windows. On macOS, `recite.so` is the usual Neovim runtimepath name for this
parser. The generated library is ignored by Git and must be rebuilt for the host where Neovim runs.
`mise run check-tree-sitter` checks grammar generation, captures, recovery, and canonical fixture
coverage. `scripts/check-neovim.sh` builds and loads the ABI14 parser in Neovim and is the
authoritative integration gate; it does not install the Neovim parser library.

Highlighting starts automatically for `recite` buffers when the parser is available. Without it, the
filetype and LSP still work. The grammar only provides tolerant visual captures; parser/compiler/LSP
diagnostics remain the authority for malformed syntax, IDs, references, schema, conditions, effects,
markup, and match exhaustiveness.

## Authoring commands

The integration exposes scriptable `require("recite")` functions and matching user commands for the
versioned structured CLI boundary:

```lua
local recite = require("recite")
recite.validate({ project_root = "/path/to/project", paths = { "/path/to/project/dialogue.recite" } })
recite.compile({ project_root = "/path/to/project", paths = { "/path/to/project/dialogue.recite" },
  output = "/path/to/project/build/dialogue.recitec" })
recite.extract({ project_root = "/path/to/project", paths = { "/path/to/project/dialogue.recite" } })
recite.run({ asset = "/path/to/project/build/dialogue.recitec", block = "which_way", fixture = "/path/to/project/runtime-fixture.toml" })
recite.trace({ asset = "/path/to/project/build/dialogue.recitec", block = "which_way", fixture = "/path/to/project/runtime-fixture.toml" })
```

The corresponding commands are `:ReciteValidate`, `:ReciteCompile`, `:ReciteExtract`, `:ReciteRun`,
`:ReciteTrace`, `:ReciteWatchStart`, and `:ReciteWatchStop`. Run and trace deliberately require
explicit asset, block, and fixture paths; Neovim does not guess runtime inputs. Compile derives
`build/dialogue.recitec` under the selected project when `output` is omitted, creates its missing
parent after input and CLI preflight, and reports that path because an existing generated file may
be replaced. Explicit output paths retain the CLI's directory semantics. Set `commands.binary` to an
absolute checkout-local executable when PATH discovery is not suitable.

The adapter passes argv directly to `vim.system`, uses the explicit project root as its working
directory, validates the version-1 finite and watch NDJSON records, and owns a separate child
lifecycle from the LSP client. CLI diagnostics use a `recite-cli` namespace and are only projected
for clean disk-backed buffers; unsaved or changed buffers retain LSP ownership. Watch owns one
child, uses the version-1 stdin cancel record, and bounds TERM/KILL recovery. Late records from
retired generations are ignored.

Use `recite play ASSET --block BLOCK --ui plain` for a terminal preview that accepts typed input.
These commands remain ordinary CLI operations; the integration consumes structured records rather
than scraping their human output.

## Health and troubleshooting

Run `:checkhealth recite` to inspect filetype registration, the configured `recite-lsp` executable,
the Tree-sitter query, and the parser library. The health module is installed at
`lua/recite/health.lua`, Neovim's standard runtimepath discovery location.

- If `:set filetype?` is not `recite` for a `.recite` file, make sure the `editors/recite-neovim`
  directory is on `runtimepath` and run `:filetype on`.
- If the LSP is not attached, check `:checkhealth recite`, then set `lsp.cmd` to an absolute
  executable path. The server must be executable by Neovim and speak LSP over stdio.
- If highlighting is absent but the LSP works, build `parser/recite.so` from the Tree-sitter grammar
  and ensure the package directory is on runtimepath.
- If the server reports diagnostics for the wrong project, inspect the nearest `recite.project.toml`
  and either open the intended project root or provide a fixed `lsp.root_dir`. Do not duplicate
  discovery rules in Lua.
- If schema diagnostics are missing, configure the manifest's schema or pass
  `lsp.init_options.schemaManifest`; schema loading failures remain visible as server diagnostics.

The syntax query in `queries/recite/highlights.scm` is kept byte-for-byte equal to the host-neutral
query in `../recite-tree-sitter/queries/highlights.scm` by the Neovim integration check. Change the
canonical query and regenerate its host projections together.
