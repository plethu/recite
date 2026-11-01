use super::*;

#[test]
fn placement_survives_reopening_and_separates_scene_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let file = UserStateFile::new(directory.path().join("layouts.json"));
    let positions = Positions::from([("passage:one".into(), (120., 240.))]);
    save(&file, "file:/project-a/scene.recite", &positions)?;
    save(
        &file,
        "file:/project-b/scene.recite",
        &Positions::from([("passage:one".into(), (480., 120.))]),
    )?;
    assert_eq!(load(&file, "file:/project-a/scene.recite")?, positions);
    assert_ne!(load(&file, "file:/project-b/scene.recite")?, positions);
    save(&file, "file:/project-a/scene.recite", &Positions::new())?;
    assert!(load(&file, "file:/project-a/scene.recite")?.is_empty());
    assert!(!load(&file, "file:/project-b/scene.recite")?.is_empty());
    Ok(())
}
#[test]
fn invalid_and_future_layouts_are_not_overwritten() -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("layouts.json");
    let file = UserStateFile::new(path.clone());
    for original in [
        "broken",
        r#"{"version":2,"scenes":{}}"#,
        r#"{"version":1,"scenes":{"x":{"y":[-5,0]}}}"#,
    ] {
        std::fs::write(&path, original)?;
        assert!(save(&file, "scene", &Positions::new()).is_err());
        assert_eq!(std::fs::read_to_string(&path)?, original);
    }
    Ok(())
}
