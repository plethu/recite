use recite_compiler::authoring::{CancellationToken, CatalogInput, CatalogResolutionPolicy};
use recite_core::{LocaleId, ScalarValue, po::PoDocument};
use recite_runtime::{ConditionValue, preview::PreviewTrace};
use recite_writer_model::{PreviewSetup, Workbench};
use recite_writer_session::preview::trace::{Browser, FIELDS, Place, Projection};
use recite_writer_session::preview::{
    Inputs, Request,
    trace::{Section, condition, effect},
};
use std::collections::BTreeMap;

fn fields(
    trace: &PreviewTrace,
    section: Section,
    record: usize,
) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
    let record = section
        .record(trace, record)
        .ok_or("missing trace record")?;
    let mut fields = Vec::new();
    for index in 0..record.field_count() {
        let field = record.field(index).ok_or("missing trace field")?;
        let value = field.part(0).value;
        fields.push((field.name.into_owned(), value));
    }
    Ok(fields)
}
const SOURCE: &str = ":: start default\n:if accepts(actor, \"spoken\", 7, 2.5, true, false)\n  > line@11111111111111111111\n    Accepted.\n? reply@22222222222222222222\n  Go.\n  -> chosen\n:: chosen\n:if follows()\n  > after@33333333333333333333\n    Afterwards.\n! immediate notify(actor, \"spoken\", 7, 2.5, true, false)\n! deferred mark()\n-> END\n";

#[test]
fn record_pages_keep_function_and_typed_arguments_together_and_clamp_obsolete_cursors() {
    let mut model = Workbench::new(SOURCE).unwrap();
    model.start_preview().unwrap();
    let mut browser = Browser {
        open: true,
        place: Place::Record { index: 0, page: 0 },
        ..Default::default()
    };
    let Projection::Record(page) = browser.project(model.preview_trace().unwrap()) else {
        panic!("record page")
    };
    assert_eq!(page.fields.len(), FIELDS);
    assert!(page.summary.contains("accepts"));
    for (name, value) in [
        ("Function", "accepts"),
        ("Argument 1 · Identifier", "actor"),
        ("Argument 2 · String", "spoken"),
    ] {
        assert!(
            page.fields
                .iter()
                .any(|field| field.name == name && field.value == value)
        );
    }
    browser.place = Place::Record {
        index: usize::MAX,
        page: usize::MAX,
    };
    let Projection::Record(page) = browser.project(model.preview_trace().unwrap()) else {
        panic!("clamped page")
    };
    assert_eq!(page.page, page.pages - 1);
    assert!(
        page.fields
            .iter()
            .any(|field| field.name == "Argument 6 · Boolean" && field.value == "false")
    );
    browser.restart();
    assert!(
        matches!(browser.project(model.preview_trace().unwrap()), Projection::List(page) if page.page == 0)
    );
}

#[test]
fn typed_arguments_and_pending_versus_committed_choices_remain_distinct() {
    let mut model = Workbench::new(SOURCE).unwrap();
    let request = Request::capture(&model, Inputs::default(), None).unwrap();
    assert!(
        model
            .accept_preview(request.prepare(&CancellationToken::new()).unwrap())
            .is_ok()
    );
    let query = model
        .preview_page()
        .unwrap()
        .condition
        .as_ref()
        .unwrap()
        .query();
    assert_eq!(
        condition(query),
        "accepts(actor, \"spoken\", 7, 2.5, true, false)"
    );
    let trace = model.preview_trace().unwrap();
    let index = Section::Events.len(trace) - 1;
    let arguments = fields(trace, Section::Events, index).unwrap();
    for (kind, value) in [
        ("Argument 1 · Identifier", "actor"),
        ("Argument 2 · String", "spoken"),
        ("Argument 3 · Integer", "7"),
        ("Argument 4 · Float", "2.5"),
        ("Argument 5 · Boolean", "true"),
        ("Argument 6 · Boolean", "false"),
    ] {
        assert!(arguments.contains(&(kind.into(), value.into())));
    }
    model.answer_preview(ConditionValue::Bool(true)).unwrap();
    model.advance_preview(None).unwrap();
    model.advance_preview(Some(0)).unwrap();
    let trace = model.preview_trace().unwrap();
    let captions: Vec<_> = (0..Section::Events.len(trace))
        .map(|index| Section::Events.record(trace, index).unwrap().caption())
        .collect();
    assert!(captions.contains(&"Choice accepted; traversal pending"));
    assert!(!captions.contains(&"Choice selected"));
    model.answer_preview(ConditionValue::Bool(true)).unwrap();
    model.advance_preview(None).unwrap();
    assert_eq!(
        effect(model.preview_page().unwrap().effects.first().unwrap()),
        "notify(actor, \"spoken\", 7, 2.5, true, false)"
    );
    model.advance_preview(None).unwrap();
    let trace = model.preview_trace().unwrap();
    for caption in [
        "Choice selected",
        "Effect requested",
        "Deferred effect scheduled",
        "Conversation ended",
    ] {
        assert!(
            (0..Section::Events.len(trace)).any(|index| Section::Events
                .record(trace, index)
                .unwrap()
                .caption()
                == caption),
            "missing {caption}"
        );
    }
}

#[test]
fn localisation_and_plural_details_preserve_fallback_steps_and_selected_form() {
    let source = ":: start default\n> hello@11111111111111111111\n  Hello.\n> tickets@22222222222222222222 bind=(count:int=$tickets)\n  {count} ticket\n  | {count} tickets\n-> END\n";
    let po = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"{count} billet\"\nmsgstr[1] \"{count} billets\"\n";
    let mut model = Workbench::new(source).unwrap();
    model
        .start_preview_with(PreviewSetup {
            policy: CatalogResolutionPolicy::new(Some(LocaleId::new("fr-CA").unwrap()))
                .with_variant("formal")
                .unwrap(),
            catalogues: vec![
                CatalogInput::from_document(
                    LocaleId::new("fr").unwrap(),
                    PoDocument::parse(po).unwrap(),
                )
                .unwrap(),
            ],
            values: BTreeMap::from([("tickets".into(), ScalarValue::Integer(2))]),
            ..PreviewSetup::default()
        })
        .unwrap();
    let trace = model.preview_trace().unwrap();
    assert_eq!(Section::Localisation.len(trace), 1);
    let lookup = fields(trace, Section::Localisation, 0).unwrap();
    for (key, value) in [
        ("Resolved text", "Bonjour."),
        ("Matched language", "fr"),
        ("Attempt 1 · Outcome", "Missing entry"),
    ] {
        assert!(lookup.contains(&(key.into(), value.into())), "{lookup:?}");
    }
    model.advance_preview(None).unwrap();
    let plural = fields(model.preview_trace().unwrap(), Section::Plurals, 0).unwrap();
    assert!(plural.contains(&("Count".into(), "2".into())));
    assert!(plural.contains(&("Selected plural form".into(), "2".into())));
    assert!(
        plural
            .iter()
            .any(|(name, value)| name.starts_with("Attempt ")
                && name.ends_with("Outcome")
                && value == "Matched")
    );
    assert!(plural.contains(&("Matched language".into(), "fr".into())));
}
