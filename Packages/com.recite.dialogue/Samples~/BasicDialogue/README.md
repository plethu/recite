# Recite Unity Basic Dialogue

This sample demonstrates the runtime MVP flow:

- load a compiled Recite `TextAsset`;
- start a session and emit line/prompt output;
- select a stable choice ID;
- register C# conditions through `ReciteDialogueService`;
- acknowledge blocking effects;
- save and restore an opaque Recite session snapshot.

Open `BasicDialogue.unity` after importing the package sample. Assign the
generated native plugin (`recite_ffi`) under `Assets/Plugins/` for the current
platform before entering Play Mode.
