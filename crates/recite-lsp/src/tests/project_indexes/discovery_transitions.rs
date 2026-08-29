#[path = "discovery_transitions/fallback.rs"]
mod fallback;
#[path = "discovery_transitions/manifest.rs"]
mod manifest;
#[path = "discovery_transitions/ownership.rs"]
mod ownership;

pub(crate) fn all() {
    fallback::all();
    manifest::all();
    ownership::all();
}
