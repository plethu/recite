# Installed VS Code host probe

`host-probe.cjs` is loaded by the real VS Code or VSCodium extension-test
runner. The surrounding `scripts/check-vscode-host.sh` harness installs the
local deterministic VSIX through the host's extension API, then starts a
second host process with that VSIX in an isolated temporary profile.

The second process opens the canonical valid and invalid `.recite` fixtures,
configures the actual `recite-lsp` and `recite` binaries, and asserts host API
observations for activation, language registration, diagnostics, completion,
hover, same- and cross-file navigation, deterministic host-ordered references, UTF-16
positions, malformed/incomplete overlay recovery, stable-ID code actions,
guarded explicit rename preconditions, structured command success/failure
including run/trace, and watch cancellation. The host process is then
required to exit with its entire captured Cage process group empty.
The probe writes only a small JSON result file under the harness temporary
directory. Assertions deliberately fail the host test when an expected
activation, diagnostic, command, or shutdown observation is absent.
Set `RECITE_HOST_TMPDIR` to an ignored build directory when `/tmp` does not
have enough space for a full host profile; `/tmp` remains the default.

This is host API automation, not keyboard or visual accessibility evidence.
The probe does not access the user's display, desktop, profiles, extensions,
Marketplace, or Open VSX. Keyboard-only navigation and visual/a11y checks
remain manual platform follow-up until a portable isolated GUI automation
boundary can make those observations repeatably.
