use recite_benchmarks::memory_profiles::{
    MemoryProfileOptions, build_memory_profile_report, parse_linux_vm_hwm_kib,
};
use recite_benchmarks::{BenchmarkFixture, BenchmarkScale};

#[test]
fn tiny_memory_profile_reports_core_size_metrics() -> Result<(), Box<dyn std::error::Error>> {
    let report = build_memory_profile_report(
        &MemoryProfileOptions::new(vec![BenchmarkFixture::Synthetic(BenchmarkScale::Tiny)])
            .without_compiler_peak(),
    )?;

    let fixture = report.fixtures.first().expect("tiny fixture report");
    assert_eq!(fixture.fixture, "tiny");
    assert_eq!(fixture.counts.source_files, 2);
    assert_eq!(fixture.counts.dialogue_lines, 100);
    assert_eq!(fixture.counts.choices, 20);
    assert!(fixture.project_bytes.sources > 0);
    assert!(fixture.project_bytes.schema > 0);
    assert!(fixture.project_bytes.runtime_fixture > 0);
    assert!(fixture.project_bytes.total >= fixture.project_bytes.sources);
    assert!(fixture.compiled_asset.messagepack_bytes > 0);
    assert_eq!(fixture.compiled_asset.blocks, 10);
    assert_eq!(fixture.lsp_index.source_files, 2);
    assert!(fixture.lsp_index.estimated_summary_bytes >= fixture.lsp_index.indexed_source_bytes);
    assert!(fixture.compiler_peak_rss_kib.is_none());
    Ok(())
}

#[test]
fn runtime_session_report_tracks_max_sample_size() -> Result<(), Box<dyn std::error::Error>> {
    let report = build_memory_profile_report(
        &MemoryProfileOptions::new(vec![BenchmarkFixture::Synthetic(BenchmarkScale::Tiny)])
            .without_compiler_peak(),
    )?;

    let sessions = &report.fixtures[0].runtime_sessions;
    assert!(sessions.samples.len() >= 3);
    assert!(
        sessions
            .samples
            .iter()
            .all(|sample| sample.messagepack_bytes > 0)
    );
    assert_eq!(
        sessions.max_messagepack_bytes,
        sessions
            .samples
            .iter()
            .map(|sample| sample.messagepack_bytes)
            .max()
            .expect("at least one session sample")
    );
    Ok(())
}

#[test]
fn memory_profile_markdown_contains_release_caveats() -> Result<(), Box<dyn std::error::Error>> {
    let report = build_memory_profile_report(
        &MemoryProfileOptions::new(vec![BenchmarkFixture::Synthetic(BenchmarkScale::Tiny)])
            .without_compiler_peak(),
    )?;
    let markdown = report.to_markdown();

    assert!(markdown.contains("# Recite Memory Profiles And Known Scale Limits"));
    assert!(markdown.contains("| tiny |"));
    assert!(markdown.contains("Numeric budgets remain evidence"));
    assert!(markdown.contains("LSP memory is an estimated index-summary size"));
    Ok(())
}

#[test]
fn linux_vm_hwm_parser_accepts_proc_status_units() {
    let status = "\
Name:\trecite\n\
VmRSS:\t  1200 kB\n\
VmHWM:\t  4096 kB\n";

    assert_eq!(parse_linux_vm_hwm_kib(status), Some(4096));
}

#[test]
fn linux_vm_hwm_parser_rejects_missing_or_unexpected_units() {
    assert_eq!(parse_linux_vm_hwm_kib("VmRSS:\t1200 kB\n"), None);
    assert_eq!(parse_linux_vm_hwm_kib("VmHWM:\t4096 bytes\n"), None);
}
