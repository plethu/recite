use super::*;
use crate::{preferences::Preferences, presentation::Typography};
use freya_testing::prelude::*;
use recite_compiler::authoring::CatalogResolutionPolicy;
use recite_config::{WriterPresentation, WriterPresentationField};
use recite_core::LocaleId;
use recite_runtime::preview::PreviewEvent;
use recite_writer_model::{ConditionValue, PreviewSetup, Workbench};
use recite_writer_session::preview::trace::FIELDS;

fn trace_app() -> impl IntoElement {
    use_init_theme(light_theme);
    let trace = use_consume::<PreviewTrace>();
    let initial = use_consume::<Browser>();
    let browser = use_state(move || initial);
    let presentation = use_consume::<WriterPresentation>();
    let preferences = use_state(move || {
        let mut preferences = Preferences::load(None);
        preferences.config.writer.presentation = presentation;
        preferences
    });
    use_provide_context(move || Typography(preferences));
    ScrollView::new()
        .width(Size::fill())
        .height(Size::fill())
        .child(rect().padding(24.).child(render(&trace, browser)))
}

#[test]
fn runtime_trace_keeps_typed_arguments_choice_identity_and_deferred_requests_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n:if accepts(actor, \"spoken\", 7, 2.5, true, false)\n  > line@11111111111111111111\n    Accepted.\n? reply@22222222222222222222\n  Go.\n  -> chosen\n:: chosen\n! immediate notify(actor, \"spoken\", 7, 2.5, true, false)\n! deferred mark()\n-> END\n";
    let mut model = Workbench::new(source)?;
    model.start_preview_with(PreviewSetup {
        policy: CatalogResolutionPolicy::new(Some(LocaleId::new("fr")?)),
        ..PreviewSetup::default()
    })?;
    let request = model
        .preview_page()
        .and_then(|page| page.condition.as_ref())
        .ok_or("condition")?;
    assert_eq!(
        query(request.query()),
        "accepts(actor, \"spoken\", 7, 2.5, true, false)"
    );
    model.answer_preview(ConditionValue::Bool(true))?;
    model.advance_preview(None)?;
    model.advance_preview(Some(0))?;
    let page = model.preview_page().ok_or("effects")?;
    assert_eq!(
        effect(page.effects.first().ok_or("immediate effect")?),
        "notify(actor, \"spoken\", 7, 2.5, true, false)"
    );
    model.advance_preview(None)?;
    let trace = model.preview_trace().ok_or("trace")?.clone();
    for scale in [100, 200] {
        let presentation =
            WriterPresentation::default().with_value(WriterPresentationField::UiScale, scale)?;
        for (kind, field, expected) in [
            ("ConditionRequested", 5, "spoken"),
            ("EffectRequested", 3, "notify"),
            ("DeferredEffectScheduled", 3, "mark"),
            ("ChoiceSelected", 1, "22222222222222222222"),
        ] {
            let record = trace
                .events()
                .iter()
                .position(|event| {
                    matches!(
                        (kind, event),
                        ("ConditionRequested", PreviewEvent::ConditionRequested(_))
                            | ("EffectRequested", PreviewEvent::EffectRequested(_))
                            | (
                                "DeferredEffectScheduled",
                                PreviewEvent::DeferredEffectScheduled(_)
                            )
                            | ("ChoiceSelected", PreviewEvent::ChoiceSelected { .. })
                    )
                })
                .ok_or(kind)?;
            let trace = trace.clone();
            let mut test = TestingRunner::new(
                trace_app,
                Size2D::new(1000., 900.),
                |runner| {
                    runner.provide_root_context(move || trace);
                    runner.provide_root_context(move || presentation);
                    runner.provide_root_context(move || Browser {
                        open: true,
                        place: Place::Record {
                            index: record,
                            page: field / FIELDS,
                        },
                        ..Default::default()
                    });
                },
                1.,
            )
            .0;
            assert!(
                test.find(
                    |_, e| Label::try_downcast(e).filter(|label| label.text.contains(expected))
                )
                .is_some(),
                "missing {expected}"
            );
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_TRACE_FREYA") {
                test.render_to_file(format!("{prefix}-{scale}-{kind}.png"));
            }
        }
    }
    Ok(())
}

#[test]
fn trace_list_record_and_complete_value_render_at_personal_scales()
-> Result<(), Box<dyn std::error::Error>> {
    let argument = "A captured argument remains fully inspectable. ".repeat(80);
    let source = format!(
        ":: start default\n:if accepts(\"{argument}\")\n  > line@11111111111111111111\n    Accepted.\n-> END\n"
    );
    let mut model = Workbench::new(&source)?;
    model.start_preview()?;
    for scale in [100, 200] {
        let presentation =
            WriterPresentation::default().with_value(WriterPresentationField::UiScale, scale)?;
        for (stage, place, expected) in [
            ("list", Place::List { page: 0 }, "Open trace record 1"),
            (
                "record",
                Place::Record { index: 0, page: 0 },
                "Read complete trace value",
            ),
            (
                "value",
                Place::Value(ValuePosition {
                    record: 0,
                    field: 4,
                    part: 1,
                }),
                "Trace value part 2",
            ),
        ] {
            let trace = model.preview_trace().ok_or("trace")?.clone();
            let mut test = TestingRunner::new(
                trace_app,
                Size2D::new(1000., 1800.),
                |runner| {
                    runner.provide_root_context(move || trace);
                    runner.provide_root_context(move || presentation);
                    runner.provide_root_context(move || Browser {
                        open: true,
                        place,
                        ..Default::default()
                    });
                },
                1.,
            )
            .0;
            assert!(
                test.find(
                    |_, e| Label::try_downcast(e).filter(|label| label.text.contains(expected))
                )
                .is_some(),
                "missing {expected}"
            );
            if let Ok(prefix) = std::env::var("RECITE_CAPTURE_TRACE_FREYA") {
                test.render_to_file(format!("{prefix}-{scale}-{stage}.png"));
            }
        }
    }
    Ok(())
}
