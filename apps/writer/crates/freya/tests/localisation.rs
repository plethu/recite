mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn has_text(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.contains(text)))
        .is_some()
}
#[test]
fn localisation_keeps_full_beat_and_source_only_entry_is_optional()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1600., 1100.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Localize")?;
    assert!(has_text(&test, "Localisation is optional"));
    support::click(&mut test, "Open PO catalogue")?;
    assert!(has_text(&test, "PO catalogue path"));
    support::click(&mut test, "Close")?;
    support::click(&mut test, "Write")?;
    assert!(!has_text(&test, "Localisation is optional"));
    assert!(has_text(&test, "Replies"));
    Ok(())
}

#[test]
fn po_open_edit_and_close_protection_are_native_interactions()
-> Result<(), Box<dyn std::error::Error>> {
    let example = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let passages = example.document().passage_snapshot()?;
    let source = passages
        .iter()
        .map(|p| {
            format!(
                "msgctxt {:?}\nmsgid {}\nmsgstr \"\"\n\n",
                p.id,
                serde_json::to_string(&p.text).unwrap_or_default()
            )
        })
        .collect::<String>();
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, source)?;
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1600., 1100.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Localize")?;
    support::click(&mut test, "Open PO catalogue")?;
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.contains("/path/to/fr.po")))
                .map(|_| node.layout().area)
        })
        .ok_or("catalogue input")?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.write_text(path.to_string_lossy());
    support::click(&mut test, "Open")?;
    assert!(has_text(&test, "Translation queue"));
    if let Ok(path) = std::env::var("RECITE_WRITER_SCREENSHOT") {
        test.render_to_file(path);
    }

    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| {
                    p.spans
                        .iter()
                        .any(|s| s.text.contains("Write a translation"))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("translation input")?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.write_text("A translation draft");
    test.sync_and_update();
    assert!(has_text(&test, "Unsaved changes"));
    assert!(matches!(
        recite_writer::request_close(),
        CloseDecision::KeepOpen
    ));
    support::click(&mut test, "fr.po")?;
    support::click(&mut test, "Add language")?;
    support::click(&mut test, "Create catalogue")?;
    assert!(has_text(&test, "Save or discard translation drafts"));
    support::click(&mut test, "Cancel")?;
    assert!(has_text(&test, "Unsaved changes"));
    let external =
        std::fs::read_to_string(&path)?.replace("msgstr \"\"", "msgstr \"External version\"");
    std::fs::write(&path, external)?;
    support::click(&mut test, "fr.po")?;
    support::click(&mut test, "Compare external changes")?;
    support::click(&mut test, "Keep my drafts")?;
    support::click(&mut test, "Close")?;
    support::click(&mut test, "Save")?;
    assert!(std::fs::read_to_string(path)?.contains("A translation draft"));
    support::click(&mut test, "Write")?;
    assert!(has_text(&test, "Replies"));
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
fn start_localisation_creates_a_catalogue_and_returns_to_the_same_beat()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("locale/fr-CA.po");
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    std::fs::write(
        dir.path().join("scene.recite"),
        recite_writer_model::FIXTURE,
    )?;
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(1600., 1100.),
        |_| {},
        1.,
    )
    .0;
    fill_placeholder(&mut test, "Project folder", &dir.path().to_string_lossy())?;
    support::click(&mut test, "Open project")?;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Localize")?;
    support::click(&mut test, "Start localisation")?;
    assert!(has_text(&test, "Includes dialogue across the project"));
    support::click(&mut test, "Choose language")?;
    fill_placeholder(&mut test, "Search languages", "fr-CA")?;
    for (key, code) in [
        (NamedKey::ArrowDown, Code::ArrowDown),
        (NamedKey::Enter, Code::Enter),
    ] {
        for name in [KeyboardEventName::KeyDown, KeyboardEventName::KeyUp] {
            test.send_event(PlatformEvent::Keyboard {
                name,
                key: Key::Named(key),
                code,
                modifiers: Modifiers::empty(),
            });
            test.poll_n(std::time::Duration::from_millis(16), 3);
        }
    }
    assert!(has_text(&test, &path.to_string_lossy()));
    if let Ok(path) = std::env::var("RECITE_WRITER_SETUP_SCREENSHOT") {
        test.render_to_file(path);
    }
    support::click(&mut test, "Create catalogue")?;
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        test.poll_n(std::time::Duration::from_millis(16), 5);
        if has_text(&test, "Translation queue") {
            break;
        }
    }
    assert!(has_text(&test, "Translation queue"));
    assert!(has_text(&test, "Replies"));
    assert!(has_text(&test, "fr-CA"));
    assert!(!has_text(&test, "Catalogue location"));
    let document = recite_core::PoDocument::read(&path)?;
    assert!(document.entries().iter().any(|e| e.context().is_some()));
    assert!(
        document
            .entries()
            .iter()
            .filter(|e| !e.is_header())
            .all(|e| e.translation() == Some(""))
    );
    let baseline = std::fs::read(&path)?;
    // A second language is available from file identity, not permanent manuscript chrome.
    support::click(&mut test, "fr-CA.po")?;
    support::click(&mut test, "Add language")?;
    support::click(&mut test, "Choose language")?;
    support::click(&mut test, "French · fr-CA")?;
    support::click(&mut test, "Create catalogue")?;
    assert!(has_text(&test, "A file already exists"));
    assert_eq!(std::fs::read(&path)?, baseline);
    support::click(&mut test, "Cancel")?;
    assert!(has_text(&test, "Translation queue"));
    Ok(())
}

#[test]
fn setup_can_be_cancelled_and_invalid_language_keeps_the_form()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(900., 650.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Localize")?;
    support::click(&mut test, "Start localisation")?;
    support::click(&mut test, "Create catalogue")?;
    assert!(has_text(&test, "Choose a recognised language"));
    assert!(has_text(&test, "Catalogue location"));
    support::click(&mut test, "Choose language")?;
    fill_placeholder(&mut test, "Search languages", "burger")?;
    assert!(has_text(&test, "No matching"));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Escape),
        code: Code::Escape,
        modifiers: Modifiers::empty(),
    });
    test.sync_and_update();
    assert!(has_text(&test, "Catalogue location"));
    assert!(!has_text(&test, "No matching"));
    if let Ok(path) = std::env::var("RECITE_WRITER_SETUP_SMALL_SCREENSHOT") {
        test.render_to_file(path);
    }
    support::click(&mut test, "Cancel")?;
    assert!(!has_text(&test, "Catalogue location"));
    assert!(has_text(&test, "Start localisation"));
    Ok(())
}
