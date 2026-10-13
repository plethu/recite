//! Authored bound line/choice text; all prepared receivers and outputs stay live.
#[path = "support/bound_text.rs"]
mod bound_text;
#[path = "support/budget.rs"]
mod budget;

use std::hint::black_box;

use bound_text::BoundText;
use gungraun::{library_benchmark, library_benchmark_group, main};
use recite_runtime::{EmptyDialogueContext, LocaleResolution, next_with};

#[library_benchmark(teardown = bound_text::validate)]
#[bench::authored_64(setup = bound_text::authored)]
fn prompt(mut workload: BoundText) -> BoundText {
    for session in &mut workload.sessions {
        let event = next_with(
            &workload.asset,
            session,
            &EmptyDialogueContext,
            LocaleResolution::new().with_values(&workload.values),
        )
        .unwrap_or_else(|error| panic!("validated bound text failed: {error}"));
        workload.events.push(event);
    }
    black_box(workload)
}

library_benchmark_group!(name = bound_text_workflows; config = budget::budget(10.0); benchmarks = prompt);
main!(library_benchmark_groups = bound_text_workflows);
