use super::*;
use crate::{presentation::Theme, tests::support};
use recite_config::WriterPresentationField;
use std::sync::Arc;
use xilem::{
    ViewCtx,
    core::View as _,
    masonry::{
        accesskit::Role,
        testing::{TestHarness, TestHarnessParams},
    },
};

const SOURCE: &str = ":: start default\n> @ffee6p8\n  A repeated delivery.\n-> start\n";

#[test]
fn trace_render_cost_depends_on_visible_records_not_retained_history() {
    let mut measurements = Vec::new();
    for advances in [1_026, 10_260] {
        let directory = support::project(SOURCE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer
            .session
            .edit(|model| {
                model.start_preview()?;
                for _ in 0..advances {
                    model.advance_preview(None)?;
                }
                Ok(())
            })
            .unwrap();
        writer.trial.visible = true;
        writer.trial.trace.open = true;
        writer.trial.trace.place = Place::List { page: 10 };
        let mut context = ViewCtx::new(
            Arc::new(support::NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = sized_box(flex_col((panel(&writer, false),)));
        let (pod, _) = view.build(&mut context, &mut writer);
        let harness = TestHarness::create_with_size(
            Theme::new(&writer.config.writer).initial_properties(),
            pod.new_widget,
            (1000, 900),
        );
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let mut nodes = 0;
        let mut records = Vec::new();
        while let Some(node) = pending.pop() {
            nodes += 1;
            if node.role() == Role::Button
                && let Some(label) = node.label()
                && label.starts_with("Open trace record")
            {
                records.push(label);
            }
            pending.extend(node.children());
        }
        assert_eq!(records.len(), RECORDS);
        assert!(
            records
                .iter()
                .any(|caption| caption.starts_with("Open trace record 81 · Line delivered"))
        );
        measurements.push(nodes);
    }
    assert_eq!(
        measurements[0], measurements[1],
        "trace node counts {measurements:?}"
    );
    assert!(
        measurements.iter().all(|nodes| *nodes <= 88),
        "record controls stay within the measured budget"
    );
    eprintln!("trace node counts: {measurements:?}");
}

#[test]
fn selected_argument_render_cost_is_independent_of_argument_count_and_text_length() {
    let mut measurements = Vec::new();
    let mut record_measurements = Vec::new();
    for (arguments, length) in [(32, 10_000), (1_026, 200_000)] {
        let value = "駅🚉".repeat(length);
        let rest = vec!["actor"; arguments].join(", ");
        let source = format!(
            ":: start default\n:if accepts(\"{value}\", {rest})\n  > @ffee6p8\n    Accepted.\n-> END\n"
        );
        let directory = support::project(&source).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer.session.edit(|model| model.start_preview()).unwrap();
        let record = writer
            .session
            .model()
            .preview_trace()
            .unwrap()
            .events()
            .len()
            - 1;
        writer.trial.visible = true;
        writer.trial.trace.open = true;
        for place in [
            Place::Value(ValuePosition {
                record,
                field: 4,
                part: 3,
            }),
            Place::Record {
                index: record,
                page: 0,
            },
        ] {
            writer.trial.trace.place = place;
            let (nodes, values) = rendered_values(&mut writer);
            assert!(
                values
                    .iter()
                    .any(|text| text.starts_with("Argument 1 · String")),
                "{values:?}"
            );
            if matches!(place, Place::Record { .. }) {
                assert!(values.iter().any(|text| text == "Function"));
                assert!(values.iter().any(|text| text == "accepts"));
                assert!(values.iter().any(|text| text == "Argument 2 · Identifier"));
                assert!(values.iter().all(|text| text.len() <= 163));
                record_measurements.push(nodes);
            } else {
                let part = Section::Events
                    .record(writer.session.model().preview_trace().unwrap(), record)
                    .unwrap()
                    .field(4)
                    .unwrap()
                    .part(3);
                assert!(value.contains(&part.value));
                assert!(!part.value.is_empty() && part.value.len() <= 1_003);
                assert!(values.iter().all(|text| text.len() <= 1_003));
                measurements.push(nodes);
            }
        }
    }
    assert_eq!(
        measurements[0], measurements[1],
        "argument node counts: {measurements:?}"
    );
    assert!(
        measurements.iter().all(|nodes| *nodes <= 72),
        "field controls stay within the measured budget"
    );
    eprintln!("argument node counts: {measurements:?}");
    assert_eq!(
        record_measurements[0], record_measurements[1],
        "record node counts: {record_measurements:?}"
    );
    assert!(record_measurements.iter().all(|nodes| *nodes <= 128));
    eprintln!("argument record node counts: {record_measurements:?}");
}

fn rendered_values(writer: &mut Writer) -> (usize, Vec<String>) {
    let mut context = ViewCtx::new(
        Arc::new(support::NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = sized_box(flex_col((panel(writer, false),)));
    let (pod, _) = view.build(&mut context, writer);
    let harness = TestHarness::create_with_size(
        Theme::new(&writer.config.writer).initial_properties(),
        pod.new_widget,
        (1000, 900),
    );
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut nodes = 0;
    let mut values = Vec::new();
    while let Some(node) = pending.pop() {
        nodes += 1;
        if let Some(value) = node.value() {
            values.push(value.to_owned());
        }
        pending.extend(node.children());
    }
    (nodes, values)
}

#[test]
fn trace_list_record_and_complete_value_render_at_personal_scales() {
    let argument = "A captured argument remains fully inspectable. ".repeat(80);
    let source = format!(
        ":: start default\n:if accepts(\"{argument}\")\n  > @ffee6p8\n    Accepted.\n-> END\n"
    );
    for scale in [100, 200] {
        for (stage, place) in [
            ("list", Place::List { page: 0 }),
            ("record", Place::Record { index: 0, page: 0 }),
            (
                "value",
                Place::Value(ValuePosition {
                    record: 0,
                    field: 4,
                    part: 1,
                }),
            ),
        ] {
            let directory = support::project(&source).unwrap();
            let mut writer = support::open(&directory).unwrap();
            writer.config.writer.presentation = writer
                .config
                .writer
                .presentation
                .with_value(WriterPresentationField::UiScale, scale)
                .unwrap();
            writer.session.edit(|model| model.start_preview()).unwrap();
            writer.trial.trace.open = true;
            writer.trial.trace.place = place;
            let mut context = ViewCtx::new(
                Arc::new(support::NoProxy),
                Arc::new(tokio::runtime::Runtime::new().unwrap()),
            );
            let view = sized_box(flex_col((panel(&writer, false),))).padding(24.px());
            let (pod, _) = view.build(&mut context, &mut writer);
            let mut harness = TestHarness::create_with(
                Theme::new(&writer.config.writer).initial_properties(),
                pod.new_widget,
                TestHarnessParams::default()
                    .with_size((1000, 1800))
                    .with_background(writer.palette.canvas),
            );
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_TRACE_NATIVE") {
                harness
                    .render()
                    .save(format!("{prefix}-{scale}-{stage}.png"))
                    .unwrap();
            }
        }
    }
}
