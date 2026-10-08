use crate::compiler::{self, CompilerProject};
use crate::lsp::LspBenchmarkProject;
use crate::project::{BenchmarkProject, RealisticFixtureCounts};
use crate::runtime::RuntimeProject;
use crate::{BenchmarkFixture, BenchmarkResult, error};

use super::{
    BenchCounts, BenchGroup, BenchOperationReport, BenchTargetKind, BenchTargetReport,
    TargetMetadata, timed_operation, timed_operation_with_setup,
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
    // Shape metadata uses the same compiled tables for every selected group.
    let compiler_project = CompilerProject::load(&project)?;
    let compiled = compiler_project.compile_with_schema()?;
    let mut operations = Vec::new();
    for group in groups {
        match group {
            BenchGroup::Compiler => {
                operations.extend(compiler_fixture_operations(&compiler_project, samples)?);
            }
            BenchGroup::Runtime => {
                let runtime = RuntimeProject::load(&project, &compiled)?;
                operations.extend(runtime_fixture_operations(&runtime, samples)?);
            }
            BenchGroup::Lsp => {
                let lsp = LspBenchmarkProject::load(&project)?;
                operations.extend(super::lsp::fixture_operations(&lsp, samples)?);
            }
        }
    }

    let mut counts = fixture_counts(&project)?;
    counts.conditions = super::compiled_condition_sites(&compiled.asset().dialogue);
    counts.compiled_asset_bytes = Some(compiled.asset().messagepack.len() as u64);
    Ok(BenchTargetReport {
        target: project.fixture_label().to_owned(),
        kind: BenchTargetKind::Fixture,
        metadata: TargetMetadata {
            fixture: Some(project.fixture_label().to_owned()),
            project_root: None,
            counts,
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
        || compiler::parse_inputs(std::hint::black_box(&inputs)).map(std::hint::black_box),
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "lower",
        samples,
        || compiler::lower_inputs(std::hint::black_box(&inputs)).map(std::hint::black_box),
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "validate",
        samples,
        || {
            Ok(std::hint::black_box(compiler::validate_without_schema(
                std::hint::black_box(&source_files),
            )))
        },
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "validate_with_schema",
        samples,
        || {
            Ok(std::hint::black_box(compiler::validate_with_schema(
                std::hint::black_box(&source_files),
                std::hint::black_box(&schema),
            )))
        },
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "compile_with_schema",
        samples,
        || {
            let report = recite_compiler::compile::compile_inputs_with_schema(
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
            Ok(std::hint::black_box(report.asset))
        },
    )?);
    operations.push(timed_operation(
        BenchGroup::Compiler,
        "extract_pot_with_schema",
        samples,
        || {
            let report = recite_compiler::pot::extract_pot_with_schema(inputs.clone(), &schema);
            if !report.is_ok() {
                return Err(error(format!(
                    "POT extraction fixture produced {} diagnostics",
                    report.diagnostics.len()
                )));
            }
            Ok(std::hint::black_box(report.catalog))
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
            driver.start_scene()
        })?,
        timed_operation_with_setup(
            BenchGroup::Runtime,
            "next_line",
            samples,
            || driver.session_before_first_line(),
            |session| driver.next_line(session),
        )?,
        timed_operation_with_setup(
            BenchGroup::Runtime,
            "next_prompt",
            samples,
            || driver.session_before_first_prompt(),
            |session| driver.next_prompt(session),
        )?,
        timed_operation_with_setup(
            BenchGroup::Runtime,
            "choose_first",
            samples,
            || driver.session_with_prompt(),
            |session| driver.choose_first(session),
        )?,
        timed_operation_with_setup(
            BenchGroup::Runtime,
            "condition_dispatch",
            samples,
            || driver.session_before_condition_prompt(),
            |session| driver.condition_dispatch(session),
        )?,
        timed_operation_with_setup(
            BenchGroup::Runtime,
            "effect_immediate",
            samples,
            || driver.start_scene(),
            |session| driver.immediate_effect(session),
        )?,
        timed_operation_with_setup(
            BenchGroup::Runtime,
            "effect_deferred",
            samples,
            || driver.session_before_deferred_effect(),
            |session| driver.deferred_effect(session),
        )?,
        timed_operation_with_setup(
            BenchGroup::Runtime,
            "effect_blocking_ack",
            samples,
            || {
                let mut session = driver.session_before_blocking_effect()?;
                driver.blocking_effect(&mut session)?;
                Ok(session)
            },
            |session| driver.acknowledge_blocking(session),
        )?,
        timed_operation_with_setup(
            BenchGroup::Runtime,
            "localised_next",
            samples,
            || driver.localised_session_before_first_line(),
            |session| driver.localised_next(session),
        )?,
        timed_operation(BenchGroup::Runtime, "session_encode", samples, || {
            driver.encode_session(std::hint::black_box(&prompt_session))
        })?,
        timed_operation(BenchGroup::Runtime, "session_decode", samples, || {
            driver.decode_session(std::hint::black_box(&encoded_prompt))
        })?,
        timed_operation(BenchGroup::Runtime, "full_traversal", samples, || {
            driver.full_traversal()
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
