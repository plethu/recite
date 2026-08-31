#[path = "discovery_transitions/fallback.rs"]
mod fallback;
#[path = "discovery_transitions/manifest.rs"]
mod manifest;
#[path = "discovery_transitions/ownership.rs"]
mod ownership;

pub(crate) use fallback::malformed_workspace_root_does_not_block_independent_root;

pub(crate) fn all() {
    fallback::all();
    manifest::all();
    ownership::all();
}
