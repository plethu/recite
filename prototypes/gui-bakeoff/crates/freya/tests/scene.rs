use freya::prelude::*;
use freya_testing::prelude::*;

fn click_label(test: &mut TestingRunner, caption: &str) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, element| {
            Label::try_downcast(element)
                .filter(|label| label.text.as_ref() == caption)
                .map(|_| node.layout().area)
        })
        .ok_or("label missing")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.poll_n(std::time::Duration::from_millis(16), 4);
    Ok(())
}

#[test]
fn draft_refusal_apply_and_source_script_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_bakeoff_freya::app,
        Size2D::new(1200., 800.),
        |_| {},
        1.,
    )
    .0;
    let area = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|paragraph| {
                    paragraph
                        .spans
                        .iter()
                        .any(|span| span.text.contains("Would you tell me"))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("prose input missing")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    test.write_text("A different question.");
    click_label(&mut test, "Source")?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.contains("Apply or discard")))
            .is_some()
    );
    click_label(&mut test, "Apply draft")?;
    click_label(&mut test, "Source")?;
    assert!(
        test.find(
            |_, element| Paragraph::try_downcast(element).filter(|paragraph| paragraph
                .spans
                .iter()
                .any(|span| span.text.contains("A different question.")))
        )
        .is_some()
    );
    click_label(&mut test, "Script")?;
    assert!(
        test.find(
            |_, element| Paragraph::try_downcast(element).filter(|paragraph| paragraph
                .spans
                .iter()
                .any(|span| span.text == "A different question."))
        )
        .is_some()
    );
    click_label(&mut test, "Undo")?;
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("Would you tell me"))))
            .is_some()
    );
    click_label(&mut test, "Redo")?;
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text == "A different question.")))
            .is_some()
    );
    Ok(())
}

#[test]
fn whole_scene_source_and_dark_theme_render() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_bakeoff_freya::app,
        Size2D::new(1200., 800.),
        |_| {},
        1.,
    )
    .0;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.contains("That depends a good deal")))
            .is_some()
    );
    capture(&mut test, "script-light.png")?;
    click_label(&mut test, "Dark")?;
    capture(&mut test, "script-dark.png")?;
    click_label(&mut test, "Source")?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.as_ref() == "Light"))
            .is_some()
    );
    test.poll_n(std::time::Duration::from_millis(16), 4);
    capture(&mut test, "source-dark.png")?;
    click_label(&mut test, "Light")?;
    capture(&mut test, "source-light.png")?;
    Ok(())
}

fn capture(test: &mut TestingRunner, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(directory) = std::env::var("RECITE_BAKEOFF_CAPTURE_DIR") {
        std::fs::create_dir_all(&directory)?;
        test.poll_n(std::time::Duration::from_millis(16), 4);
        test.render_to_file(std::path::Path::new(&directory).join(name));
    }
    Ok(())
}

#[test]
fn project_open_save_and_conflict_keep_the_authored_text() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    std::fs::write(
        dir.path().join("second.recite"),
        ":: other\n> other@33333333333333333333\n  Other scene.\n-> END\n",
    )?;
    let source_path = dir.path().join("scene.recite");
    std::fs::write(&source_path, recite_bakeoff_authoring::FIXTURE)?;
    let mut test = TestingRunner::new(
        recite_bakeoff_freya::editor_app,
        Size2D::new(1200., 800.),
        |_| {},
        1.,
    )
    .0;
    let path_field = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("Project folder")))
                .map(|_| node.layout().area)
        })
        .ok_or("project path field")?;
    test.click_cursor((
        f64::from(path_field.min_x() + 2.),
        f64::from(path_field.min_y() + 2.),
    ));
    test.write_text(dir.path().to_string_lossy());
    click_label(&mut test, "Open project")?;
    let area = test
        .find(|node, element| {
            Paragraph::try_downcast(element)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("Would you tell me")))
                .map(|_| node.layout().area)
        })
        .ok_or("prose field")?;
    test.click_cursor((f64::from(area.min_x() + 2.), f64::from(area.min_y() + 2.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    test.write_text("A saved question.");
    click_label(&mut test, "second.recite")?;
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|l| l.text.contains("before changing files")))
            .is_some()
    );
    assert_eq!(
        std::fs::read_to_string(&source_path)?,
        recite_bakeoff_authoring::FIXTURE
    );
    click_label(&mut test, "Save")?;
    assert!(std::fs::read_to_string(&source_path)?.contains("A saved question."));
    assert!(std::fs::read_to_string(&source_path)?.contains("7701ceab59d2adfa057a"));
    std::fs::write(&source_path, "external edit")?;
    click_label(&mut test, "Save")?;
    assert_eq!(std::fs::read_to_string(&source_path)?, "external edit");
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|l| l.text.contains("changed on disk")))
            .is_some()
    );
    assert!(
        test.find(|_, element| Paragraph::try_downcast(element)
            .filter(|p| p.spans.iter().any(|s| s.text == "A saved question.")))
            .is_some()
    );
    Ok(())
}
