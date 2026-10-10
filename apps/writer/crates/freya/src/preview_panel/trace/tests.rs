use super::*;
use freya_testing::prelude::*;
use recite_writer_model::{PreviewSetup, Workbench};

fn trace_app() -> impl IntoElement {
    use_init_theme(light_theme);
    let trace = use_consume::<PreviewTrace>();
    render(&trace)
}

#[test]
fn runtime_trace_keeps_typed_arguments_choice_identity_and_deferred_requests_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n:if accepts(actor, \"spoken\", 7, 2.5, true, false)\n  > line@11111111111111111111\n    Accepted.\n? reply@22222222222222222222\n  Go.\n  -> chosen\n:: chosen\n! immediate notify(actor, \"spoken\", 7, 2.5, true, false)\n! deferred mark()\n-> END\n";
    let mut model = Workbench::new(source)?;
    model.start_preview_with(PreviewSetup {
        policy: recite_compiler::authoring::CatalogResolutionPolicy::new(Some(
            recite_core::LocaleId::new("fr")?,
        )),
        ..PreviewSetup::default()
    })?;
    let request = model
        .preview_page()
        .and_then(|page| page.condition.as_ref())
        .ok_or("condition")?;
    assert_eq!(
        query(request.query()),
        "accepts(actor, spoken, 7, 2.5, True, False)"
    );
    model.answer_preview(ConditionValue::Bool(true))?;
    model.advance_preview(None)?;
    model.advance_preview(Some(0))?;
    let page = model.preview_page().ok_or("effects")?;
    assert_eq!(
        effect(page.effects.first().ok_or("immediate effect")?),
        "notify(actor, spoken, 7, 2.5, True, False)"
    );
    model.advance_preview(None)?;
    let trace = model.preview_trace().ok_or("trace")?.clone();
    let test = TestingRunner::new(
        trace_app,
        Size2D::new(1000., 1300.),
        |runner| {
            runner.provide_root_context(move || trace);
        },
        1.,
    )
    .0;
    for expected in [
        "accepts(actor, spoken, 7, 2.5, True, False) → True",
        "notify",
        "mark",
        "22222222222222222222",
    ] {
        assert!(
            test.find(|_, e| Label::try_downcast(e).filter(|label| label.text.contains(expected)))
                .is_some(),
            "missing {expected}"
        );
    }
    Ok(())
}
