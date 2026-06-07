use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use recite_benchmarks::compiler::{
    CompilerProject, compile_with_schema, extract_pot, lower_inputs, parse_inputs,
    validate_with_schema, validate_without_schema,
};
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::{BenchmarkFixture, BenchmarkResult};

fn compiler_benchmarks(criterion: &mut Criterion) {
    for fixture in load_compiler_projects() {
        bench_parse(criterion, &fixture);
        bench_lower(criterion, &fixture);
        bench_validate(criterion, &fixture);
        bench_validate_with_schema(criterion, &fixture);
        bench_compile_with_schema(criterion, &fixture);
        bench_extract_pot_with_schema(criterion, &fixture);
    }
}

fn bench_parse(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion.benchmark_group("compiler/parse").bench_function(
        BenchmarkId::from_parameter(fixture.fixture.as_str()),
        |bencher| {
            bencher.iter_batched(
                || fixture.project.compile_inputs(),
                |inputs| black_box(must(parse_inputs(black_box(&inputs)))),
                BatchSize::SmallInput,
            );
        },
    );
}

fn bench_lower(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion.benchmark_group("compiler/lower").bench_function(
        BenchmarkId::from_parameter(fixture.fixture.as_str()),
        |bencher| {
            bencher.iter_batched(
                || fixture.project.compile_inputs(),
                |inputs| black_box(must(lower_inputs(black_box(&inputs)))),
                BatchSize::SmallInput,
            );
        },
    );
}

fn bench_validate(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion
        .benchmark_group("compiler/validate")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || fixture.project.source_files(),
                    |sources| black_box(validate_without_schema(black_box(&sources))),
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_validate_with_schema(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion
        .benchmark_group("compiler/validate_with_schema")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || fixture.project.source_files(),
                    |sources| {
                        black_box(validate_with_schema(
                            black_box(&sources),
                            black_box(fixture.project.schema()),
                        ))
                    },
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_compile_with_schema(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion
        .benchmark_group("compiler/compile_with_schema")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || fixture.project.clone(),
                    |project| black_box(must(compile_with_schema(black_box(&project)))),
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_extract_pot_with_schema(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion
        .benchmark_group("compiler/extract_pot_with_schema")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || fixture.project.clone(),
                    |project| black_box(must(extract_pot(black_box(&project)))),
                    BatchSize::SmallInput,
                );
            },
        );
}

#[derive(Clone)]
struct CompilerFixture {
    fixture: BenchmarkFixture,
    project: CompilerProject,
}

fn load_compiler_projects() -> Vec<CompilerFixture> {
    must(load_compiler_projects_result())
}

fn load_compiler_projects_result() -> BenchmarkResult<Vec<CompilerFixture>> {
    BenchmarkFixture::selected_from_env()?
        .into_iter()
        .map(|fixture| {
            let project = BenchmarkProject::load_fixture(fixture)?;
            let compiler = CompilerProject::load(&project)?;
            Ok(CompilerFixture {
                fixture,
                project: compiler,
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

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = compiler_benchmarks
}
criterion_main!(benches);
