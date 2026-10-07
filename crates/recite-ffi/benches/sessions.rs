mod support;

use std::hint::black_box;
use std::time::Duration;

use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use support::{HISTORY_SIZES, HostFixture};

fn sessions(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("sessions/ffi_choice_and_encoding");
    for deferred in [false, true] {
        for history in HISTORY_SIZES {
            let fixture = HostFixture::new(history, deferred);
            group.bench_function(
                BenchmarkId::new(if deferred { "deferred" } else { "history" }, history),
                |bencher| {
                    bencher.iter_batched_ref(
                        || fixture.prepare(),
                        |session| black_box(session.choose()),
                        BatchSize::LargeInput,
                    )
                },
            );
        }
    }
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(1));
    targets = sessions
}
criterion_main!(benches);
