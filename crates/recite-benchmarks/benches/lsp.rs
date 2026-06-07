use std::hint::black_box;
use std::path::PathBuf;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use recite_benchmarks::lsp::LspBenchmarkProject;
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::{BenchmarkFixture, BenchmarkResult};

fn lsp_benchmarks(criterion: &mut Criterion) {
    for fixture in load_lsp_projects() {
        bench_initial_index(criterion, &fixture);
        bench_open_file_parse(criterion, &fixture);
        bench_change_refresh(criterion, &fixture);
        bench_diagnostics_refresh(criterion, &fixture);
        bench_completion(criterion, &fixture);
        bench_definition(criterion, &fixture);
        bench_rename(criterion, &fixture);
        bench_stale_change_suppression(criterion, &fixture);
        write_memory_report(&fixture);
    }
}

fn bench_initial_index(criterion: &mut Criterion, fixture: &LspFixture) {
    criterion
        .benchmark_group("lsp/initial_index")
        .bench_function(
            BenchmarkId::from_parameter(fixture.project.fixture_label()),
            |bencher| {
                bencher.iter(|| black_box(fixture.project.driver().memory_report()));
            },
        );
}

fn bench_open_file_parse(criterion: &mut Criterion, fixture: &LspFixture) {
    criterion
        .benchmark_group("lsp/open_file_parse")
        .bench_function(
            BenchmarkId::from_parameter(fixture.project.fixture_label()),
            |bencher| {
                bencher.iter_batched(
                    || (fixture.project.driver(), fixture.probes.document.clone()),
                    |(mut driver, probe)| black_box(driver.open_file(black_box(&probe))),
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_change_refresh(criterion: &mut Criterion, fixture: &LspFixture) {
    criterion
        .benchmark_group("lsp/change_refresh")
        .bench_function(
            BenchmarkId::from_parameter(fixture.project.fixture_label()),
            |bencher| {
                bencher.iter_batched(
                    || (fixture.project.driver(), fixture.probes.document.clone()),
                    |(mut driver, probe)| black_box(driver.change_file(black_box(&probe))),
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_diagnostics_refresh(criterion: &mut Criterion, fixture: &LspFixture) {
    criterion
        .benchmark_group("lsp/diagnostics_refresh")
        .bench_function(
            BenchmarkId::from_parameter(fixture.project.fixture_label()),
            |bencher| {
                bencher.iter_batched(
                    || (fixture.project.driver(), fixture.probes.document.clone()),
                    |(mut driver, probe)| black_box(driver.diagnostics_refresh(black_box(&probe))),
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_completion(criterion: &mut Criterion, fixture: &LspFixture) {
    criterion.benchmark_group("lsp/completion").bench_function(
        BenchmarkId::from_parameter(fixture.project.fixture_label()),
        |bencher| {
            let mut driver = fixture.project.driver();
            let _ = driver.open_file(&fixture.probes.document);
            let probe = fixture.probes.completion.clone();
            bencher.iter(|| black_box(driver.completion(black_box(&probe))));
        },
    );
}

fn bench_definition(criterion: &mut Criterion, fixture: &LspFixture) {
    criterion.benchmark_group("lsp/definition").bench_function(
        BenchmarkId::from_parameter(fixture.project.fixture_label()),
        |bencher| {
            let driver = fixture.project.driver();
            let probe = fixture.probes.definition.clone();
            bencher.iter(|| black_box(driver.definition(black_box(&probe))));
        },
    );
}

fn bench_rename(criterion: &mut Criterion, fixture: &LspFixture) {
    criterion.benchmark_group("lsp/rename").bench_function(
        BenchmarkId::from_parameter(fixture.project.fixture_label()),
        |bencher| {
            let driver = fixture.project.driver();
            let probe = fixture.probes.rename.clone();
            bencher.iter(|| black_box(driver.rename(black_box(&probe), black_box("renamed"))));
        },
    );
}

fn bench_stale_change_suppression(criterion: &mut Criterion, fixture: &LspFixture) {
    criterion
        .benchmark_group("lsp/stale_change_suppression")
        .bench_function(
            BenchmarkId::from_parameter(fixture.project.fixture_label()),
            |bencher| {
                bencher.iter_batched(
                    || (fixture.project.driver(), fixture.probes.document.clone()),
                    |(mut driver, probe)| {
                        black_box(driver.stale_change_is_suppressed(black_box(&probe)))
                    },
                    BatchSize::SmallInput,
                );
            },
        );
}

#[derive(Clone)]
struct LspFixture {
    project: LspBenchmarkProject,
    probes: recite_lsp::bench_support::LspBenchmarkProbes,
}

fn load_lsp_projects() -> Vec<LspFixture> {
    must(load_lsp_projects_result())
}

fn load_lsp_projects_result() -> BenchmarkResult<Vec<LspFixture>> {
    BenchmarkFixture::selected_from_env()?
        .into_iter()
        .map(|fixture| {
            let project = BenchmarkProject::load_fixture(fixture)?;
            let lsp = LspBenchmarkProject::load(&project)?;
            let probes = lsp.probes();
            Ok(LspFixture {
                project: lsp,
                probes,
            })
        })
        .collect()
}

fn write_memory_report(fixture: &LspFixture) {
    let output = workspace_root().join("target/recite-benchmarks/lsp");
    must(fixture.project.write_memory_report(&output));
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn must<T>(result: BenchmarkResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("benchmark setup failed: {error}"),
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = lsp_benchmarks
}
criterion_main!(benches);
