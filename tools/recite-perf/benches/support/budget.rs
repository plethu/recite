//! Limits apply to these fixed inputs and instrumentation, not elapsed time.
use gungraun::{Callgrind, Dhat, DhatMetric, EventKind, LibraryBenchmarkConfig};

pub(crate) fn budget(instructions_percent: f64) -> LibraryBenchmarkConfig {
    LibraryBenchmarkConfig::default()
        .tool(
            Callgrind::with_args(["--cache-sim=no"])
                .soft_limits([(EventKind::Ir, instructions_percent)]),
        )
        // Scoped allocations stay here. Whole-process peak heap is checked from DHAT logs:
        // filtering allocation sites cannot move the global peak timestamp into this function.
        .tool(Dhat::with_args(["--num-callers=100"]).soft_limits([
            (DhatMetric::TotalBytes, 10.0),
            (DhatMetric::TotalBlocks, 5.0),
        ]))
        .to_owned()
}
