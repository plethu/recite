use super::*;

#[test]
fn inspected_baseline_survives_sibling_navigation_with_local_history() {
    for decision in [
        ExternalDecision::KeepWriter,
        ExternalDecision::UseInspectedDisk,
    ] {
        let (directory, files, _) = fixture();
        let original = files.current.clone();
        let sibling = directory.path().join("other.recite");
        fs::write(&sibling, ":: sibling\n-> END\n").unwrap();
        drop(files);
        let mut files = ProjectFiles::open(directory.path()).unwrap();
        let mut model = files.workbench().unwrap();
        files
            .switch(
                &mut model,
                &original,
                Some(View::Passage(LINE.into())),
                |_| Ok(()),
            )
            .unwrap();
        model.set_draft("Accepted local prose.".into());
        model.apply().unwrap();
        let local = model.document().source().to_owned();
        model.set_draft("Pending local prose.".into());
        let before = model.recovery();
        let disk = SOURCE.replace("Original.", "Inspected external.");
        fs::write(&original, &disk).unwrap();
        files.compare_external(&model).unwrap();
        let resolution = files.resolve_external(&mut model, decision).unwrap();
        assert!(resolution.recovery_error.is_none());
        if decision == ExternalDecision::KeepWriter {
            assert_eq!(model.recovery(), before);
            model.discard();
        } else {
            model.undo().unwrap();
        }
        assert_eq!(model.document().source(), local);
        files
            .switch(&mut model, &sibling, None, |_| Ok(()))
            .unwrap();
        files
            .switch(&mut model, &original, None, |_| Ok(()))
            .unwrap();
        assert_eq!(model.document().source(), local);
        assert_eq!(files.saved.as_ref(), disk);
        assert!(files.dirty(model.document().source()));
        assert!(!files.external_check(&original).unwrap().observe().changed());
        model.undo().unwrap();
        assert_eq!(model.document().source(), SOURCE);
        model.redo().unwrap();
        assert_eq!(model.document().source(), local);
        files.save_current(&mut model).unwrap();
        assert_eq!(fs::read_to_string(original).unwrap(), local);
    }
}
