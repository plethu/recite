use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use recite_benchmarks::compiler::CompilerProject;
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::runtime::RuntimeProject;
use recite_benchmarks::{BenchmarkFixture, BenchmarkResult};
use recite_core::compiled::{CompiledDialogue, decode_compiled_dialogue_messagepack};

fn runtime_benchmarks(criterion: &mut Criterion) {
    for fixture in load_runtime_projects() {
        bench_asset_decode(criterion, &fixture);
        bench_asset_preparation(criterion, &fixture);
        bench_start_scene(criterion, &fixture);
        bench_next_line(criterion, &fixture);
        bench_next_prompt(criterion, &fixture);
        bench_choose_first(criterion, &fixture);
        bench_condition_dispatch(criterion, &fixture);
        bench_effect_immediate(criterion, &fixture);
        bench_effect_deferred(criterion, &fixture);
        bench_effect_blocking_ack(criterion, &fixture);
        bench_localised_next(criterion, &fixture);
        bench_session_encode(criterion, &fixture);
        bench_session_decode(criterion, &fixture);
        bench_full_traversal(criterion, &fixture);
    }
}

fn bench_asset_decode(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/asset_decode")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                // Bytes are prepared once; the decoded asset is disposed after the
                // sample, matching asset_preparation's ownership boundary.
                bencher.iter_batched(
                    || (),
                    |()| {
                        black_box(
                            decode_compiled_dialogue_messagepack(black_box(&fixture.bytes))
                                .unwrap_or_else(|error| panic!("invalid benchmark asset: {error}")),
                        )
                    },
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_asset_preparation(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/asset_preparation")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                bencher.iter_batched(
                    || fixture.project.dialogue().clone().into_payload(),
                    |payload| {
                        let asset = match CompiledDialogue::prepare(black_box(payload)) {
                            Ok(asset) => asset,
                            Err(error) => panic!("invalid benchmark asset: {error}"),
                        };
                        black_box(asset)
                    },
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_start_scene(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/start_scene")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched(
                    || (),
                    |()| black_box(must(driver.start_scene())),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_next_line(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/next_line")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || must(driver.session_before_first_line()),
                    |session| black_box(must(driver.next_line(black_box(session)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_next_prompt(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/next_prompt")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || must(driver.session_before_first_prompt()),
                    |session| black_box(must(driver.next_prompt(black_box(session)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_choose_first(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/choose_first")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || must(driver.session_with_prompt()),
                    |session| black_box(must(driver.choose_first(black_box(session)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_condition_dispatch(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/condition_dispatch")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || must(driver.session_before_condition_prompt()),
                    |session| black_box(must(driver.condition_dispatch(black_box(session)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_effect_immediate(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/effect_immediate")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || must(driver.start_scene()),
                    |session| black_box(must(driver.immediate_effect(black_box(session)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_effect_deferred(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/effect_deferred")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || must(driver.session_before_deferred_effect()),
                    |session| black_box(must(driver.deferred_effect(black_box(session)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_effect_blocking_ack(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/effect_blocking_ack")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || {
                        let mut session = must(driver.session_before_blocking_effect());
                        let _event = must(driver.blocking_effect(&mut session));
                        session
                    },
                    |session| {
                        must(driver.acknowledge_blocking(session));
                        black_box(())
                    },
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_localised_next(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/localised_next")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || must(driver.localised_session_before_first_line()),
                    |session| black_box(must(driver.localised_next(black_box(session)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_session_encode(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/session_encode")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter_batched_ref(
                    || must(driver.session_with_prompt()),
                    |session| black_box(must(driver.encode_session(black_box(session)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_session_decode(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/session_decode")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                let bytes = must(driver.encoded_prompt_session());
                bencher.iter_batched_ref(
                    || bytes.clone(),
                    |bytes| black_box(must(driver.decode_session(black_box(bytes)))),
                    BatchSize::LargeInput,
                );
            },
        );
}

fn bench_full_traversal(criterion: &mut Criterion, fixture: &RuntimeFixture) {
    criterion
        .benchmark_group("runtime/full_traversal")
        .bench_function(
            BenchmarkId::from_parameter(fixture.fixture.as_str()),
            |bencher| {
                let driver = fixture.project.driver();
                bencher.iter(|| black_box(must(driver.full_traversal())));
            },
        );
}

#[derive(Clone)]
struct RuntimeFixture {
    fixture: BenchmarkFixture,
    project: RuntimeProject,
    bytes: Vec<u8>,
}

fn load_runtime_projects() -> Vec<RuntimeFixture> {
    must(load_runtime_projects_result())
}

fn load_runtime_projects_result() -> BenchmarkResult<Vec<RuntimeFixture>> {
    BenchmarkFixture::selected_from_env()?
        .into_iter()
        .map(|fixture| {
            let project = BenchmarkProject::load_fixture(fixture)?;
            let compiler = CompilerProject::load(&project)?;
            let compiled = compiler.compile_with_schema()?;
            let runtime = RuntimeProject::load(&project, &compiled)?;
            Ok(RuntimeFixture {
                fixture,
                project: runtime,
                bytes: compiled.asset().messagepack.clone(),
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
    targets = runtime_benchmarks
}
criterion_main!(benches);
