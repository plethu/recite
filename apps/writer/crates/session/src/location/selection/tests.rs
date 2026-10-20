use super::*;

#[test]
fn incomplete_or_ambiguous_beats_cannot_be_selected_through_a_link() -> Result<(), WorkbenchError> {
    for source in [
        ":: start default\n->\n",
        ":: start default\n-> END\n:: start\n-> END\n",
    ] {
        let mut model = Workbench::new(source)?;
        let before = model.view().clone();
        let location = Location {
            beat: Some("start".into()),
            ..Location::default()
        };
        assert!(select(&mut model, &location).is_err());
        assert_eq!(model.view(), &before);
        assert_eq!(model.document().source(), source);
    }
    Ok(())
}

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
#[test]
fn rules_reuse_preserves_validation_but_explicit_clean_source_links_are_freeform() {
    let source = ":: start default\n? reply@22222222222222222222\n  Continue.\n  -> END\n";
    let mut model = recite_writer_model::Workbench::new(source).unwrap();
    let rules = super::Location {
        screen: super::Screen::Rules,
        passage: Some("22222222222222222222".into()),
        ..super::Location::default()
    };
    super::select(&mut model, &rules).unwrap();
    super::select_reusing_rules(&mut model, &rules).unwrap();
    assert_eq!(
        model.source_validation(),
        recite_writer_model::SourceValidation::NoNewErrors
    );
    super::select(
        &mut model,
        &super::Location {
            source: true,
            ..super::Location::default()
        },
    )
    .unwrap();
    assert_eq!(
        model.source_validation(),
        recite_writer_model::SourceValidation::Freeform
    );
    model.set_draft(":: start default\n? invalid".into());
    model.apply().unwrap();
}
