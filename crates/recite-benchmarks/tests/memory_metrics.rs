use std::fs;

use recite_lsp::bench_support::{LspBenchmarkConfig, LspBenchmarkDriver};
use tempfile::tempdir;

#[test]
fn lsp_memory_report_counts_only_frozen_line_and_choice_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        ":: start default\n",
        ">\n",
        "  Missing line.\n",
        "> draft_line@\n",
        "  Draft line.\n",
        "> malformed_line@bad\n",
        "  Malformed line.\n",
        "> frozen_line@11111111111111111111\n",
        "  Frozen line.\n",
        "> prompt\n",
        "  Prompt.\n",
        "  ?\n",
        "    Missing choice.\n",
        "    -> END\n",
        "  ? draft_choice@\n",
        "    Draft choice.\n",
        "    -> END\n",
        "  ? malformed_choice@bad\n",
        "    Malformed choice.\n",
        "    -> END\n",
        "  ? frozen_choice@33333333333333333333\n",
        "    Frozen choice.\n",
        "    -> END\n",
    );
    let first = report_for_source(source)?;
    let second = report_for_source(source)?;

    assert_eq!(first, second);
    assert_eq!(first.source_files, 1);
    assert_eq!(first.line_ids, 1);
    assert_eq!(first.choice_ids, 1);
    assert_eq!(first.block_definitions, 1);
    assert_eq!(first.block_references, 0);
    assert_eq!(first.metadata_keys, 0);
    assert_eq!(first.condition_functions, 0);
    assert_eq!(first.effect_functions, 0);

    let expected_estimate = first
        .indexed_source_bytes
        .saturating_add(96)
        .saturating_add(96)
        .saturating_add(96)
        .saturating_add(first.diagnostics.saturating_mul(192));
    assert_eq!(first.estimated_summary_bytes, expected_estimate);
    Ok(())
}

fn report_for_source(
    source: &str,
) -> Result<recite_lsp::bench_support::LspMemoryReport, Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    fs::write(directory.path().join("metrics.recite"), source)?;
    let driver =
        LspBenchmarkDriver::new(&LspBenchmarkConfig::new(vec![directory.path().to_owned()]));
    Ok(driver.memory_report())
}
