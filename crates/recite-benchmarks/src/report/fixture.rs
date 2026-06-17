use crate::compiler::{self, CompilerProject};
use crate::lsp::LspBenchmarkProject;
use crate::project::{BenchmarkProject, RealisticFixtureCounts};
use crate::runtime::RuntimeProject;
use crate::{BenchmarkFixture, BenchmarkResult, error};

use super::{
    BenchCounts, BenchGroup, BenchOperationReport, BenchTargetKind, BenchTargetReport,
    TargetMetadata, timed_operation,
};

pub(super) fn build_fixture_reports(
    fixtures: &[BenchmarkFixture],
    groups: &[BenchGroup],
    samples: usize,
) -> BenchmarkResult<Vec<BenchTargetReport>> {
    if fixtures.is_empty() {
        return Err(error("recite bench requires at least one fixture"));
    }
    fixtures
        .iter()
        .copied()
        .map(|fixture| build_fixture_report(fixture, groups, samples))
        .collect()
}

fn build_fixture_report(
    fixture: BenchmarkFixture,
    groups: &[BenchGroup],
    samples: usize,
) -> BenchmarkResult<BenchTargetReport> {
    let project = BenchmarkProject::load_fixture(fixture)?;
    let compiler_project = if groups.iter().any(|group| {
        matches!(
            group,
            BenchGroup::Compiler | BenchGroup::Runtime | BenchGroup::Lsp
        )
    }) {
        Some(CompilerProject::load(&project)?)
    } else {
        None
    };
    let compiled = if groups
        .iter()
        .any(|group| matches!(group, BenchGroup::Runtime))
    {
        Some(
            compiler_project
                .as_ref()
                .ok_or_else(|| error("compiler project was not loaded"))?
                .compile_with_schema()?,
        )
    } else {
        None
    };
    let mut operations = Vec::new();
    for group in groups {
        match group {
            BenchGroup::Compiler => {
                operations.extend(compiler_fixture_operations(
                    compiler_project
                        .as_ref()
                        .ok_or_else(|| error("compiler project was not loaded"))?,
                    samples,
                )?);
            }
            BenchGroup::Runtime => {
                let runtime = RuntimeProject::load(
                    &project,
                    compiled
                        .as_ref()
                        .ok_or_else(|| error("compiled project was not loaded"))?,
                )?;
                operations.extend(runtime_fixture_operations(&runtime, samples)?);
            }
            BenchGroup::Lsp => {
                let lsp = LspBenchmarkProject::load(&project)?;
                operations.extend(lsp_fixture_operations(&lsp, samples)?);
            }
        }
    }

    Ok(BenchTargetReport {
        target: project.fixture_label().to_owned(),
        kind: BenchTargetKind::Fixture,
        metadata: TargetMetadata {
            fixture: Some(project.fixture_label().to_owned()),
            project_root: None,
            counts: fixture_counts(&project)?,
            notes: fixture_notes(project.fixture()),
        },
        operations,
    })
}

fn compiler_fixture_operations(
    project: &CompilerProject,
    samples: usize,
) -> BenchmarkResult<Vec<BenchOperationReport>> {
    let inputs = project.compile_inputs();
    let source_files = project.source_files();
    let schema = project.schema().clone();
    let options = project.options();
    let mut operations = Vec::new();
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "parse",
        samples,
        || {
            compiler::parse_inputs(std::hint::black_box(&inputs)).map(|count| {
                std::hint::black_box(count);
            })
        },
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "lower",
        samples,
        || {
            compiler::lower_inputs(std::hint::black_box(&inputs)).map(|files| {
                std::hint::black_box(files);
            })
        },
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "validate",
        samples,
        || {
            std::hint::black_box(compiler::validate_without_schema(std::hint::black_box(
                &source_files,
            )));
            Ok(())
        },
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "validate_with_schema",
        samples,
        || {
            std::hint::black_box(compiler::validate_with_schema(
                std::hint::black_box(&source_files),
                std::hint::black_box(&schema),
            ));
            Ok(())
        },
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "compile_with_schema",
        samples,
        || {
            let report = recite_compiler::compile_inputs_with_schema(
                inputs.clone(),
                options.clone(),
                &schema,
            )?;
            if !report.is_ok() {
                return Err(error(format!(
                    "compile fixture produced {} diagnostics",
                    report.diagnostics.len()
                )));
            }
            std::hint::black_box(report.asset);
            Ok(())
        },
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "extract_pot_with_schema",
        samples,
        || {
            let report = recite_compiler::extract_pot_with_schema(inputs.clone(), &schema);
            if !report.is_ok() {
                return Err(error(format!(
                    "POT extraction fixture produced {} diagnostics",
                    report.diagnostics.len()
                )));
            }
            std::hint::black_box(report.catalog);
            Ok(())
        },
    )?);
    Ok(operations)
}

fn runtime_fixture_operations(
    runtime: &RuntimeProject,
    samples: usize,
) -> BenchmarkResult<Vec<BenchOperationReport>> {
    let driver = runtime.driver();
    let encoded_prompt = driver.encoded_prompt_session()?;
    let prompt_session = driver.session_with_prompt()?;
    Ok(vec![
        timed_operation(BenchGroup::Runtime, "start_scene", samples, || {
            driver.start_scene().map(|session| {
                std::hint::black_box(session);
            })
        })?,
        timed_operation(BenchGroup::Runtime, "next_line", samples, || {
            let mut session = driver.session_before_first_line()?;
            driver.next_line(&mut session).map(|event| {
                std::hint::black_box(event);
            })
        })?,
        timed_operation(BenchGroup::Runtime, "next_prompt", samples, || {
            let mut session = driver.session_before_first_prompt()?;
            driver.next_prompt(&mut session).map(|event| {
                std::hint::black_box(event);
            })
        })?,
        timed_operation(BenchGroup::Runtime, "choose_first", samples, || {
            let mut session = driver.session_with_prompt()?;
            driver.choose_first(&mut session).map(|event| {
                std::hint::black_box(event);
            })
        })?,
        timed_operation(BenchGroup::Runtime, "condition_dispatch", samples, || {
            let mut session = driver.session_before_condition_prompt()?;
            driver.condition_dispatch(&mut session).map(|event| {
                std::hint::black_box(event);
            })
        })?,
        timed_operation(BenchGroup::Runtime, "effect_immediate", samples, || {
            let mut session = driver.start_scene()?;
            driver.immediate_effect(&mut session).map(|event| {
                std::hint::black_box(event);
            })
        })?,
        timed_operation(BenchGroup::Runtime, "effect_deferred", samples, || {
            let mut session = driver.session_before_deferred_effect()?;
            driver.deferred_effect(&mut session).map(|event| {
                std::hint::black_box(event);
            })
        })?,
        timed_operation(BenchGroup::Runtime, "effect_blocking_ack", samples, || {
            let mut session = driver.session_before_blocking_effect()?;
            driver.blocking_effect(&mut session)?;
            driver.acknowledge_blocking(&mut session)?;
            std::hint::black_box(session);
            Ok(())
        })?,
        timed_operation(BenchGroup::Runtime, "localised_next", samples, || {
            let mut session = driver.localised_session_before_first_line()?;
            driver.localised_next(&mut session).map(|event| {
                std::hint::black_box(event);
            })
        })?,
        timed_operation(BenchGroup::Runtime, "session_encode", samples, || {
            driver
                .encode_session(std::hint::black_box(&prompt_session))
                .map(|bytes| {
                    std::hint::black_box(bytes);
                })
        })?,
        timed_operation(BenchGroup::Runtime, "session_decode", samples, || {
            driver
                .decode_session(std::hint::black_box(&encoded_prompt))
                .map(|session| {
                    std::hint::black_box(session);
                })
        })?,
        timed_operation(BenchGroup::Runtime, "full_traversal", samples, || {
            driver.full_traversal().map(|events| {
                std::hint::black_box(events);
            })
        })?,
    ])
}

fn lsp_fixture_operations(
    project: &LspBenchmarkProject,
    samples: usize,
) -> BenchmarkResult<Vec<BenchOperationReport>> {
    let probes = project.probes();
    Ok(vec![
        timed_operation(BenchGroup::Lsp, "initial_index", samples, || {
            std::hint::black_box(project.memory_report());
            Ok(())
        })?,
        timed_operation(BenchGroup::Lsp, "open_file_parse", samples, || {
            let mut driver = project.driver();
            std::hint::black_box(driver.open_file(&probes.document));
            Ok(())
        })?,
        timed_operation(BenchGroup::Lsp, "change_refresh", samples, || {
            let mut driver = project.driver();
            std::hint::black_box(driver.change_file(&probes.document));
            Ok(())
        })?,
        timed_operation(BenchGroup::Lsp, "diagnostics_refresh", samples, || {
            let mut driver = project.driver();
            std::hint::black_box(driver.diagnostics_refresh(&probes.document));
            Ok(())
        })?,
        timed_operation(BenchGroup::Lsp, "completion", samples, || {
            let driver = project.driver();
            std::hint::black_box(driver.completion(&probes.completion));
            Ok(())
        })?,
        timed_operation(BenchGroup::Lsp, "definition", samples, || {
            let driver = project.driver();
            std::hint::black_box(driver.definition(&probes.definition));
            Ok(())
        })?,
        timed_operation(BenchGroup::Lsp, "rename", samples, || {
            let driver = project.driver();
            std::hint::black_box(driver.rename(&probes.rename, "renamed_block"));
            Ok(())
        })?,
        timed_operation(BenchGroup::Lsp, "stale_change_suppression", samples, || {
            let mut driver = project.driver();
            std::hint::black_box(driver.stale_change_is_suppressed(&probes.document));
            Ok(())
        })?,
    ])
}

fn fixture_counts(project: &BenchmarkProject) -> BenchmarkResult<BenchCounts> {
    if let Some(summary) = project.realistic_summary() {
        return Ok(realistic_counts(&summary.counts, Some(summary.bytes)));
    }
    let summary = project.summary();
    Ok(BenchCounts {
        source_files: summary.counts.shards as u64,
        schema_files: 1,
        runtime_fixtures: 1,
        locale_catalogs: 1,
        recite_lines: 0,
        blocks: summary.counts.blocks as u64,
        dialogue_lines: summary.counts.lines as u64,
        choices: summary.counts.choices as u64,
        effects: 0,
        conditions: 0,
        generated_words: Some(summary.counts.generated_words as u64),
        project_bytes: Some(summary.files.iter().map(|file| file.bytes).sum()),
        compiled_asset_bytes: None,
    })
}

fn realistic_counts(counts: &RealisticFixtureCounts, project_bytes: Option<u64>) -> BenchCounts {
    BenchCounts {
        source_files: counts.source_files,
        schema_files: counts.schema_files,
        runtime_fixtures: counts.runtime_fixtures,
        locale_catalogs: counts.locale_catalogs,
        recite_lines: counts.recite_lines,
        blocks: 0,
        dialogue_lines: counts.dialogue_lines,
        choices: counts.choices,
        effects: counts.effects,
        conditions: counts.conditions,
        generated_words: None,
        project_bytes,
        compiled_asset_bytes: None,
    }
}

fn fixture_notes(fixture: BenchmarkFixture) -> Vec<String> {
    match fixture {
        BenchmarkFixture::Synthetic(scale) => vec![format!(
            "`{}` is a synthetic fixture ID; use the counts above when comparing scale reports.",
            scale.as_str()
        )],
        BenchmarkFixture::RealisticV1Pack => {
            vec!["`realistic:v1-pack` is a checked realistic fixture pack.".to_owned()]
        }
    }
}
