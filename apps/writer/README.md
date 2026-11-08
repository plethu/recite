# Recite writer

The native application for editing Recite dialogue, built on Xilem. It is a preview. Script reads
and edits a beat, Map shows how beats branch and return, Localise edits PO catalogues, and preview
plays dialogue without running game effects. Source editing goes to your own editor.

From the repository root:

```sh
mise exec -- just writer run                      # project welcome screen
mise exec -- just writer run --project /path/to/project --document dialogue/scene.recite
mise exec -- just writer run --examples           # temporary examples; edits are not saved
mise exec -- just writer run --design-system      # control specimen; opens no project
mise exec -- just writer run --project /path/to/project --route 'recite://writer/write?beat=start'
mise exec -- just writer check
```

`--editor nvim` and repeated `--editor-arg` choose the external editor; without them Writer uses the
system's preferred application. Arguments are passed without a shell.

## Requirements

Builds need the platform graphics and window-system libraries and pkg-config. The workspace has its
own lockfile for the pinned toolkit forks. Creating a PO catalogue needs GNU gettext (`msginit` on
`PATH`), and so does the full test gate.

Writer bundles Latin, Cyrillic and Greek faces; other scripts use system fonts (on Linux, the Noto
families listed in [`assets/fonts/FONTS.md`](assets/fonts/FONTS.md)). Right-to-left text is shaped
within a line, but paragraphs are laid out left to right.

## Crates

- `crates/authoring` (`recite-writer-model`): source-preserving documents, edits and preview.
- `crates/session` (`recite-writer-session`): files, saves, recovery and configuration.
- `crates/app` (`recite-writer-app`): Writer state, workflows and policy, with no GUI toolkit.
- `crates/xilem` (`recite-writer`): the native Xilem frontend that renders that state.

Put behaviour that does not need a widget in `crates/app`; the frontend should only present it and
adapt native input. See [spec §15.5][strategy].

[strategy]: ../../docs/spec/tooling.md#155-native-gui-strategy-and-accessibility-proof

[Packaging](packaging.md) covers native packages, Flatpak and desktop links. Physical-host
acceptance is tracked in #237.
