use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
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
        bench_restore_parity(criterion, &fixture);
    }
}

fn bench_step(criterion: &mut Criterion, fixture: &PreviewFixture) {
    criterion.benchmark_group("preview/step").bench_function(
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
    criterion
        .benchmark_group("preview/full_traversal")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter(|| black_box(must(fixture.project.full_traversal())));
            },
        );
}

fn bench_snapshot_encode(criterion: &mut Criterion, fixture: &PreviewFixture) {
    criterion
        .benchmark_group("preview/snapshot_encode")
        .bench_function(
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
    criterion.benchmark_group("preview/restore").bench_function(
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
                    |preview| black_box(must(fixture.project.retention_report(&preview))),
                    BatchSize::SmallInput,
                );
            },
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
}

fn load_preview_projects() -> Vec<PreviewFixture> {
    must(load_preview_projects_result())
}

fn load_preview_projects_result() -> BenchmarkResult<Vec<PreviewFixture>> {
    BenchmarkFixture::selected_preview_from_env()?
        .into_iter()
        .map(|fixture| {
            Ok(PreviewFixture {
                fixture,
                project: PreviewProject::load(fixture)?,
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
