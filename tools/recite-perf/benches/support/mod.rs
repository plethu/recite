//! Gungraun owns counters, baselines, comparison and regression exit codes.
mod budget;
pub(super) use budget::budget;
use recite_benchmarks::compiler::{CompiledProject, CompilerProject};
use recite_benchmarks::project::BenchmarkProject;
use recite_benchmarks::runtime::RuntimeProject;
use recite_benchmarks::{BenchmarkFixture, BenchmarkResult};
use recite_runtime::{DialogueEvent, DialogueSession};

pub(super) type Steps = (RuntimeProject, Vec<DialogueSession>, Vec<DialogueEvent>);
const STEP_OPERATIONS: usize = 64;

fn steps(project: RuntimeProject, session: DialogueSession) -> Steps {
    // Independent prepared operations dilute one-off allocator housekeeping.
    (
        project,
        vec![session; STEP_OPERATIONS],
        Vec::with_capacity(STEP_OPERATIONS),
    )
}

pub(super) fn validate_steps((_, sessions, events): Steps) {
    assert_eq!(sessions.len(), STEP_OPERATIONS);
    assert_eq!(events.len(), STEP_OPERATIONS);
    assert!(events.windows(2).all(|pair| pair[0] == pair[1]));
}

pub(super) fn compiler(fixture: BenchmarkFixture) -> CompilerProject {
    must(CompilerProject::load(&must(
        BenchmarkProject::load_fixture(fixture),
    )))
}

pub(super) fn compiled(fixture: BenchmarkFixture) -> CompiledProject {
    must(compiler(fixture).compile_with_schema())
}

pub(super) fn runtime(fixture: BenchmarkFixture) -> RuntimeProject {
    let project = must(BenchmarkProject::load_fixture(fixture));
    let compiled = must(must(CompilerProject::load(&project)).compile_with_schema());
    must(RuntimeProject::load(&project, &compiled))
}

pub(super) fn line(fixture: BenchmarkFixture) -> Steps {
    let runtime = runtime(fixture);
    let session = must(runtime.driver().session_before_first_line());
    steps(runtime, session)
}

pub(super) fn localised_line(fixture: BenchmarkFixture) -> Steps {
    let runtime = runtime(fixture);
    let session = must(runtime.driver().localised_session_before_first_line());
    steps(runtime, session)
}

pub(super) fn prompt(fixture: BenchmarkFixture) -> Steps {
    let runtime = runtime(fixture);
    let session = must(runtime.driver().session_with_prompt());
    steps(runtime, session)
}

pub(super) fn bytes(fixture: BenchmarkFixture) -> Vec<u8> {
    compiled(fixture).asset().messagepack.clone()
}

pub(super) fn must<T>(result: BenchmarkResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("validated benchmark workload failed: {error}"),
    }
}
