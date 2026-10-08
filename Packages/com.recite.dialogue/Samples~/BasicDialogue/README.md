# Recite Unity Basic Dialogue

This sample uses an imported `ReciteCompiledAsset` to demonstrate conditions, line and prompt
output, stable choice selection, blocking effects, and opaque session snapshots.

1. Follow the package's
   [installation guide](https://github.com/plethu/recite/blob/main/Packages/com.recite.dialogue/README.md#install-and-build).
   The Linux x86_64 bundle includes its native plugin under `Runtime/Plugins/x86_64`; a source-tree
   install needs the matching library at the same location inside the package.
2. Import **Basic Dialogue** from Package Manager. Run **Tools > Recite > Migrate Legacy Compiled
   Asset References** to bind the sample scene to the imported `.recitec` object. The
   [upgrade guide](https://github.com/plethu/recite/blob/main/Packages/com.recite.dialogue/README.md#upgrading-old-scenes)
   explains the migration and backups.
3. Open `BasicDialogue.unity` and enter Play Mode. `BasicDialogueDriver` registers the condition,
   starts dialogue and logs structured output and errors.

Save game state alongside the opaque Recite snapshot. Restoring a pending blocking effect can emit
the same request ID again. The sample checks its existing `hasRelayKey` gameplay state before
applying `grant_item`, then acknowledges the effect. A real save system must restore that gameplay
state too.

The package's
[verification guide](https://github.com/plethu/recite/blob/main/Packages/com.recite.dialogue/README.md#verification)
owns current managed, Editor and player evidence and platform limits.
