# Recite Unity Adapter

This package is the Unity GameObject/OO MVP for Recite. It targets Unity
2022.3 LTS and wraps the `recite-ffi` native library through P/Invoke.

## Native Plugin

Build `crates/recite-ffi` for the Unity editor/player platform and place the
native library under the Unity project's plugin folder, for example:

```text
Assets/Plugins/recite_ffi.dll
Assets/Plugins/librecite_ffi.so
Assets/Plugins/librecite_ffi.dylib
```

The managed declarations in `Runtime/Native/ReciteNativeBridge.cs` are checked
against the generated `include/recite.h` header by
`scripts/check-unity-adapter.sh`.

## Runtime Flow

Use `ReciteDialogueService` as the semantic owner for one active Recite session:

1. Construct a `ReciteDialogueAsset` from compiled Recite bytes.
2. Register pure C# condition handlers with `RegisterCondition`, or use the
   additive `RegisterTypedCondition` API when identifier and string arguments
   must remain distinct. Typed callbacks receive `ReciteConditionArgument`
   values for all five ABI kinds: identifier, string, integer, float, and
   boolean. The original `IReadOnlyList<object>` API remains available.
3. Call `Start` with an optional start block and locale.
4. Present the returned `ReciteOutputBatch` values in the game's UI.
5. Call `SelectChoice` with the stable choice ID from the current prompt.
6. For blocking effects, perform game-side work and call `AcknowledgeEffect`
   with the exact effect request ID.
7. Store `Snapshot()` bytes beside game save data, and pass them back to
   `Restore` with the same compiled asset identity.

`ReciteDialogueRunner` is a `MonoBehaviour` facade for inspector-wired scenes.
It emits structured `ReciteOutput` and `ReciteAdapterException` values through
UnityEvents; it does not implement traversal itself.

## Sample

Import the `Basic Dialogue` sample from Package Manager. Add
`ReciteDialogueRunner` and `BasicDialogueDriver` to a scene, assign a compiled
Recite `TextAsset` to the runner, and wire the runner's output/error UnityEvents
to the driver methods. The sample driver demonstrates start, choice selection,
blocking-effect acknowledgement, snapshot, and restore calls.

## Checks

Repository checks:

```bash
scripts/check-unity-adapter.sh
scripts/check-project-gates.sh
```

Manual Unity checks for this MVP:

- Package imports in Unity 2022.3 LTS.
- A scene with `ReciteDialogueRunner` loads a compiled Recite asset.
- Start/select/blocking-effect acknowledge emits ordered structured output.
- Registered C# conditions are called synchronously and missing handlers report
  `ReciteStatus.MissingConditionHandler`.
- `Snapshot` and `Restore` preserve a pending prompt or pending blocking effect.

Editor import/refresh tooling, schema export, native binary distribution, DOTS,
and package-manager release automation are tracked separately from this runtime
MVP.
