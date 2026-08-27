# Recite Unity Basic Dialogue

This sample demonstrates the runtime MVP flow:

- load a compiled Recite `TextAsset`;
- start a session and emit line/prompt output;
- select a stable choice ID;
- register C# conditions through `ReciteDialogueService`;
- acknowledge blocking effects;
- save and restore an opaque Recite session snapshot.

When restoring while a blocking effect is pending, Recite may emit the same
effect request ID again. The sample reconciles the `grant_item` operation with
its existing gameplay state before acknowledging it, so the irreversible
operation is not blindly repeated.

Open `BasicDialogue.unity` after importing the package sample. Assign the
generated native plugin (`recite_ffi`) under `Assets/Plugins/` for the current
platform before entering Play Mode. Repository headless checks cover the
managed package contract; they do not provide a native Unity editor/player
integration test.
