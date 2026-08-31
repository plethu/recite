mod inputs;
mod paths;
mod project;
mod project_asset;
mod project_diagnostics;
mod project_sources;
mod schema;
mod write;

#[cfg(test)]
mod tests;

pub(crate) use inputs::{
    collect_input_files, compile_options, read_compile_inputs_for_output,
    read_compile_inputs_from_files, validate_inputs,
};
pub(crate) use paths::{display_path, reject_output_input_alias, resolve_project_path};
pub(crate) use project::{check_fresh, validate_project};
pub(crate) use schema::{load_optional_schema, load_schema, load_schema_for_freshness};
pub(crate) use write::write_staged;
