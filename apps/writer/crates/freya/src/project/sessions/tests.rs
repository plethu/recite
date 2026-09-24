use super::*;

#[test]
fn extraction_uses_retained_edits_and_refreshes_clean_scenes()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let root = dir.path().canonicalize()?;
    std::fs::write(
        root.join("recite.project.toml"),
        "format_version = 1\n[project]\n",
    )?;
    let a = root.join("a.recite");
    let b = root.join("b.recite");
    let source = ":: start default\n> line@11111111111111111111\n  Original.\n-> END\n";
    std::fs::write(&a, source)?;
    std::fs::write(&b, ":: other\n-> END\n")?;
    let mut files = ProjectFiles::open(&root)?;
    let mut current = files.workbench()?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft(source.replace("Original.", "Unsaved."));
    current.apply()?;
    files.switch(&mut current, &b, |_| Ok(()))?;
    let extract =
        |files: &ProjectFiles, current: &Workbench| -> Result<String, Box<dyn std::error::Error>> {
            let discovered = recite_config::discover_project(&root)?;
            let context = files.catalogue_context(crate::project_context::load(&discovered)?)?;
            let document = Document::in_project(
                current.document().key().clone(),
                current.document().source(),
                context,
            )?;
            Ok(document
                .extract_catalogue()
                .catalog
                .ok_or("catalogue")?
                .to_pot_string())
        };
    assert!(extract(&files, &current)?.contains("Unsaved."));
    files.save_retained()?;
    std::fs::write(&a, source.replace("Original.", "External."))?;
    assert!(extract(&files, &current)?.contains("External."));
    files.switch(&mut current, &a, |_| Ok(()))?;
    current.select(recite_writer_model::View::Source)?;
    current.set_draft("Unapplied".into());
    files.switch(&mut current, &b, |_| Ok(()))?;
    assert!(extract(&files, &current).is_err());
    Ok(())
}
