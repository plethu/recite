use std::fs;

use crate::workspace::LspWorkspace;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LspMemoryReport {
    pub source_files: usize,
    pub indexed_source_bytes: usize,
    pub diagnostics: usize,
    pub block_definitions: usize,
    pub block_references: usize,
    pub line_ids: usize,
    pub choice_ids: usize,
    pub metadata_keys: usize,
    pub condition_functions: usize,
    pub effect_functions: usize,
    pub estimated_summary_bytes: usize,
}

impl LspMemoryReport {
    pub(crate) fn from_workspace(workspace: &LspWorkspace) -> Self {
        let mut report = Self {
            source_files: 0,
            indexed_source_bytes: 0,
            diagnostics: 0,
            block_definitions: 0,
            block_references: 0,
            line_ids: 0,
            choice_ids: 0,
            metadata_keys: 0,
            condition_functions: 0,
            effect_functions: 0,
            estimated_summary_bytes: 0,
        };

        for summary in workspace.snapshot().summaries() {
            report.source_files += 1;
            report.diagnostics += summary.diagnostics.len();
            report.block_definitions += summary.blocks.len();
            report.block_references += summary.block_references.len();
            report.line_ids += summary.line_ids.len();
            report.choice_ids += summary.choice_ids.len();
            report.metadata_keys += summary.metadata_keys.len();
            report.condition_functions += summary.condition_functions.len();
            report.effect_functions += summary.effect_functions.len();
            if let Some(path) = summary.saved_path()
                && let Ok(metadata) = fs::metadata(path)
            {
                report.indexed_source_bytes = report
                    .indexed_source_bytes
                    .saturating_add(usize::try_from(metadata.len()).unwrap_or(usize::MAX));
            }
        }

        report.estimated_summary_bytes = report.estimate_summary_bytes();
        report
    }

    fn estimate_summary_bytes(&self) -> usize {
        self.indexed_source_bytes
            .saturating_add(self.block_definitions.saturating_mul(96))
            .saturating_add(self.block_references.saturating_mul(128))
            .saturating_add(self.line_ids.saturating_mul(96))
            .saturating_add(self.choice_ids.saturating_mul(96))
            .saturating_add(self.metadata_keys.saturating_mul(96))
            .saturating_add(self.condition_functions.saturating_mul(112))
            .saturating_add(self.effect_functions.saturating_mul(112))
            .saturating_add(self.diagnostics.saturating_mul(192))
    }

    #[must_use]
    pub fn to_markdown(&self) -> String {
        format!(
            concat!(
                "| Metric | Value |\n",
                "| --- | ---: |\n",
                "| source_files | {} |\n",
                "| indexed_source_bytes | {} |\n",
                "| diagnostics | {} |\n",
                "| block_definitions | {} |\n",
                "| block_references | {} |\n",
                "| line_ids | {} |\n",
                "| choice_ids | {} |\n",
                "| metadata_keys | {} |\n",
                "| condition_functions | {} |\n",
                "| effect_functions | {} |\n",
                "| estimated_summary_bytes | {} |\n"
            ),
            self.source_files,
            self.indexed_source_bytes,
            self.diagnostics,
            self.block_definitions,
            self.block_references,
            self.line_ids,
            self.choice_ids,
            self.metadata_keys,
            self.condition_functions,
            self.effect_functions,
            self.estimated_summary_bytes
        )
    }
}
