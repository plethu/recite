//! Concrete workload shape; schema declarations do not count as executed condition sites.
use recite_core::compiled::{CompiledDialogue, CompiledStatementKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Default, Eq, PartialEq, Serialize)]
pub struct BenchCounts {
    pub source_files: u64,
    pub schema_files: u64,
    pub runtime_fixtures: u64,
    pub locale_catalogs: u64,
    pub recite_lines: u64,
    pub blocks: u64,
    pub dialogue_lines: u64,
    pub choices: u64,
    pub effects: u64,
    /// Condition sites: if/match statements and choice availability requirements.
    pub conditions: u64,
    pub generated_words: Option<u64>,
    pub project_bytes: Option<u64>,
    pub compiled_asset_bytes: Option<u64>,
}

pub(super) fn compiled_condition_sites(dialogue: &CompiledDialogue) -> u64 {
    let statements = dialogue
        .statements
        .iter()
        .filter(|statement| {
            matches!(
                statement.kind,
                CompiledStatementKind::If { .. } | CompiledStatementKind::Match { .. }
            )
        })
        .count();
    let choices = dialogue
        .choices
        .iter()
        .filter(|choice| choice.availability_requirement.is_some())
        .count();
    (statements + choices) as u64
}
