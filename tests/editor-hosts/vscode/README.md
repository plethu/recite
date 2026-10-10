# Installed VS Code host probe

Run `scripts/check-vscode-host.sh` to test the deterministic VSIX in an isolated VS Code or VSCodium
profile against the real Recite binaries. The harness owns host installation, process cleanup and
the temporary result files. Set `RECITE_HOST_TMPDIR` to an ignored build directory when `/tmp` lacks
space for a full host profile.

[`host-probe.cjs`](host-probe.cjs) owns the host API assertions. The keyboard phase runs under a
private Cage/WLR session with real Wayland input: it opens a fixture, navigates to a diagnostic,
invokes supported rename, and starts and stops watch. Disposable bindings used by the harness are
not product shortcuts. Each host must produce fresh activation and result evidence.

This verifies the scripted host path. Arbitrary focus traversal, visual rendering, screen-reader and
high-contrast output still need native acceptance. The harness uses no user display, profile,
extensions, Marketplace or Open VSX installation.
