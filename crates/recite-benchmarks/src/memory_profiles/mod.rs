use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use recite_runtime::DialogueSession;
use serde::Serialize;

use crate::compiler::{CompiledProject, CompilerProject};
use crate::lsp::LspBenchmarkProject;
use crate::project::{BenchmarkProject, RealisticFixtureCounts};
use crate::runtime::RuntimeProject;
use crate::{BenchmarkFixture, BenchmarkResult, error};

mod markdown;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryProfileOptions {
    fixtures: Vec<BenchmarkFixture>,
    include_compiler_peak: bool,
    compiler_peak_executable: Option<PathBuf>,
}

impl MemoryProfileOptions {
    #[must_use]
    pub fn new(fixtures: Vec<BenchmarkFixture>) -> Self {
        Self {
            fixtures,
            include_compiler_peak: true,
            compiler_peak_executable: None,
        }
    }

    #[must_use]
    pub fn fixtures(&self) -> &[BenchmarkFixture] {
        &self.fixtures
    }

    #[must_use]
    pub fn without_compiler_peak(mut self) -> Self {
        self.include_compiler_peak = false;
        self
    }

    #[must_use]
    pub fn with_compiler_peak_executable(mut self, executable: PathBuf) -> Self {
        self.compiler_peak_executable = Some(executable);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct MemoryProfileReport {
    pub generated_by: &'static str,
    pub compiler_peak_caveat: &'static str,
    pub fixtures: Vec<FixtureMemoryProfile>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FixtureMemoryProfile {
    pub fixture: &'static str,
    pub counts: FixtureMemoryCounts,
    pub project_bytes: ProjectByteReport,
    pub compiled_asset: CompiledAssetMemoryReport,
    pub runtime_sessions: RuntimeSessionMemoryReport,
    pub lsp_index: recite_lsp::bench_support::LspMemoryReport,
    pub compiler_peak_rss_kib: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FixtureMemoryCounts {
    pub source_files: u64,
    pub schema_files: u64,
    pub runtime_fixtures: u64,
    pub locale_catalogs: u64,
    pub recite_lines: u64,
    pub dialogue_lines: u64,
    pub choices: u64,
    pub effects: u64,
    pub conditions: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProjectByteReport {
    pub sources: u64,
    pub schema: u64,
    pub runtime_fixture: u64,
    pub locale_catalogs: u64,
    pub total: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompiledAssetMemoryReport {
    pub messagepack_bytes: usize,
    pub blocks: usize,
    pub lines: usize,
    pub choices: usize,
    pub effects: usize,
    pub conditions: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeSessionMemoryReport {
    pub samples: Vec<RuntimeSessionSample>,
    pub max_messagepack_bytes: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct RuntimeSessionSample {
    pub name: &'static str,
    pub messagepack_bytes: usize,
}

pub fn build_memory_profile_report(
    options: &MemoryProfileOptions,
) -> BenchmarkResult<MemoryProfileReport> {
    let mut fixtures = Vec::with_capacity(options.fixtures().len());
    for fixture in options.fixtures() {
        fixtures.push(build_fixture_profile(
            *fixture,
            options.include_compiler_peak,
            options.compiler_peak_executable.as_deref(),
        )?);
    }

    Ok(MemoryProfileReport {
        generated_by: "recite-benchmarks memory_profile_report",
        compiler_peak_caveat: compiler_peak_caveat(),
        fixtures,
    })
}

pub fn parse_linux_vm_hwm_kib(status: &str) -> Option<u64> {
    status.lines().find_map(|line| {
        let value = line.strip_prefix("VmHWM:")?.trim();
        let mut parts = value.split_whitespace();
        let kib = parts.next()?.parse::<u64>().ok()?;
        (parts.next() == Some("kB")).then_some(kib)
    })
}

fn build_fixture_profile(
    fixture: BenchmarkFixture,
    include_compiler_peak: bool,
    compiler_peak_executable: Option<&Path>,
) -> BenchmarkResult<FixtureMemoryProfile> {
    let project = BenchmarkProject::load_fixture(fixture)?;
    let project_bytes = project_byte_report(&project)?;
    let compiler = CompilerProject::load(&project)?;
    let compiled = compiler.compile_with_schema()?;
    let counts = fixture_counts(&project, &compiler, &compiled)?;
    let runtime = RuntimeProject::load(&project, &compiled)?;
    let runtime_sessions = runtime_session_report(&runtime)?;
    let lsp = LspBenchmarkProject::load(&project)?;
    let compiler_peak_rss_kib = if include_compiler_peak {
        measure_compiler_peak_rss_kib(fixture, compiler_peak_executable)?
    } else {
        None
    };
    let dialogue = &compiled.asset().dialogue;

    Ok(FixtureMemoryProfile {
        fixture: project.fixture_label(),
        counts,
        project_bytes,
        compiled_asset: CompiledAssetMemoryReport {
            messagepack_bytes: compiled.asset().messagepack.len(),
            blocks: dialogue.blocks.len(),
            lines: dialogue.lines.len(),
            choices: dialogue.choices.len(),
            effects: dialogue.effects.len(),
            conditions: dialogue.condition_availability_reasons.len(),
        },
        runtime_sessions,
        lsp_index: lsp.memory_report(),
        compiler_peak_rss_kib,
    })
}

pub fn compiler_peak_child(fixture: BenchmarkFixture) -> BenchmarkResult<Option<u64>> {
    let project = BenchmarkProject::load_fixture(fixture)?;
    let compiler = CompilerProject::load(&project)?;
    let compiled = compiler.compile_with_schema()?;
    std::hint::black_box(compiled);
    read_peak_rss_kib()
}

fn measure_compiler_peak_rss_kib(
    fixture: BenchmarkFixture,
    executable: Option<&Path>,
) -> BenchmarkResult<Option<u64>> {
    let Some(executable) = executable else {
        return read_peak_rss_kib();
    };

    if !cfg!(target_os = "linux") {
        return Ok(None);
    }

    let output = Command::new(executable)
        .arg("--compiler-peak-child")
        .arg(fixture.as_str())
        .output()?;
    if !output.status.success() {
        return Err(error(format!(
            "compiler peak child for `{fixture}` failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed == "null" {
        return Ok(None);
    }
    trimmed.parse::<u64>().map(Some).map_err(|parse_error| {
        error(format!(
            "compiler peak child for `{fixture}` returned `{trimmed}`: {parse_error}"
        ))
    })
}

fn fixture_counts(
    project: &BenchmarkProject,
    compiler: &CompilerProject,
    compiled: &CompiledProject,
) -> BenchmarkResult<FixtureMemoryCounts> {
    if let Some(summary) = project.realistic_summary() {
        return Ok(realistic_counts(&summary.counts));
    }

    let dialogue = &compiled.asset().dialogue;
    let counts = &project.summary().counts;
    Ok(FixtureMemoryCounts {
        source_files: project.source_files()?.len() as u64,
        schema_files: 1,
        runtime_fixtures: 1,
        locale_catalogs: directory_file_count(&project.root().join("locale"))?,
        recite_lines: u64::from(counts.lines),
        dialogue_lines: dialogue.lines.len() as u64,
        choices: u64::from(counts.choices),
        effects: dialogue.effects.len() as u64,
        conditions: compiler.schema().conditions.len() as u64,
    })
}

fn realistic_counts(counts: &RealisticFixtureCounts) -> FixtureMemoryCounts {
    FixtureMemoryCounts {
        source_files: counts.source_files,
        schema_files: counts.schema_files,
        runtime_fixtures: counts.runtime_fixtures,
        locale_catalogs: counts.locale_catalogs,
        recite_lines: counts.recite_lines,
        dialogue_lines: counts.dialogue_lines,
        choices: counts.choices,
        effects: counts.effects,
        conditions: counts.conditions,
    }
}

fn project_byte_report(project: &BenchmarkProject) -> BenchmarkResult<ProjectByteReport> {
    let source_bytes = project
        .source_files()?
        .iter()
        .map(|file| file.source.len() as u64)
        .sum::<u64>();
    let schema_bytes = project.schema_file()?.source.len() as u64;
    let runtime_fixture_bytes = project.runtime_fixture_source()?.len() as u64;
    let locale_catalog_bytes = directory_bytes(&project.root().join("locale"))?;
    let total = source_bytes
        .saturating_add(schema_bytes)
        .saturating_add(runtime_fixture_bytes)
        .saturating_add(locale_catalog_bytes);

    Ok(ProjectByteReport {
        sources: source_bytes,
        schema: schema_bytes,
        runtime_fixture: runtime_fixture_bytes,
        locale_catalogs: locale_catalog_bytes,
        total,
    })
}

fn directory_bytes(path: &Path) -> BenchmarkResult<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total = total.saturating_add(directory_bytes(&entry.path())?);
        } else if metadata.is_file() {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

fn directory_file_count(path: &Path) -> BenchmarkResult<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let mut total = 0_u64;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            total = total.saturating_add(directory_file_count(&entry.path())?);
        } else if metadata.is_file() {
            total = total.saturating_add(1);
        }
    }
    Ok(total)
}

fn runtime_session_report(runtime: &RuntimeProject) -> BenchmarkResult<RuntimeSessionMemoryReport> {
    let driver = runtime.driver();
    let samples = vec![
        session_sample("start_scene", driver.start_scene()?, &driver)?,
        session_sample(
            "before_first_line",
            driver.session_before_first_line()?,
            &driver,
        )?,
        session_sample("with_prompt", driver.session_with_prompt()?, &driver)?,
        session_sample(
            "before_blocking_effect",
            driver.session_before_blocking_effect()?,
            &driver,
        )?,
    ];
    let max_messagepack_bytes = samples
        .iter()
        .map(|sample| sample.messagepack_bytes)
        .max()
        .ok_or_else(|| error("runtime session report produced no samples"))?;

    Ok(RuntimeSessionMemoryReport {
        samples,
        max_messagepack_bytes,
    })
}

fn session_sample(
    name: &'static str,
    session: DialogueSession,
    driver: &crate::runtime::TraversalDriver<'_>,
) -> BenchmarkResult<RuntimeSessionSample> {
    Ok(RuntimeSessionSample {
        name,
        messagepack_bytes: driver.encode_session(&session)?.len(),
    })
}

fn read_peak_rss_kib() -> BenchmarkResult<Option<u64>> {
    if !cfg!(target_os = "linux") {
        return Ok(None);
    }
    let status = fs::read_to_string("/proc/self/status")?;
    Ok(parse_linux_vm_hwm_kib(&status))
}

fn compiler_peak_caveat() -> &'static str {
    if cfg!(target_os = "linux") {
        "On Linux the report binary measures `/proc/self/status` `VmHWM` in a fresh child process per fixture."
    } else {
        "This platform does not expose Linux `/proc/self/status` `VmHWM`; compiler peak RSS is reported as null."
    }
}
