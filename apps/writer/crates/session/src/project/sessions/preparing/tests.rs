use super::*;
use recite_compiler::authoring::CancellationToken;
use std::fs;

#[test]
fn capture_shares_all_saved_inputs_and_only_captures_open_overlays()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    for index in 0..128 {
        fs::write(
            directory.path().join(format!("scene_{index:03}.recite")),
            format!(
                ":: beat_{index:03}{}\n> line@{index:020}\n  Scene {index}.\n-> END\n",
                if index == 0 { " default" } else { "" }
            ),
        )?;
    }
    let mut files = ProjectFiles::open(directory.path())?;
    let mut model = files.workbench()?;
    let target = directory.path().join("scene_127.recite");
    for _ in 0..64 {
        let captured = files.switch_request(&model, &target, None)?;
        assert!(Arc::ptr_eq(&files.context, &captured.context));
        assert_eq!(captured.overlays.len(), 1);
        assert_eq!(captured.context.documents.len(), 128);
    }
    // The worker must still combine saved inputs with accepted unsaved overlays.
    model.rename_beat("renamed")?;
    let mut prepared = files
        .switch_request(&model, &target, None)?
        .prepare(&CancellationToken::new(), |_| Ok(()))?;
    let checkpoint = files.begin_navigation_checkpoint(&model)?.wait()?;
    files.accept_switch(&mut model, &mut prepared, &checkpoint)?;
    assert_eq!(files.current, target);
    assert!(
        model
            .document()
            .project_sections()
            .iter()
            .any(|(key, beats)| key == "scene_000.recite" && beats == &["renamed"])
    );
    assert!(
        model.document().diagnostics().is_empty(),
        "{:?}",
        model.document().diagnostics()
    );
    assert!(model.document().source().contains("Scene 127."));
    Ok(())
}
