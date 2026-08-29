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
3. Call `SetInterpolationValues` with typed `ReciteInterpolationValue` records
   before `Start` when the dialogue uses line or choice bindings. Values are
   copied at the native boundary and may be replaced between traversal calls.
4. Install a `ReciteLocaleCatalog` with `SetLocaleCatalog` when translated
   lines, choices, availability reasons, or presentation labels are wanted.
   Catalog entries are keyed by source ID/text and optional grammatical
   variant; missing entries deliberately fall back to authored source text.
   Call `Start` with an optional start block, locale, and variant.
5. Present the returned `ReciteOutputBatch` values in the game's UI.
6. Call `SelectChoice` with the stable choice ID from the current prompt.
7. For blocking effects, perform game-side work and call `AcknowledgeEffect`
   with the exact effect request ID.
8. Store `Snapshot()` bytes beside game save data, and pass them back to
   `Restore` with the same compiled asset identity and variant when needed. If the snapshot contains a
   pending blocking effect, restore may emit that effect again with the same
   request ID; reconcile the game-side operation idempotently before calling
   `AcknowledgeEffect`.

`ReciteDialogueRunner` is a `MonoBehaviour` facade for inspector-wired scenes.
It emits structured `ReciteOutput` and `ReciteAdapterException` values through
UnityEvents; it does not implement traversal itself.

## Localisation example

The catalog is an owned, deterministic input. Install a complete gettext rule
before its plural entries; the native Recite core validates every reachable arm
and selects the arm at lookup time:

```csharp
var catalog = new ReciteLocaleCatalog();
catalog.SetPluralRule("fr", "nplurals=2; plural=(n != 1);");
catalog.AddTranslation("fr", "greeting", "Hello {name}.", "Bonjour {name}.",
    ReciteLocaleTextDomain.Line, "formal");
catalog.AddPluralTranslation("fr", "letters", "One letter.", "{count} letters.",
    new[] { "Une lettre.", "{count} lettres." }, "formal");
catalog.AddChoiceTranslation("fr", "continue", "Continue", "Continuer");

var service = new ReciteDialogueService();
service.SetLocaleCatalog(catalog);
service.SetInterpolationValues(new[] {
    ReciteInterpolationValue.Integer("count", 2),
    ReciteInterpolationValue.String("name", "Ada")
});
var first = service.Start(asset, locale: "fr-CA", variant: "formal");
var snapshot = service.Snapshot();
service.End();
var resumed = service.Restore(asset, snapshot, variant: "formal");
```

Lookup uses explicit variant context, unqualified context, locale fallback,
and finally authored source text. Empty translations deliberately take that
source fallback. Catalogs, typed interpolation values, and the selected
variant are re-supplied on restore because they are host-owned inputs; the
runtime snapshot stores locale/session state only. Add/install operations
reject malformed rules, missing rules, wrong arm counts, conflicting entries,
and placeholder mismatches before traversal. Public strings reject embedded
NUL and unpaired UTF-16 surrogates.

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

The repository's headless package check builds and loads `librecite_ffi` and
exercises raw native session create/begin, locale callbacks, variant restore,
choice/acknowledgement traversal, and buffer ownership; it does not exercise
the managed service's `GCHandle` callback path or claim Unity editor/player or
IL2CPP integration.

Manual Unity checks for this MVP:

- Package imports in Unity 2022.3 LTS.
- A scene with `ReciteDialogueRunner` loads a compiled Recite asset.
- Start/select/blocking-effect acknowledge emits ordered structured output.
- Registered C# conditions are called synchronously and missing handlers report
  `ReciteStatus.MissingConditionHandler`.
- `Snapshot` and `Restore` preserve a pending prompt or pending blocking effect;
  a restored blocking request may be re-emitted with its original stable ID.

## Localisation boundary

`ReciteLocaleCatalog.SetPluralRule` takes only the declared gettext
`Plural-Forms` header. The shared native Recite contract validates the complete
header and supplies plural-arm selection; managed callers cannot replace that
authority with an arbitrary delegate. A plural entry must contain exactly the
validated `nplurals` arms, with empty arms reserved for source fallback.

Catalogs are cloned on install. Locale callback strings, plural attempt arrays,
and error strings remain owned by the service until the enclosing native
`Start`, `SelectChoice`, `AcknowledgeEffect`, or `Restore` call returns; only
then are they released (with `End`/`Dispose` also acting as cleanup for
rollback and direct callback tests). Embedded NULs and unpaired UTF-16
surrogates are rejected for all public block, locale, variant, choice, effect,
condition, and catalog strings. Managed callback exceptions are caught and
returned as localisation failures. Native C/C++ callbacks must enforce the
strict non-null, synchronous, no-throw/no-panic/no-unwind contract. C++ callers
should use an `extern "C"` wrapper that catches C++ exceptions before entering
Recite; a Rust panic in an `extern "C"` callback aborts before Recite can catch
it.

Reverse P/Invoke callbacks are static and route to the service through an
owned `GCHandle`; Unity builds annotate them with `AOT.MonoPInvokeCallback`.
The headless check loads the native validators and exercises native start,
restore, choice, and acknowledgement traversal through the raw bridge, but it
does not prove the service's GCHandle callback path or an IL2CPP player build.
It also cannot prove a Godot-hosted Resource serialization round trip. Validate
those engine-hosted paths in target players before release.

Editor import/refresh tooling, schema export, native binary distribution, DOTS,
and package-manager release automation are tracked separately from this runtime
MVP.
