# Bevy companion example

Run `cargo run -p recite-bevy --example headless_dialogue` from the repository root. The
package-owned example and `.recite` source live under `crates/recite-bevy/examples/` so they are
included when the crate is packaged.

The example builds a canonical Rust schema, compiles the source in an authoring-only dev dependency,
and drives a headless Bevy `App` through condition evaluation, all three effect modes, localization,
prompt and blocking-effect save/load, and a rebuilt asset used by the next session.

For an on-disk game project, export the generated schema JSON, run `recite watch <project-root>`
during editing, and load the resulting `.recitec` with `AssetServer::load::<ReciteDialogueAsset>`.
Call `AssetServer::reload(path)` after a rebuild if file watching is disabled. Start the next
session only after `ReciteAssetImport::Accepted` reports the new revision; an active session keeps
its original compiled bytes.
