mod support;
use criterion::measurement::{Measurement, WallTime};
use freya::prelude::*;
use freya_testing::prelude::*;
use std::time::Duration;

#[test]
#[ignore = "explicit large-scene timing workload"]
fn large_scene_keeps_mounted_nodes_bounded() -> Result<(), Box<dyn std::error::Error>> {
    let beats: usize = std::env::var("RECITE_BENCH_BEATS")
        .ok()
        .map(|v| v.parse())
        .transpose()?
        .unwrap_or(1000);
    let source = recite_writer_model::workload::source(beats, 0, 1);
    let started = WallTime.start();
    let mut test = TestingRunner::new(
        recite_writer::regression_app,
        Size2D::new(1440., 1000.),
        |runner| runner.provide_root_context(move || recite_writer::InitialSource(source)),
        1.,
    )
    .0;
    test.poll_n(Duration::from_millis(16), 6);
    let first_ms = WallTime.end(started).as_secs_f64() * 1000.;
    support::click(&mut test, "Map")?;
    let mounted = test
        .find_many(|_, element| {
            Rect::try_downcast(element)
                .filter(|rect| {
                    rect.accessibility
                        .builder
                        .label()
                        .is_some_and(|label| label.starts_with("Select "))
                })
                .map(|_| ())
        })
        .len();
    assert!(
        (1..100).contains(&mounted),
        "{mounted} mounted cards for {beats} beats"
    );
    support::open_beat(&mut test)?;
    support::click(&mut test, "Map")?;
    let selected = |test: &TestingRunner| {
        test.find(|_, element| {
            Rect::try_downcast(element).and_then(|r| {
                r.accessibility
                    .builder
                    .label()
                    .filter(|label| label.starts_with("Scene map. Selected"))
                    .map(str::to_owned)
            })
        })
    };
    let before = selected(&test);
    let mut frames = Vec::new();
    for _ in 0..30 {
        let start = WallTime.start();
        test.send_event(PlatformEvent::Keyboard {
            name: KeyboardEventName::KeyDown,
            key: Key::Named(NamedKey::ArrowDown),
            code: Code::ArrowDown,
            modifiers: Modifiers::empty(),
        });
        test.sync_and_update();
        frames.push(WallTime.end(start).as_secs_f64() * 1000.);
    }
    assert_ne!(
        selected(&test),
        before,
        "navigation benchmark must actually change selection"
    );
    println!(
        "{}",
        serde_json::json!({"beats": beats, "first_usable_ms":first_ms,
        "mounted_cards":mounted, "navigation_ms":frames, "host":std::env::consts::OS,
        "profile":"test", "native_frame_pacing":false})
    );
    Ok(())
}
