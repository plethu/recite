mod inputs;
mod paths;
mod project;
mod project_asset;
mod project_diagnostics;
mod project_freshness;
mod project_sources;
mod scene_block;
mod schema;
mod sources;
mod write;

#[cfg(test)]
mod tests;

pub(crate) use inputs::{collect_input_files, compile_options, validate_inputs, validate_sources};
pub(crate) use paths::{display_path, reject_output_input_alias, resolve_project_path};
pub(crate) use project::{check_fresh, validate_project};
pub(crate) use project_freshness::validate_project_asset_freshness;
pub(crate) use scene_block::start_block;
pub(crate) use schema::{
    load_optional_schema, load_schema, load_schema_for_freshness, warn_schema_choice,
};
pub(crate) use sources::{LooseKeys, SchemaChoice, SourceSet};
pub(crate) use write::write_staged;
