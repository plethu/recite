use recite_compiler::authoring::CancellationToken;
use recite_core::po::PoEntryId;
use recite_writer_model::View;
use recite_writer_session::{
    ProjectSession,
    localisation::{
        catalogue::{Catalogue, Draft, RefreshPreview},
        extraction,
    },
};
use std::fs;

const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n";
const PO: &str = "# Translator's note\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n";

#[test]
fn refreshed_sources_preserve_translations_and_require_exact_reviewed_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("fr.po");
    fs::write(&path, PO)?;
    let mut catalogue = Catalogue::open(&path)?;
    let template = PO.replace("Hello.", "Welcome.").replace("Bonjour.", "");
    let preview = RefreshPreview::prepare(&catalogue, template.clone())?;
    assert_eq!(preview.changes.len(), 1);
    assert_eq!(preview.changes[0].old.as_deref(), Some("Hello."));
    assert_eq!(preview.changes[0].new.as_deref(), Some("Welcome."));
    assert_eq!(preview.changes[0].translation, "Bonjour.");
    assert_eq!(fs::read_to_string(&path)?, PO);
    assert!(preview.apply(&mut catalogue, PO).is_err());
    catalogue.update(
        PoEntryId::new(0),
        Draft {
            forms: vec!["Salut.".into()],
            reviewed: false,
        },
    );
    catalogue.discard(PoEntryId::new(0));
    assert!(
        !preview.is_current(&catalogue),
        "editing then discarding invalidates review"
    );
    assert!(preview.apply(&mut catalogue, &template).is_err());
    let preview = RefreshPreview::prepare(&catalogue, template.clone())?;
    preview.apply(&mut catalogue, &template)?;
    let saved = fs::read_to_string(&path)?;
    assert!(
        saved.contains("Welcome.")
            && saved.contains("Bonjour.")
            && saved.contains("# Translator's note")
    );
    assert!(
        saved.contains("fuzzy"),
        "changed source requires translation review"
    );
    assert!(!preview.is_current(&catalogue));
    Ok(())
}

#[test]
fn source_refresh_rejects_foreign_owners_and_external_po_writes()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("fr.po");
    fs::write(&path, PO)?;
    let mut catalogue = Catalogue::open(&path)?;
    let mut foreign = Catalogue::open(&path)?;
    let preview = RefreshPreview::prepare(&catalogue, PO.to_owned())?;
    assert!(!preview.is_current(&foreign));
    assert!(preview.apply(&mut foreign, PO).is_err());
    let external = PO.replace("Bonjour.", "Bonsoir.");
    fs::write(&path, &external)?;
    assert!(preview.apply(&mut catalogue, PO).is_err());
    assert_eq!(fs::read_to_string(&path)?, external);
    assert_eq!(catalogue.document.source(), PO);
    Ok(())
}

#[test]
fn shared_extraction_uses_retained_accepted_edits_and_rejects_unresolved_fields()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().canonicalize()?;
    fs::write(
        root.join("recite.project.toml"),
        "format_version = 1\n[project]\n",
    )?;
    let a = root.join("a.recite");
    let b = root.join("b.recite");
    fs::write(&a, SOURCE)?;
    fs::write(&b, ":: other\n-> END\n")?;
    let mut project = ProjectSession::open(&root, Some(&a))?;
    project.current_mut().edit(|model| {
        model.select(View::Passage("11111111111111111111".into()))?;
        model.set_draft("Accepted unsaved.".into());
        model.apply()
    })?;
    project.switch(&b)?;
    let extract = |project: &ProjectSession| {
        extraction::template(
            project.current().model(),
            Some(project.files()),
            &CancellationToken::new(),
        )
    };
    assert!(extract(&project)?.contains("Accepted unsaved."));
    project.save_all()?;
    fs::write(&a, SOURCE.replace("Hello.", "Changed on disk."))?;
    assert!(extract(&project)?.contains("Changed on disk."));
    project.switch(&a)?;
    project.current_mut().edit(|model| {
        model.select(View::Passage("11111111111111111111".into()))?;
        model.set_draft("Unapplied field.".into());
        Ok(())
    })?;
    assert!(extract(&project).is_err());
    project.switch(&b)?;
    assert!(
        extract(&project).is_err(),
        "retained fields also require a decision"
    );
    project.switch(&a)?;
    project.current_mut().edit(|model| {
        model.discard();
        model.set_draft("Accepted retained edit.".into());
        model.apply()
    })?;
    project.switch(&b)?;
    fs::write(
        root.join("recite.project.toml"),
        "format_version=1\n[project]\n[discovery]\nexcludes=['a.recite']\n",
    )?;
    let error = extract(&project)
        .err()
        .ok_or("excluded retained source must not disappear silently")?;
    assert!(error.contains("a.recite"));
    Ok(())
}
