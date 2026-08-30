#[path = "tests/authoring_registry/tests.rs"]
mod authoring_registry;
mod availability;
mod code_action;
#[path = "tests/config_registry/tests.rs"]
mod config_registry;
mod diagnostics;
mod edit_projection;
mod lifecycle;
mod navigation;
mod navigation_corrections;
mod navigation_ranges;
mod position;
mod project_indexes;
#[path = "tests/protocol_registry/tests.rs"]
mod protocol_registry;
mod support;
mod sync;
