use super::*;

#[test]
fn inconsistent_beat_and_passage_link_does_not_change_selection() -> Result<(), WorkbenchError> {
    let mut model = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let before = model.view().clone();
    let passage = model.document().passage_snapshot()?[0].id.clone();
    let location = Location {
        beat: Some("not_the_passage_beat".into()),
        passage: Some(passage),
        ..Location::default()
    };
    assert!(select(&mut model, &location).is_err());
    assert_eq!(model.view(), &before);
    Ok(())
}
#[test]
fn linked_catalogue_must_stay_inside_its_project() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("locale"))?;
    let inside = root.path().join("locale/fr.po");
    std::fs::write(&inside, "")?;
    std::fs::write(outside.path().join("outside.po"), "")?;
    assert_eq!(catalogue_path(root.path(), "locale/fr.po")?, inside);
    assert!(catalogue_path(root.path(), "../outside.po").is_err());
    assert!(catalogue_path(root.path(), outside.path().to_str().ok_or("path")?).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            outside.path().join("outside.po"),
            root.path().join("locale/link.po"),
        )?;
        assert!(catalogue_path(root.path(), "locale/link.po").is_err());
    }
    Ok(())
}
