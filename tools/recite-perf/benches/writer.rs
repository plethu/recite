//! Cold script projection under typed effects and Unicode prose pressure.
//! Opening/validating the document and checking outputs happen outside counters.
#[path = "support/budget.rs"]
mod budget;
#[path = "support/writer.rs"]
mod writer_support;

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use writer_support::{ProjectedScript, ScriptWorkload, effects, validate_script};

#[library_benchmark(teardown = validate_script)]
#[bench::effects_1000(args = (1_000), setup = effects)]
#[bench::effects_10000(args = (10_000), setup = effects)]
fn cold_script(workload: ScriptWorkload) -> ProjectedScript {
    let script = workload
        .document
        .script_snapshot()
        .unwrap_or_else(|error| panic!("validated Writer script failed: {error}"));
    black_box(ProjectedScript { workload, script })
}

library_benchmark_group!(name = writer_workflows; config = budget::budget(20.0); benchmarks = cold_script);
main!(library_benchmark_groups = writer_workflows);
