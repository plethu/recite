use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use recite_benchmarks::preview::PreviewProject;
use recite_benchmarks::{BenchmarkFixture, BenchmarkResult};
use recite_runtime::PreviewSnapshot;

fn preview_benchmarks(criterion: &mut Criterion) {
    for fixture in load_preview_projects() {
        bench_step(criterion, &fixture);
        bench_full_traversal(criterion, &fixture);
        bench_snapshot_encode(criterion, &fixture);
        bench_restore(criterion, &fixture);
        bench_retained_trace_shape(criterion, &fixture);
        bench_evidence_report(criterion, &fixture);
        bench_restore_parity(criterion, &fixture);
    }
}

fn bench_step(criterion: &mut Criterion, fixture: &PreviewFixture) {
    let mut group = criterion.benchmark_group("preview/step");
    group.throughput(Throughput::Elements(fixture.step_event_count));
    group.bench_function(
        BenchmarkId::from_parameter(fixture.fixture.as_str()),
        |bencher| {
            bencher.iter_batched(
                || must(fixture.project.start()),
                |mut preview| black_box(preview.step(fixture.project.inputs())),
                BatchSize::SmallInput,
            );
        },
    );
}

fn bench_full_traversal(criterion: &mut Criterion, fixture: &PreviewFixture) {
    let mut group = criterion.benchmark_group("preview/full_traversal");
    group.throughput(Throughput::Elements(fixture.event_count));
    group.bench_function(
        BenchmarkId::from_parameter(fixture.fixture.as_str()),
        |bencher| {
            bencher.iter(|| black_box(must(fixture.project.full_traversal_count())));
        },
    );
}

fn bench_snapshot_encode(criterion: &mut Criterion, fixture: &PreviewFixture) {
    let mut group = criterion.benchmark_group("preview/snapshot_encode");
    group.throughput(Throughput::Bytes(fixture.snapshot_bytes));
    group.bench_function(
        BenchmarkId::from_parameter(fixture.fixture.as_str()),
        |bencher| {
            bencher.iter_batched(
                || must(fixture.project.at_first_prompt()),
                |preview| {
                    let snapshot = must_preview(preview.snapshot());
                    black_box(must_preview(snapshot.encode()))
                },
                BatchSize::SmallInput,
            );
        },
    );
}

fn bench_restore(criterion: &mut Criterion, fixture: &PreviewFixture) {
    let mut group = criterion.benchmark_group("preview/restore");
    group.throughput(Throughput::Bytes(fixture.snapshot_bytes));
    group.bench_function(
        BenchmarkId::from_parameter(fixture.fixture.as_str()),
        |bencher| {
            bencher.iter_batched(
                || {
                    let preview = must(fixture.project.at_first_prompt());
                    let bytes = must_preview(must_preview(preview.snapshot()).encode());
                    (bytes, must(fixture.project.start()))
                },
                |(bytes, mut receiver)| {
                    let snapshot = must_preview(PreviewSnapshot::decode(black_box(&bytes)));
                    black_box(must_preview(receiver.restore(snapshot)))
                },
                BatchSize::SmallInput,
            );
        },
    );
}

fn bench_retained_trace_shape(criterion: &mut Criterion, fixture: &PreviewFixture) {
    criterion
        .benchmark_group("preview/retained_trace_shape")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || {
                        let mut preview = must(fixture.project.start());
                        must(fixture.project.collect_to_end(&mut preview));
                        preview
                    },
                    |preview| black_box(fixture.project.retained_trace_shape(&preview)),
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_evidence_report(criterion: &mut Criterion, fixture: &PreviewFixture) {
    criterion
        .benchmark_group("preview/evidence_report")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| bencher.iter(|| black_box(must(fixture.project.evidence_report()))),
        );
}

fn bench_restore_parity(criterion: &mut Criterion, fixture: &PreviewFixture) {
    criterion
        .benchmark_group("preview/restore_parity")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter(|| black_box(must(fixture.project.restore_parity())));
            },
        );
}

#[derive(Clone)]
struct PreviewFixture {
    fixture: BenchmarkFixture,
    project: PreviewProject,
    step_event_count: u64,
    event_count: u64,
    snapshot_bytes: u64,
}

fn load_preview_projects() -> Vec<PreviewFixture> {
    must(load_preview_projects_result())
}

fn load_preview_projects_result() -> BenchmarkResult<Vec<PreviewFixture>> {
    BenchmarkFixture::selected_preview_from_env()?
        .into_iter()
        .map(|fixture| {
            let project = PreviewProject::load(fixture)?;
            let step_event_count = project.start()?.step(project.inputs()).events().len() as u64;
            let event_count = project.full_traversal()?.event_count as u64;
            let snapshot_preview = project.at_first_prompt()?;
            let snapshot_bytes = snapshot_preview
                .snapshot()
                .map_err(|error| {
                    recite_benchmarks::BenchmarkError::Message(format!(
                        "preview setup failed: {error}"
                    ))
                })?
                .encode()
                .map_err(|error| {
                    recite_benchmarks::BenchmarkError::Message(format!(
                        "preview setup failed: {error}"
                    ))
                })?
                .len() as u64;
            Ok(PreviewFixture {
                fixture,
                project,
                step_event_count,
                event_count,
                snapshot_bytes,
            })
        })
        .collect()
}

fn must<T>(result: BenchmarkResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("benchmark setup failed: {error}"),
    }
}

fn must_preview<T>(result: Result<T, recite_runtime::PreviewError>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("preview benchmark setup failed: {error}"),
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = preview_benchmarks
}
criterion_main!(benches);
