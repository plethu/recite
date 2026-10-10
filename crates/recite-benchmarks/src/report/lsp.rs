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
        Ok(driver)
    };
    Ok(vec![
        timed_operation(BenchGroup::Lsp, "initial_index", samples, || {
            Ok(std::hint::black_box(project.driver()))
        })?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "open_file_parse",
            samples,
            || Ok(project.driver()),
            |driver| Ok(std::hint::black_box(driver.open_file(&probes.document))),
        )?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "change_refresh",
            samples,
            opened,
            |driver| Ok(std::hint::black_box(driver.change_file(&probes.document))),
        )?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "diagnostics_refresh",
            samples,
            || Ok(project.driver()),
            |driver| {
                Ok(std::hint::black_box(
                    driver.diagnostics_refresh(&probes.document),
                ))
            },
        )?,
        timed_operation_with_setup(BenchGroup::Lsp, "completion", samples, opened, |driver| {
            Ok(std::hint::black_box(driver.completion(&probes.completion)))
        })?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "definition",
            samples,
            || Ok(project.driver()),
            |driver| Ok(std::hint::black_box(driver.definition(&probes.definition))),
        )?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "rename",
            samples,
            || Ok(project.driver()),
            |driver| {
                Ok(std::hint::black_box(
                    driver.rename(&probes.rename, "renamed_block"),
                ))
            },
        )?,
        timed_operation_with_setup(
            BenchGroup::Lsp,
            "stale_change_suppression",
            samples,
            opened,
            |driver| {
                Ok(std::hint::black_box(
                    driver.stale_change_is_suppressed(&probes.document),
                ))
            },
        )?,
    ])
}
