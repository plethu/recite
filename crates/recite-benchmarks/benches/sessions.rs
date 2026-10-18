use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
#[path = "support/sessions.rs"]
mod sessions;

use sessions::{SESSION_HISTORY_SIZES, SESSION_SOURCE_COUNTS, SessionResult, SessionWorkload};

fn sessions(criterion: &mut Criterion) {
    for deferred in [false, true] {
        let workload = must(SessionWorkload::new(deferred));
        let label = if deferred { "deferred" } else { "history" };
        for history in SESSION_HISTORY_SIZES {
            let adapter = must(workload.adapter_checkpoint(history));
            let preview = must(workload.preview_checkpoint(history));
            criterion
                .benchmark_group("sessions/adapter_choice")
                .bench_function(BenchmarkId::new(label, history), |bencher| {
                    bencher.iter_batched_ref(
                        || must(workload.restore_adapter(&adapter)),
                        |driver| black_box(must(workload.adapter_cycle(driver))),
                        BatchSize::LargeInput,
                    );
                });
            criterion
                .benchmark_group("sessions/preview_condition_replay")
                .bench_function(BenchmarkId::new(label, history), |bencher| {
                    bencher.iter_batched_ref(
                        || must(workload.restore_preview(&preview)),
                        |session| black_box(must(workload.preview_cycle(session))),
                        BatchSize::LargeInput,
                    );
                });
        }
    }
}

fn must<T>(result: SessionResult<T>) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("invalid session benchmark: {error}"),
    }
}

fn source_counts(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("sessions/adapter_source_files");
    for count in SESSION_SOURCE_COUNTS {
        let workload = must(SessionWorkload::with_source_count(false, count));
        let checkpoint = must(workload.adapter_checkpoint(0));
        group.bench_function(BenchmarkId::from_parameter(count), |bencher| {
            bencher.iter_batched_ref(
                || must(workload.restore_adapter(&checkpoint)),
                |driver| black_box(must(workload.adapter_cycle(driver))),
                BatchSize::LargeInput,
            );
        });
    }
    group.finish();
}

fn snapshot_encoding(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("sessions/snapshot_encode");
    for sources in [1, 4_000] {
        for deferred in [false, true] {
            let workload = must(SessionWorkload::with_source_count(deferred, sources));
            for history in [0, 2_048] {
                let checkpoint = must(workload.adapter_checkpoint(history));
                let driver = must(workload.restore_adapter(&checkpoint));
                assert_eq!(must(driver.snapshot().map_err(Into::into)), checkpoint);
                let label = format!("sources-{sources}/deferred-{deferred}");
                group.bench_function(BenchmarkId::new(label, history), |bencher| {
                    bencher.iter_batched(
                        || (),
                        |()| black_box(must(driver.snapshot().map_err(Into::into))),
                        BatchSize::LargeInput,
                    );
                });
            }
        }
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = sessions, source_counts, snapshot_encoding
}
criterion_main!(benches);
