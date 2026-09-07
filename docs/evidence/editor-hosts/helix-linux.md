# Helix Linux package evidence

This record covers the source Helix projection only. It does not promote Helix
to the editor parity contract or claim installed-host keyboard evidence.

## Observed local host

| Item | Evidence |
| --- | --- |
| Binary | `/usr/bin/helix` |
| Version | `helix 25.07.1 (4746c276)` |
| Platform | Linux x86_64 |
| Recite language entry before this integration | absent from the installed Helix language set |
| Ki binary/config | not installed/present |

## Automated package check

Run from the Recite repository:

```sh
scripts/check-helix.sh
```

The check asserts that the language configuration is valid TOML and contains
the `.recite` file type, `recite-lsp` stdio command, explicit
`recite.project.toml` root marker, and pinned nested Tree-sitter grammar. It
also requires the Helix query to remain byte-for-byte equal to
`editors/recite-tree-sitter/queries/highlights.scm`.

When Helix is installed, the check copies the configuration and query into a
temporary `HELIX_RUNTIME`/XDG configuration and runs `helix --health recite`.
This proves that Helix discovers the isolated language configuration and query
entries; it does not prove that the parser or grammar is built and loaded, that
`recite-lsp` is installed, or that semantic requests cross a live editor host.

The shared semantic protocol remains covered independently by:

```sh
cargo test --locked -p recite-lsp --test editor_parity
```

## Host boundary

Helix 25.07.1 exposes no headless mode in its command-line interface. This
slice therefore has no automated keyboard, active-buffer, diagnostic-panel,
rename-application, process-shutdown, or installed-host LSP evidence. A future
host lane must use a private PTY/compositor harness with fail-closed semantic
assertions before claiming those behaviors. The Helix root marker also follows
Helix's topmost-root selection; nearest `recite.project.toml` discovery is not
claimed until it has a dedicated host test.

Tasks, structured CLI diagnostics, watch lifecycle, cancellation, accessibility,
macOS/Windows, and distribution remain out of scope.
