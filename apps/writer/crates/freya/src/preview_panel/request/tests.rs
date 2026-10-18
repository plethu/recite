use super::*;

use super::super::tests::capture;

#[test]
fn publication_rejects_departure_reopen_widget_draft_and_catalogue_change()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n> guide@12345678901234567890\n  Hello.\n-> END\n";
    let mut model = Workbench::new(source)?;
    model.select(View::Source)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(
        &path,
        "msgctxt \"12345678901234567890\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n",
    )?;
    let mut catalogue = Catalogue::open(&path)?;
    let requested = capture(&model, Some(&catalogue), true);
    let source_only = capture(&model, Some(&catalogue), false);
    assert!(requested.is_current(&model, Pane::Preview, Some(&catalogue)));
    assert!(!requested.is_current(&model, Pane::Script, Some(&catalogue)));
    let mut reopened = Workbench::new(source)?;
    reopened.select(View::Source)?;
    assert!(!requested.is_current(&reopened, Pane::Preview, Some(&catalogue)));
    // Same widget harvest called by the actual result task, before its predicate.
    model.select(View::Source)?;
    crate::buffers::harvest_draft(
        &mut model,
        &freya::code_editor::Rope::from_str(&format!("{source}# draft\n")),
        "",
    );
    assert!(!requested.is_current(&model, Pane::Preview, Some(&catalogue)));
    model.discard();
    catalogue.reload()?;
    assert!(!requested.is_current(&model, Pane::Preview, Some(&catalogue)));
    assert!(source_only.is_current(&model, Pane::Preview, Some(&catalogue)));
    Ok(())
}
