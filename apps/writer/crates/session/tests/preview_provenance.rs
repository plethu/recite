use recite_compiler::authoring::CancellationToken;
use recite_writer_session::{
    ProjectSession,
    localisation::catalogue::{Catalogue, Draft},
    preview::{Inputs, Request, catalogue_stale},
};
use std::fs;

const FIRST: &str = ":: first default\n> @ffee6p8\n  First.\n-> END\n";
const SECOND: &str = ":: second\n> @rthh7ab\n  Second.\n-> END\n";
const PO: &str = "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\nmsgctxt \"rthh7ab\"\nmsgid \"Second.\"\nmsgstr \"Deuxième.\"\n";

fn start(
    project: &mut ProjectSession,
    inputs: Inputs,
    catalogue: Option<&Catalogue>,
) -> Result<String, Box<dyn std::error::Error>> {
    let request = Request::capture(project.current().model(), inputs, catalogue)?;
    let prepared = request.prepare(&CancellationToken::new())?;
    assert!(project.current_mut().accept_preview(prepared).is_ok());
    Ok(project
        .current()
        .model()
        .preview_provenance()
        .ok_or("Prepared trial has no provenance")?
        .caption
        .clone())
}

#[test]
fn retained_files_own_their_trial_captions_and_catalogue_freshness_through_restarts() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.recite");
    let second = dir.path().join("second.recite");
    let po = dir.path().join("fr.po");
    fs::write(dir.path().join("recite.project.toml"), "format_version=1\n").unwrap();
    fs::write(&first, FIRST).unwrap();
    fs::write(&second, SECOND).unwrap();
    fs::write(&po, PO).unwrap();
    let mut catalogue = Catalogue::open(&po).unwrap();
    let mut project =
        ProjectSession::open(dir.path(), Some(std::path::Path::new("first.recite"))).unwrap();
    let first_caption = start(&mut project, Inputs::default(), Some(&catalogue)).unwrap();
    project.switch(&second).unwrap();
    let second_caption = start(
        &mut project,
        Inputs {
            locale: "fr".into(),
            ..Inputs::default()
        },
        Some(&catalogue),
    )
    .unwrap();
    assert_ne!(first_caption, second_caption);
    assert_eq!(
        project.current().model().preview_page().unwrap().text,
        "Deuxième."
    );
    let entry = catalogue
        .document
        .entries()
        .iter()
        .find(|entry| entry.context() == Some("rthh7ab"))
        .unwrap()
        .id();
    catalogue.update(
        entry,
        Draft {
            forms: vec!["Nouveau.".into()],
            reviewed: false,
        },
    );
    assert!(catalogue_stale(
        project.current().model().preview_provenance().unwrap(),
        Some(&catalogue)
    ));
    project.switch(&first).unwrap();
    let model = project.current().model();
    assert_eq!(model.preview_page().unwrap().text, "First.");
    assert_eq!(model.preview_provenance().unwrap().caption, first_caption);
    assert!(!catalogue_stale(
        model.preview_provenance().unwrap(),
        Some(&catalogue)
    ));
    let restarted = start(
        &mut project,
        Inputs {
            variant: "formal".into(),
            ..Inputs::default()
        },
        Some(&catalogue),
    )
    .unwrap();
    assert_ne!(first_caption, restarted);
    let failed = Request::capture(
        project.current().model(),
        Inputs {
            variant: "invalid&variant".into(),
            ..Inputs::default()
        },
        Some(&catalogue),
    )
    .unwrap();
    assert!(failed.prepare(&CancellationToken::new()).is_err());
    assert_eq!(
        project
            .current()
            .model()
            .preview_provenance()
            .unwrap()
            .caption,
        restarted
    );
    assert_eq!(
        project.current().model().preview_page().unwrap().text,
        "First."
    );
    project.switch(&second).unwrap();
    assert_eq!(
        project
            .current()
            .model()
            .preview_provenance()
            .unwrap()
            .caption,
        second_caption
    );
    assert!(catalogue_stale(
        project.current().model().preview_provenance().unwrap(),
        Some(&catalogue)
    ));
    assert_eq!(
        project.current().model().preview_page().unwrap().text,
        "Deuxième."
    );
    assert_eq!(fs::read_to_string(&po).unwrap(), PO);
}
