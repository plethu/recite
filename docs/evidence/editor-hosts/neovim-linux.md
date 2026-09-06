# Neovim installed-host evidence (Linux x86_64)

Run date: 2026-09-06

This is the Neovim portion of the Milestone 4 host-evidence work. It records
the actual Linux x86_64 Neovim binaries used by the automated host lane and
does not generalise the result to macOS, Windows, another architecture, or a
GUI terminal session.

## Hosts

| Host | Version | Binary/source | Result |
| --- | --- | --- | --- |
| Neovim | 0.10.4 | Official `nvim-linux-x86_64.tar.gz`, downloaded ephemerally under `/tmp` from [`neovim/neovim` v0.10.4](https://github.com/neovim/neovim/releases/download/v0.10.4/nvim-linux-x86_64.tar.gz); SHA-256 `95aaa8e89473f5421114f2787c13ae0ec6e11ebbd1a13a1bd6fcf63420f8073f` | pass |
| Neovim | 0.12.5 | Current pinned Linux x86_64 host selected through `NVIM` (the repository toolchain pins 0.12.5) | pass |

The temporary 0.10.4 archive and extracted directory are removed by the
harness trap. The harness checks the reported Neovim version and the ELF
architecture before running the integration lane; it does not install or
change a global Neovim toolchain.

## Reproduction

From the repository root:

```sh
scripts/check-neovim-host.sh
```

This command runs `scripts/check-neovim.sh --host-evidence` once for each
version. Each lane builds and uses the real `recite-lsp` and `recite` binaries,
loads the checked-in runtimepath package and ABI14 parser in an isolated XDG
environment, and checks the Neovim process group after exit. A supplied
`NVIM_0104_BIN` may replace the download for an offline rerun, but the version,
architecture, and executable checks still apply.

The ordinary current-host gate also passes independently:

```sh
scripts/check-neovim.sh
```

## Automated host workflow

`tests/editor-hosts/neovim/keyboard-workflow.lua` drives Neovim's actual
command-line input path with `nvim_feedkeys`; it does not call a plugin test
double or install test-only mappings. Recite intentionally installs no default
keymaps, so the portable keyboard entry point is the normal `:` command line.
The sequence exercised in both hosts is:

```text
:edit <malformed .recite path><Enter>
:lua vim.diagnostic.goto_next()<Enter>
:edit <valid .recite path><Enter>
:ReciteValidate <valid .recite path><Enter>
:ReciteCompile<Enter>
:ReciteRun<Enter>                         # expected input failure
:ReciteWatchStart <project root><Enter>
:ReciteWatchStop<Enter>
```

The assertions prove that:

- editing a `.recite` path activates the `recite` filetype and attaches the
  real `recite-lsp` process;
- malformed-source diagnostics are present, navigable through Neovim's
  diagnostic command, positioned on the reported line, and have textual
  messages;
- `ReciteValidate` and `ReciteCompile` are reachable through the host command
  line and produce a structured command result (compile also creates the
  derived output in the isolated project);
- the invalid `ReciteRun` invocation presents a non-empty textual failure with
  Neovim error severity; and
- a real CLI watch child starts, publishes textual status, accepts the
  keyboard-reachable stop command, and retires cleanly.

The shell harness launches each Neovim run in its own process group. After
`VimLeavePre` and normal exit it checks that no process remains in that group,
covering the `recite-lsp`, finite CLI, and watch-child cleanup boundary.

## Evidence boundary

This is automated installed-binary/headless-host evidence for Linux x86_64. It
is not a claim that a desktop GUI, terminal emulator, colour theme, screen
reader, IME, zoom/text scaling, or pointer interaction was manually tested.
It does not establish macOS or Windows behavior, another Neovim version, a
Neovim distribution package, or broader Milestone 5 accessibility proof.
