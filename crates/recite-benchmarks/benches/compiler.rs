use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use recite_benchmarks::compiler::{
    CompiledProject, CompilerProject, compile_with_schema, extract_pot, lower_inputs, parse_inputs,
    resolve_block_references, serialize_compiled_asset, validate_localisable_id_uniqueness,
    validate_markup, validate_with_schema, validate_without_schema,
};
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::{BenchmarkFixture, BenchmarkResult};

fn compiler_benchmarks(criterion: &mut Criterion) {
    for fixture in load_compiler_projects() {
        bench_parse(criterion, &fixture);
        bench_lower(criterion, &fixture);
        bench_validate(criterion, &fixture);
        bench_validate_with_schema(criterion, &fixture);
        bench_block_reference_resolution(criterion, &fixture);
        bench_id_uniqueness(criterion, &fixture);
        bench_markup_validation(criterion, &fixture);
        bench_compile_with_schema(criterion, &fixture);
        bench_pot_extraction_pressure(criterion, &fixture);
        bench_compiled_asset_serialization(criterion, &fixture);
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

fn bench_block_reference_resolution(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion
        .benchmark_group("compiler/block_reference_resolution")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || fixture.project.source_files(),
                    |sources| black_box(resolve_block_references(black_box(&sources))),
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_id_uniqueness(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion
        .benchmark_group("compiler/id_uniqueness")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || fixture.project.source_files(),
                    |sources| black_box(validate_localisable_id_uniqueness(black_box(&sources))),
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_markup_validation(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion
        .benchmark_group("compiler/markup_validation")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || fixture.project.source_files(),
                    |sources| {
                        black_box(validate_markup(
                            black_box(&sources),
                            black_box(fixture.project.schema()),
                        ))
                    },
                    BatchSize::SmallInput,
                );
            },
        );
}

fn bench_pot_extraction_pressure(criterion: &mut Criterion, fixture: &CompilerFixture) {
    criterion
        .benchmark_group("compiler/pot_extraction_pressure")
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

fn bench_compiled_asset_serialization(criterion: &mut Criterion, fixture: &CompilerFixture) {
    let bytes = fixture.compiled.asset().messagepack.len() as u64;
    let mut group = criterion.benchmark_group("compiler/compiled_asset_serialization");
    group.throughput(Throughput::Bytes(bytes));
    group.bench_function(
        BenchmarkId::from_parameter(fixture.fixture.as_str()),
        |bencher| {
            bencher.iter(|| {
                black_box(must(serialize_compiled_asset(black_box(&fixture.compiled))));
            });
        },
    );
    group.finish();
}

#[derive(Clone)]
struct CompilerFixture {
    fixture: BenchmarkFixture,
    project: CompilerProject,
    compiled: CompiledProject,
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
            let compiled = compiler.compile_with_schema()?;
            Ok(CompilerFixture {
                fixture,
                project: compiler,
                compiled,
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
