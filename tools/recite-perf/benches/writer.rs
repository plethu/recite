//! Cold script projection under typed effects and Unicode prose pressure.
//! Opening/validating the document and checking outputs happen outside counters.
#[path = "support/budget.rs"]
mod budget;
#[path = "support/completion.rs"]
mod completion;
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

#[library_benchmark(teardown = completion::validate)]
#[bench::wide(args = (20), setup = completion::changed)]
#[bench::dense(args = (2_000), setup = completion::changed)]
fn changed_draft(workload: completion::CompletionWorkload) -> completion::Completed {
    black_box(completion::complete(workload))
}

#[library_benchmark(teardown = completion::validate)]
#[bench::wide(args = (20), setup = completion::accepted)]
#[bench::dense(args = (2_000), setup = completion::accepted)]
fn accepted_draft(workload: completion::CompletionWorkload) -> completion::Completed {
    black_box(completion::complete(workload))
}

#[library_benchmark(teardown = completion::validate_replacement)]
#[bench::wide(args = (20), setup = completion::replacement)]
#[bench::dense(args = (2_000), setup = completion::replacement)]
fn completion_replacement(workload: completion::ReplacementWorkload) -> completion::Replaced {
    black_box(completion::replace(workload))
}
library_benchmark_group!(name = writer_workflows; config = budget::budget(20.0); benchmarks = cold_script, changed_draft, accepted_draft, completion_replacement);
main!(library_benchmark_groups = writer_workflows);
