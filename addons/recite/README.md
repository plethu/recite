# Recite for Godot 4.6

The checked host target is the official Godot 4.6.3 Linux x86_64 standard build. Run
`scripts/package-godot-addon.sh` from the Recite repository to build a release library (or set
`RECITE_GODOT_PROFILE=debug` for a debug build), then extract `recite-godot-addon.tar.gz` from the
output directory and copy its `addons/recite` directory into a Godot project. The archive also
contains `examples/basic-dialogue`, a small playable project with its `.recite` source and project
manifest. Enable **Recite** in Project Settings → Plugins. The packaged library is a native
GDExtension; other platforms require a corresponding native build and library entry in
`recite.gdextension`. The package includes both MIT and Apache-2.0 license texts. Source:
<https://github.com/plethu/recite>.

The editor imports `.recitec` files as `ReciteDialogueResource`. Build them with `recite compile`;
load a source path through `ResourceLoader.load()` or reference it from another Godot Resource.
`ReciteDialogueNode` owns a session and emits structured `output` and `adapter_error` signals. Asset
refreshes apply to the next session. A rejected candidate reports a structured load error and leaves
a previously loaded Resource revision intact. A clean project with no prior valid import cannot use
an invalid compiled file. Compiled-only imports have no claim about source/schema freshness; compare
the active and available content identities with `node.asset_state(asset)` for loaded revision
visibility. The editor keeps a derived `.lastgood` byte cache alongside its imported `.res`;
clearing Godot's `.godot` import directory clears this fallback, so the source must import validly
again.

Condition callables return a Boolean for Boolean conditions, a string for enum conditions, or a
`ReciteConditionFailure` object with its `message` property set to report a game-query failure. The
latter maps to the structured `condition_evaluation_error` category; incompatible values map to
`invalid_condition_result_error`.

To upgrade an existing project, close the editor, keep authored `.recite`, `.recitec`, scene,
schema, and catalogue files in the project, then replace the entire `addons/recite` directory with
the one from the new archive. Reopen the editor so Godot scans the new native library and reimports
compiled dialogue. Do not merge new addon files over an old copy: removed addon code would remain.
The packaging script uses a dedicated output directory and replaces only its package-owned addon and
example paths on later runs.

For the packaged example, copy the archive's `addons/recite` into `examples/basic-dialogue`, compile
its source as shown in that project's README, and open the project in Godot. The
[authoring walkthrough](https://github.com/plethu/recite/blob/main/docs-site/src/content/docs/adapters/authoring.md)
shows the source edit, diagnostics, watcher, import, and next-session sequence.

For schema authoring, create and save a `ReciteSchemaDeclarations` Resource. Give `producer_id` a
stable project-owned value and fill its declaration sections with dictionaries using Recite's
canonical schema field names. Set `recite/schema_resource` to its `res://` path,
`recite/schema_manifest` to the desired output path, and optionally `recite/cli_path` to the
`recite` CLI executable in Project Settings. Run **Project → Tools → Export Recite Schema**. The
addon writes temporary TOML and asks the CLI to validate and export the canonical JSON with `godot`
producer identity and fingerprint. Failed exports leave the previous manifest in place.

## Localisation

`ReciteDialogueCatalogResource` is the Godot-facing owner for translated dialogue. Add it as a
Resource, install a complete gettext plural rule before its plural entries, and assign it to
`ReciteDialogueNode`:

```gdscript
var catalog := ReciteDialogueCatalogResource.new()
catalog.set_plural_forms("fr", "nplurals=2; plural=(n != 1);")
catalog.add_translation("fr", "greeting", "Hello {name}.", "Bonjour {name}.", "formal")
catalog.add_plural_translation(
    "fr", "letters", "One letter.", "{count} letters.",
    ["Une lettre.", "{count} lettres."], "formal")

$ReciteDialogueNode.set_locale_catalog(catalog)
$ReciteDialogueNode.start_with_variant(dialogue, "start", "fr-CA", "formal")
```

The Resource stores entries and plural headers in serializable Godot properties. When a Resource is
loaded, the Rust adapter rebuilds a validated, owned catalogue from those properties. Placeholder
names must be preserved; plural entries must have exactly the rule's `nplurals` arms. Empty
translated arms use the normal source-text fallback. The lookup order is explicit variant context,
unqualified context, BCP-47 locale fallback, and authored source.

The runtime snapshot preserves the selected locale and deterministic session state, but not the
Resource, interpolation values, or grammatical variant. On restore, keep the Resource installed,
restore the same typed values, and call `restore_with_variant` with the selected variant before
traversal resumes. No catalogue lookup or traversal operation performs game-side effects.

Rust unit tests cover the catalogue's line, choice, availability-reason, presentation-label, plural,
variant, fallback, placeholder, and restore semantics. The executable host conformance lane is
`scripts/check-godot-host.sh`. It builds the GDExtension, compiles the basic dialogue fixture,
initializes a clean Godot project, and exercises malformed persisted array/dictionary/plural shapes,
reload-before-mutation, a `.tres` round trip, class registration, and output signal delivery. It
uses the official Godot 4.6.3 stable Linux x86_64 standard build
(`4.6.3.stable.official.7d41c59c4`), matching the crate's `api-4-6` feature; set `GODOT` to that
binary when it is not on `PATH`. The ordinary Cargo lane remains host-independent, while this script
is the required engine-hosted evidence before changing the Resource format.

## Verification

The [host tests](https://github.com/plethu/recite/blob/main/tests/godot-host/run_tests.gd) observe
transactional batches through actual Godot classes and signals; related Rust tests do not establish
exact manifest execution. They also exercise rejected imports, persisted Resources, reentrant signal
order and a clean packaged example. Immutable session ownership prevents replacing the asset during
advance. Projection is unsupported; compiled-only imports cannot establish source/schema freshness.
The
[shared scenarios](https://github.com/plethu/recite/blob/main/fixtures/adapter-conformance/v1/scenarios.json)
own required observations.
