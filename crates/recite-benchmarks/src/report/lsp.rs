use super::{BenchGroup, BenchOperationReport, timed_operation, timed_operation_with_setup};
use crate::BenchmarkResult;
use crate::lsp::LspBenchmarkProject;

pub(super) fn fixture_operations(
    project: &LspBenchmarkProject,
    samples: usize,
) -> BenchmarkResult<Vec<BenchOperationReport>> {
    let probes = project.probes();
    let opened = || {
        let mut driver = project.driver();
        std::hint::black_box(driver.open_file(&probes.document));
        driver
    };
    Ok(vec![
        timed_operation(BenchGroup::Lsp, "initial_index", samples, || {
            std::hint::black_box(project.memory_report());
            Ok(())
        })?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "open_file_parse",
            samples,
            || project.driver(),
            |mut driver| {
                std::hint::black_box(driver.open_file(&probes.document));
                Ok(())
            },
        )?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "change_refresh",
            samples,
            opened,
            |mut driver| {
                std::hint::black_box(driver.change_file(&probes.document));
                Ok(())
            },
        )?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "diagnostics_refresh",
            samples,
            || project.driver(),
            |mut driver| {
                std::hint::black_box(driver.diagnostics_refresh(&probes.document));
                Ok(())
            },
        )?,
        timed_operation_with_setup(BenchGroup::Lsp, "completion", samples, opened, |driver| {
            std::hint::black_box(driver.completion(&probes.completion));
            Ok(())
        })?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "definition",
            samples,
            || project.driver(),
            |driver| {
                std::hint::black_box(driver.definition(&probes.definition));
                Ok(())
            },
        )?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "rename",
            samples,
            || project.driver(),
            |driver| {
                std::hint::black_box(driver.rename(&probes.rename, "renamed_block"));
                Ok(())
            },
        )?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "stale_change_suppression",
            samples,
            opened,
            |mut driver| {
                std::hint::black_box(driver.stale_change_is_suppressed(&probes.document));
                Ok(())
            },
        )?,
    ])
}
