# Recite for Godot 4.6

The checked host target is the official Godot 4.6.3 Linux x86_64 standard
build. Run `scripts/package-godot-addon.sh` from the Recite repository to
build a release library (or set `RECITE_GODOT_PROFILE=debug` for a debug build), then
extract `recite-godot-addon.tar.gz` from the output directory and copy its
`addons/recite` directory into a Godot project. The archive also contains
`examples/basic-dialogue`, a small playable project with its `.recite` source
and project manifest. Enable
**Recite** in Project Settings → Plugins. The packaged library is a native
GDExtension; other platforms require a corresponding native build and library
entry in `recite.gdextension`. The package includes both MIT and Apache-2.0
license texts. Source: https://github.com/plethu/recite.

The editor imports `.recitec` files as `ReciteDialogueResource`. Build them
with `recite compile`; load a source path through `ResourceLoader.load()` or
reference it from another Godot Resource. `ReciteDialogueNode` owns a session
and emits structured `output` and `adapter_error` signals. Asset refreshes
apply to the next session. A rejected candidate reports a structured load
error and leaves a previously loaded Resource revision intact. A clean project
with no prior valid import cannot use an invalid compiled file. Compiled-only
imports have no claim about source/schema freshness; compare the active and
available content identities with `node.asset_state(asset)` for loaded revision
visibility. The editor keeps a derived `.lastgood` byte cache alongside its
imported `.res`; clearing Godot's `.godot` import directory clears this
fallback, so the source must import validly again.

Condition callables return a Boolean for Boolean conditions, a string for
enum conditions, or a `ReciteConditionFailure` object with its `message`
property set to report a game-query failure. The latter maps to the structured
`condition_evaluation_error` category; incompatible values map to
`invalid_condition_result_error`.

To upgrade an existing project, close the editor, keep authored `.recite`,
`.recitec`, scene, schema, and catalogue files in the project, then replace the
entire `addons/recite` directory with the one from the new archive. Reopen the
editor so Godot scans the new native library and reimports compiled dialogue.
Do not merge new addon files over an old copy: removed addon code would remain.
The packaging script uses a dedicated output directory and replaces only its
package-owned addon and example paths on later runs.

For the packaged example, copy the archive's `addons/recite` into
`examples/basic-dialogue`, compile its source as shown in that project's
README, and open the project in Godot. The [authoring walkthrough](https://github.com/plethu/recite/blob/main/docs-site/src/content/docs/adapters/authoring.md)
shows the source edit, diagnostics, watcher, import, and next-session sequence.

For schema authoring, create and save a `ReciteSchemaDeclarations` Resource.
Give `producer_id` a stable project-owned value and fill its declaration
sections with dictionaries using Recite's canonical schema field names. Set
`recite/schema_resource` to its `res://` path, `recite/schema_manifest` to the
desired output path, and optionally `recite/cli_path` to the `recite` CLI
executable in Project Settings. Run **Project → Tools → Export Recite Schema**.
The addon writes temporary TOML and asks the CLI to validate and export the
canonical JSON with `godot` producer identity and fingerprint. Failed exports
leave the previous manifest in place.
