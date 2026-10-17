//! First-run and empty states render their frames, titles and instructions.
mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn capture(test: &mut TestingRunner, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(directory) = std::env::var("RECITE_WRITER_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.render_to_file(std::path::Path::new(&directory).join(name));
    }
    Ok(())
}

fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.as_ref() == text))
        .is_some()
}

#[test]
fn a_window_without_a_project_welcomes_with_the_open_action()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(1200., 900.),
        |_| {},
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    assert!(has(&test, "Open a project"));
    capture(&mut test, "welcome.png")?;
    Ok(())
}

fn fill_placeholder(
    test: &mut TestingRunner,
    placeholder: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.contains(placeholder)))
                .map(|_| node.layout().area)
        })
        .ok_or_else(|| format!("Missing input: {placeholder}"))?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.write_text(text);
    test.sync_and_update();
    Ok(())
}

#[test]
fn new_project_writes_a_starter_project_and_opens_it() -> Result<(), Box<dyn std::error::Error>> {
    let root = match std::env::var("RECITE_WRITER_NEW_PROJECT_DIR") {
        Ok(dir) => std::path::PathBuf::from(dir),
        Err(_) => tempfile::tempdir()?.keep(),
    };
    let folder = root.join("night-bus");
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(1200., 900.),
        |_| {},
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    support::click(&mut test, "New project…")?;
    fill_placeholder(&mut test, "Night Bus", "Night Bus")?;
    fill_placeholder(
        &mut test,
        "Folder for the new project",
        &folder.to_string_lossy(),
    )?;
    test.poll_n(std::time::Duration::from_millis(16), 6);
    capture(&mut test, "new-project-dialog.png")?;
    support::click(&mut test, "Create")?;
    for _ in 0..200 {
        test.poll_n(std::time::Duration::from_millis(16), 2);
        if has(&test, "Night Bus") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    capture(&mut test, "new-project-created.png")?;
    assert!(folder.join("recite.project.toml").is_file());
    assert!(folder.join("night_bus.recite").is_file());
    assert!(
        has(&test, "Night Bus"),
        "the new project opens on its scene"
    );
    capture(&mut test, "new-project-created.png")?;

    // A second attempt into the same folder is refused with a message, and writes nothing.
    let before = std::fs::read_to_string(folder.join("night_bus.recite"))?;
    let again = recite_config::create_starter_project(&folder, "Night Bus");
    assert!(again.is_err());
    assert_eq!(
        std::fs::read_to_string(folder.join("night_bus.recite"))?,
        before
    );
    Ok(())
}

#[test]
fn open_examples_switches_the_window_to_the_bundled_examples()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(1200., 900.),
        |_| {},
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    support::click(&mut test, "Open examples")?;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    assert!(!has(&test, "Open a project"));
    assert!(has(&test, "Relay Hub"));
    capture(&mut test, "examples-from-welcome.png")?;
    Ok(())
}

#[test]
fn the_examples_reach_an_empty_beat_and_a_scene_with_no_beats()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1200., 900.), |_| {}, 1.).0;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    support::click(&mut test, "Empty Beat")?;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    assert!(has(&test, "This beat is empty"));
    capture(&mut test, "empty-beat-script.png")?;
    support::click(&mut test, "Blank Scene")?;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    assert!(has(&test, "This scene is empty"));
    assert!(!has(
        &test,
        "The selected line no longer exists or has no unique frozen ID."
    ));
    capture(&mut test, "blank-scene-script.png")?;
    support::click(&mut test, "Map")?;
    capture(&mut test, "blank-scene-map.png")?;
    assert!(has(&test, "No beats yet"));
    support::click(&mut test, "Script")?;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    assert!(!has(&test, "Add line"));
    support::click(&mut test, "Add beat")?;
    test.poll_n(std::time::Duration::from_millis(16), 12);
    assert!(!has(&test, "This scene is empty"));
    assert!(has(&test, "Add line"));
    assert!(!has(
        &test,
        "project must declare exactly one default block"
    ));
    capture(&mut test, "blank-scene-added-beat.png")?;
    Ok(())
}
