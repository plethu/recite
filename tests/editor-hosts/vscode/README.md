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

The keyboard phase starts a second isolated host under private Cage/WLR and
sends real Wayland events with `wtype`: it opens `scratch/invalid.recite`
through Quick Open (`Ctrl+P`, path, Return), and
asserts the active URI, `recite` language, and extension activation. It then
opens Problems, returns focus, focuses the primary editor with `Ctrl+1`, and
presses F8 to activate the next diagnostic. The probe asserts that the
selection is within the `RECITE_PARSE011` range. It then invokes a disposable
binding for the supported rename command and starts watch. It sends `Ctrl+Alt+Shift+Q` through a
disposable binding for the supported Stop watch command (the product has no
default shortcut), then observes the edit, diagnostic
code/severity/location, and captured-group CLI exit through marker files and
process inspection. The temporary bindings are test-harness wiring only; they
do not claim product keybindings. This proves the scripted host path, not
arbitrary focus traversal, visual rendering, screen-reader/high-contrast
output, or all desktop accessibility behavior, which remain manual platform
follow-up. VS Code and VSCodium use distinct one-character `v` and `c`
profiles to stay within Linux's Unix-socket path limit; each host must write a
fresh host-product activation marker before its keyboard phase is accepted.
The probe does not access the
user's display, desktop, profiles,
extensions, Marketplace, or Open VSX.
