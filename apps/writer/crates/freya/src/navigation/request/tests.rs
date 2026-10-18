use super::*;
use crate::navigation::worker::{Destination, Example, Prepare, PreparedDestination};
use freya::prelude::*;
use freya_testing::prelude::*;

#[test]
fn rejected_navigation_retires_a_completed_destination_with_its_origins() {
    fn app() -> impl IntoElement {
        let mut writer = crate::tests::support::writer();
        use_hook(move || {
            let mut navigation = Request {
                target: Target::Location(crate::navigation::snapshot(writer)),
                cancelled: None,
            }
            .begin(writer)
            .unwrap();
            let worker = Worker::for_test();
            let prepared = prepare_example(&worker);
            let PreparedDestination::Example {
                prepared: Example::Cold(Some(model)),
                ..
            } = &prepared.destination
            else {
                panic!("cold destination")
            };
            let source = std::sync::Arc::downgrade(&model.document().source_snapshot());
            navigation.prepared = Some(prepared);
            assert!(source.upgrade().is_some());
            writer.settings_open.set(true);
            let result =
                futures_lite::future::block_on(execute(writer, Box::new(navigation), &worker));
            assert!(result.is_err());
            assert!(
                source.upgrade().is_none(),
                "rejection must retire the completed destination too"
            );
        });
        label().text("Completed navigation receipt retired")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}

fn prepare_example(worker: &Worker) -> Box<worker::Prepared> {
    let preparation = Prepare {
        destination: Destination::Example {
            name: "last_tram_linear.recite".into(),
            retained: None,
        },
        location: None,
        project: None,
        catalogue: None,
        reuse_rules: false,
        initial_view: None,
    };
    let work = Work::Navigation(Box::new(worker::Work::Prepare(Box::new(preparation))));
    let output = futures_lite::future::block_on(worker.execute(work)).unwrap();
    let Output::Navigation(worker::Output::Prepared(result)) = output else {
        panic!("preparation receipt")
    };
    result.unwrap()
}
