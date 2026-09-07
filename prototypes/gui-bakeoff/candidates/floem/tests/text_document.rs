use floem::{
    reactive::Scope,
    views::editor::{
        core::{editor::EditType, selection::Selection},
        text::Document,
        text_document::TextDocument,
    },
};
use recite_bakeoff_authoring::{FIXTURE, View, Workbench};

#[test]
fn editor_buffer_edits_round_trip_through_recite() -> Result<(), Box<dyn std::error::Error>> {
    let scope = Scope::new();
    let mut model = Workbench::new(FIXTURE)?;
    let doc = TextDocument::new(scope, model.draft());
    doc.edit_single(
        Selection::region(0, doc.text().len()),
        "日本\nCafé 🐈",
        EditType::InsertChars,
    );
    model.set_draft(doc.text().to_string());
    assert!(model.select(View::Source).is_err());
    model.apply()?;
    model.select(View::Source)?;
    assert!(model.draft().contains("7701ceab59d2adfa057a"));
    model.show_script()?;
    assert_eq!(model.draft(), "日本\nCafé 🐈");
    scope.dispose();
    Ok(())
}
