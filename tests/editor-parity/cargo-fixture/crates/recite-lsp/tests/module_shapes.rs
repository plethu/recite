#[path = "../build.rs"]
mod build_script_input;
const _: &str = build_script_input::BUILD_SHARED;
const _: &str = build_script_input::WORKSPACE_SHARED;

include!("module_tests.inc");
