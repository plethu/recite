use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{View, Workbench};
use recite_writer_session::{
    localisation::catalogue::{Catalogue, Draft},
    preview::{Inputs, Request},
};
use std::{collections::BTreeMap, fs};

const SOURCE: &str = ":: start default\n> bind=(count:int=$tickets) @tickets\n  {count} ticket\n  | {count} tickets\n-> END\n";
const PO: &str = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"tickets\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"{count} billet\"\nmsgstr[1] \"{count} billets\"\n";

fn inputs(drafts: bool) -> Inputs {
    Inputs {
        locale: "fr-CA".into(),
        variant: "formal".into(),
        include_drafts: drafts,
        values: Some(BTreeMap::from([("tickets".into(), "2".into())])),
    }
}
fn play(model: &mut Workbench, request: &Request) -> Result<String, Box<dyn std::error::Error>> {
    let prepared = request.prepare(&CancellationToken::new())?;
    assert!(model.accept_preview(prepared).is_ok());
    Ok(model
        .preview_page()
        .ok_or("Prepared trial has no page")?
        .text
        .clone())
}

#[test]
fn localised_trial_captures_drafts_values_and_wording_without_saving_or_reviewing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fr.po");
    fs::write(&path, PO).unwrap();
    let mut catalogue = Catalogue::open(&path).unwrap();
    let mut model = Workbench::new(SOURCE).unwrap();
    let id = catalogue
        .document
        .entries()
        .iter()
        .find(|entry| entry.context() == Some("tickets"))
        .unwrap()
        .id();
    catalogue.update(
        id,
        Draft {
            forms: vec!["{count} place".into(), "{count} places".into()],
            reviewed: false,
        },
    );
    let saved = Request::capture(&model, inputs(false), Some(&catalogue)).unwrap();
    let drafts = Request::capture(&model, inputs(true), Some(&catalogue)).unwrap();
    assert_eq!(play(&mut model, &saved).unwrap(), "2 billets");
    assert_eq!(play(&mut model, &drafts).unwrap(), "2 places");
    assert!(
        model
            .preview_provenance()
            .unwrap()
            .caption
            .contains("fr-CA")
    );
    catalogue.update(
        id,
        Draft {
            forms: vec!["{count} siège".into(), "{count} sièges".into()],
            reviewed: true,
        },
    );
    assert!(!drafts.is_current(&model, Some(&catalogue)));
    assert_eq!(
        play(&mut model, &drafts).unwrap(),
        "2 places",
        "capture stays immutable"
    );
    let mut next = inputs(true);
    next.values
        .as_mut()
        .unwrap()
        .insert("tickets".into(), "1".into());
    let next = Request::capture(&model, next, Some(&catalogue)).unwrap();
    assert_eq!(play(&mut model, &next).unwrap(), "1 siège");
    assert_eq!(fs::read_to_string(&path).unwrap(), PO);
    assert!(catalogue.dirty());
}

#[test]
fn freshness_distinguishes_source_only_catalogue_replacement_view_and_field_drafts() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fr.po");
    fs::write(&path, PO).unwrap();
    let mut catalogue = Catalogue::open(&path).unwrap();
    let mut model = Workbench::new(SOURCE).unwrap();
    let localised = Request::capture(&model, inputs(false), Some(&catalogue)).unwrap();
    let source = Request::capture(&model, Inputs::default(), Some(&catalogue)).unwrap();
    assert!(source.is_current(&model, Some(&catalogue)));
    assert!(localised.is_current(&model, Some(&catalogue)));
    assert!(!localised.is_current(&model, None));
    assert!(source.is_current(&model, None));
    catalogue.reload().unwrap();
    assert!(!localised.is_current(&model, Some(&catalogue)));
    assert!(source.is_current(&model, Some(&catalogue)));
    let reopened = Workbench::new(SOURCE).unwrap();
    assert!(!source.is_current(&reopened, Some(&catalogue)));
    model.select(View::Source).unwrap();
    assert!(!source.is_current(&model, Some(&catalogue)));
    let source = Request::capture(&model, Inputs::default(), None).unwrap();
    model.set_draft(format!("{SOURCE}# unapplied\n"));
    assert!(!source.is_current(&model, None));
    assert!(Request::capture(&model, Inputs::default(), None).is_err());
}

#[test]
fn values_parse_as_declared_scalars_and_bad_inputs_leave_the_previous_run_intact() {
    let source = ":: start default\n> bind=(name:string=$person) bind=(count:int=$count) bind=(ratio:float=$ratio) bind=(ready:bool=$ready) @ffee6p8\n  {name}: {count}, {ratio}, {ready}.\n-> END\n";
    let mut model = Workbench::new(source).unwrap();
    let mut values = BTreeMap::from([
        ("person".into(), "Renée".into()),
        ("count".into(), "7".into()),
        ("ratio".into(), "1.5".into()),
        ("ready".into(), "true".into()),
    ]);
    let capture = |values| {
        Request::capture(
            &model,
            Inputs {
                values: Some(values),
                ..Inputs::default()
            },
            None,
        )
        .unwrap()
    };
    let request = capture(values.clone());
    assert_eq!(play(&mut model, &request).unwrap(), "Renée: 7, 1.5, true.");
    for (name, invalid) in [
        ("count", "1.2"),
        ("ratio", "NaN"),
        ("ratio", "inf"),
        ("ready", "yes"),
    ] {
        let before = values.insert(name.into(), invalid.into()).unwrap();
        let request = Request::capture(
            &model,
            Inputs {
                values: Some(values.clone()),
                ..Inputs::default()
            },
            None,
        )
        .unwrap();
        let error = request.prepare(&CancellationToken::new()).err().unwrap();
        assert!(error.contains(name), "{error}");
        assert_eq!(model.preview_page().unwrap().text, "Renée: 7, 1.5, true.");
        values.insert(name.into(), before);
    }
}

#[test]
fn cancellation_precedes_catalogue_io_and_invalid_wording_is_reported() {
    let model = Workbench::new(SOURCE).unwrap();
    let request = Request::capture(
        &model,
        Inputs {
            variant: "invalid&variant".into(),
            ..Inputs::default()
        },
        None,
    )
    .unwrap();
    assert!(request.prepare(&CancellationToken::new()).is_err());
    let cancelled = CancellationToken::new();
    cancelled.interrupt();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fr.po");
    fs::write(&path, PO).unwrap();
    let catalogue = Catalogue::open(&path).unwrap();
    let request = Request::capture(&model, inputs(false), Some(&catalogue)).unwrap();
    fs::remove_file(&path).unwrap();
    let error = request.prepare(&cancelled).err().unwrap();
    assert!(error.contains("interrupted"), "{error}");
    let error = request.prepare(&CancellationToken::new()).err().unwrap();
    assert!(!error.contains("interrupted"), "{error}");
}

#[test]
fn requested_wording_selects_its_own_catalogue_entry() {
    let source = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n";
    let po = "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\nmsgctxt \"ffee6p8\"\nmsgid \"Hello.\"\nmsgstr \"Salut.\"\n\nmsgctxt \"ffee6p8&formal\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fr.po");
    fs::write(&path, po).unwrap();
    let catalogue = Catalogue::open(&path).unwrap();
    let mut model = Workbench::new(source).unwrap();
    let mut staged = Inputs {
        locale: "fr".into(),
        ..Inputs::default()
    };
    let request = Request::capture(&model, staged, Some(&catalogue)).unwrap();
    assert_eq!(play(&mut model, &request).unwrap(), "Salut.");
    staged = Inputs {
        locale: "fr".into(),
        variant: "formal".into(),
        ..Inputs::default()
    };
    let request = Request::capture(&model, staged, Some(&catalogue)).unwrap();
    assert_eq!(play(&mut model, &request).unwrap(), "Bonjour.");
    assert!(
        model
            .preview_provenance()
            .unwrap()
            .caption
            .contains("formal")
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), po);
}
