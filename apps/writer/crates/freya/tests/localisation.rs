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
    let width: f32 = std::env::var("RECITE_GUI_TEST_WIDTH")
        .ok()
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(1600.);
    let height: f32 = std::env::var("RECITE_GUI_TEST_HEIGHT")
        .ok()
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(1100.);
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(width, height), |_| {}, 1.).0;
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
    submit_dialog(&mut test);
    assert!(has_text(&test, "Translation queue"));
    assert!(has_text(&test, "Source · read only"));
    assert!(!has_text(
        &test,
        "Missing Courier → Let me ask about something else."
    ));
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .any(|s| s.text.contains("If you're here about the transmitter"))))
            .is_none()
    );
    support::click(&mut test, "▸ Station History")?;
    let preview = test
        .find(|node, e| {
            Rect::try_downcast(e)
                .filter(|r| {
                    r.accessibility.builder.label() == Some("Destination preview · read only")
                })
                .map(|_| node.layout().area)
        })
        .ok_or("source preview")?;
    let translation = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| {
                    p.spans
                        .iter()
                        .any(|s| s.text.contains("Write a translation"))
                })
                .map(|_| node.layout().area)
        })
        .ok_or("translation column")?;
    assert!(preview.max_x() < translation.min_x());

    if let Ok(path) = std::env::var("RECITE_WRITER_SCREENSHOT") {
        test.render_to_file(path);
    }

    support::click(&mut test, "▾ Station History")?;
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
    let label_area = |caption: &str| {
        test.find(|node, e| {
            Label::try_downcast(e)
                .filter(|l| l.text.as_ref() == caption)
                .map(|_| node.layout().area)
        })
        .ok_or_else(|| format!("Missing {caption}"))
    };
    let save_area = label_area("Save")?;
    let discard_area = label_area("Discard draft")?;
    let review_area = label_area("Reviewed")?;
    assert!(
        save_area.width() > 20. && save_area.height() < 32.,
        "Save must not wrap vertically"
    );
    assert!(review_area.max_x() < save_area.min_x());
    assert!(save_area.max_x() < discard_area.min_x());
    assert!(discard_area.max_x() < width);
    assert!((save_area.center().y - review_area.center().y).abs() < 4.);
    if let Ok(path) = std::env::var("RECITE_WRITER_SCREENSHOT") {
        test.render_to_file(format!("{path}.active.png"));
    }
    support::click(&mut test, "Translation queue")?;
    assert!(has_text(&test, "Needs review · Unsaved changes"));
    assert!(has_text(&test, "Relay Desk"));
    assert!(!has_text(&test, "relay_desk ·"));
    if let Ok(path) = std::env::var("RECITE_WRITER_SCREENSHOT") {
        test.render_to_file(format!("{path}.queue.png"));
    }
    support::click(&mut test, "Read passage")?;

    assert!(matches!(
        recite_writer::request_close(),
        CloseDecision::KeepOpen
    ));
    test.sync_and_update();
    test.poll_n(std::time::Duration::from_millis(16), 15);
    assert!(has_text(&test, "Unsaved translations"));
    assert!(has_text(&test, "A translation draft"));
    assert!(has_text(&test, "Save all and quit"));
    if let Ok(path) = std::env::var("RECITE_WRITER_SCREENSHOT") {
        test.render_to_file(format!("{path}.close.png"));
    }
    support::click(&mut test, "Keep editing")?;
    assert!(has_text(&test, "Edit translation"));
    support::click(&mut test, "Read passage")?;
    support::click(&mut test, "fr.po")?;
    support::click(&mut test, "Add language")?;
    support::click(&mut test, "Choose language")?;
    fill_placeholder(&mut test, "Search languages", "fr-CA")?;
    support::click(&mut test, "French (Canada) · fr-CA")?;
    support::click(&mut test, "Create catalogue")?;
    assert!(has_text(&test, "Save or discard translation drafts"));
    support::click(&mut test, "Cancel")?;
    assert!(has_text(&test, "Unsaved changes"));
    let external =
        std::fs::read_to_string(&path)?.replace("msgstr \"\"", "msgstr \"External version\"");
    std::fs::write(&path, external)?;
    assert!(matches!(
        recite_writer::request_close(),
        CloseDecision::KeepOpen
    ));
    test.poll_n(std::time::Duration::from_millis(16), 15);
    support::click(&mut test, "Save all and quit")?;
    assert!(has_text(&test, "Could not save"));
    assert!(has_text(&test, "Your drafts are still available"));
    assert!(has_text(&test, "Unsaved translations"));
    support::click(&mut test, "Open Relay Desk")?;
    assert!(has_text(&test, "Edit translation"));
    support::click(&mut test, "Read passage")?;

    support::click(&mut test, "fr.po")?;
    support::click(&mut test, "Compare external changes")?;
    support::click(&mut test, "Keep my drafts")?;
    support::click(&mut test, "Use as editable draft")?;
    support::click(&mut test, "Save")?;
    assert!(std::fs::read_to_string(path)?.contains("A translation draft"));
    support::click(&mut test, "Edit source")?;
    assert!(has_text(&test, "Replies"));
    Ok(())
}

fn submit_dialog(test: &mut TestingRunner) {
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Named(NamedKey::Enter),
        code: Code::Enter,
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    });
    test.poll_n(std::time::Duration::from_millis(16), 4);
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
    assert!(has_text(&test, "Translate project dialogue"));
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
    assert!(has_text(&test, "locale/fr-CA.po"));
    if let Ok(path) = std::env::var("RECITE_WRITER_SETUP_SCREENSHOT") {
        test.render_to_file(path);
    }
    submit_dialog(&mut test);
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
    assert!(!has_text(&test, "catalogue location"));
    let document = recite_core::po::PoDocument::read(&path)?;
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
    fill_placeholder(&mut test, "Search languages", "fr-CA")?;
    support::click(&mut test, "French (Canada) · fr-CA")?;
    support::click(&mut test, "Create catalogue")?;
    for _ in 0..100 {
        if has_text(&test, "A file already exists") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
        test.poll_n(std::time::Duration::from_millis(16), 5);
    }
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
    assert!(!has_text(&test, "Choose a recognised language"));
    assert!(has_text(&test, "catalogue location"));
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
    assert!(has_text(&test, "catalogue location"));
    assert!(!has_text(&test, "No matching"));
    if let Ok(path) = std::env::var("RECITE_WRITER_SETUP_SMALL_SCREENSHOT") {
        test.render_to_file(path);
    }
    support::click(&mut test, "Cancel")?;
    assert!(!has_text(&test, "catalogue location"));
    assert!(has_text(&test, "Start localisation"));
    Ok(())
}

#[test]
fn source_refresh_previews_cancels_and_saves_without_losing_translation()
-> Result<(), Box<dyn std::error::Error>> {
    let example = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let passages = example.document().passage_snapshot()?;
    let first = passages.first().ok_or("example passage")?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    let source = format!(
        "# Translator note\nmsgctxt {:?}\nmsgid \"Earlier source text\"\nmsgstr \"Existing translation\"\n",
        first.id
    );
    std::fs::write(&path, &source)?;
    let mut test = TestingRunner::new(
        recite_writer::app,
        if std::env::var_os("RECITE_REFRESH_SMALL").is_some() {
            Size2D::new(900., 650.)
        } else {
            Size2D::new(1600., 1100.)
        },
        |_| {},
        1.,
    )
    .0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Localize")?;
    support::click(&mut test, "Open PO catalogue")?;
    fill_placeholder(&mut test, "/path/to/fr.po", &path.to_string_lossy())?;
    support::click(&mut test, "Open")?;
    support::click(&mut test, "fr.po")?;
    support::click(&mut test, "Refresh from source")?;
    assert!(has_text(&test, "Changed source entries: 1"));
    assert_eq!(std::fs::read_to_string(&path)?, source);
    support::click(&mut test, "Anything I can do to help?")?;
    support::click(&mut test, "Back")?;
    assert!(!has_text(&test, "Update catalogue"));
    assert_eq!(std::fs::read_to_string(&path)?, source);
    support::click(&mut test, "Forward")?;
    assert!(has_text(&test, "Not present in this version"));
    support::click(&mut test, &first.text.chars().take(100).collect::<String>())?;
    assert!(has_text(&test, "Previous source"));
    if let Ok(path) = std::env::var("RECITE_REFRESH_SCREENSHOT") {
        test.render_to_file(path);
    }
    support::click(&mut test, "Update catalogue")?;
    assert!(has_text(&test, "Catalogue refreshed."));
    let saved = recite_core::po::PoDocument::read(&path)?;
    let translated = saved
        .entries()
        .iter()
        .find(|e| e.context() == Some(first.id.as_str()))
        .ok_or("refreshed entry")?;
    assert_eq!(translated.source_text(), first.text);
    assert_eq!(translated.translation(), Some("Existing translation"));
    assert!(translated.flags().iter().any(|f| f == "fuzzy"));
    assert!(saved.source().contains("# Translator note"));
    support::click(&mut test, "Read passage")?;
    assert!(has_text(&test, "Translation queue"));
    Ok(())
}

#[test]
fn refresh_refuses_a_project_changed_after_preview() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"refresh\"\nversion = \"1\"\n",
    )?;
    std::fs::write(
        dir.path().join("scene.recite"),
        recite_writer_model::FIXTURE,
    )?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, "")?;
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
    support::click(&mut test, "Open PO catalogue")?;
    fill_placeholder(&mut test, "/path/to/fr.po", &path.to_string_lossy())?;
    support::click(&mut test, "Open")?;
    support::click(&mut test, "fr.po")?;
    support::click(&mut test, "Refresh from source")?;
    std::fs::write(
        dir.path().join("other.recite"),
        ":: extra\n> new@99999999999999999999\n  New source.\n-> END\n",
    )?;
    support::click(&mut test, "Update catalogue")?;
    assert!(has_text(&test, "changed while preparing"));
    assert_eq!(std::fs::read_to_string(&path)?, "");
    Ok(())
}

#[test]
fn localisation_sidebar_opens_the_beat_it_highlights() -> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(recite_writer::app, Size2D::new(1400., 1000.), |_| {}, 1.).0;
    support::open_beat(&mut test)?;
    support::click(&mut test, "Courier Route")?;
    support::click(&mut test, "Localize")?;
    assert!(has_text(&test, "The old flood road."));
    support::click(&mut test, "Missing Courier")?;
    assert!(has_text(&test, "Our courier is two days late."));
    assert!(!has_text(&test, "The old flood road."));
    support::click(&mut test, "Edit source")?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e).filter(|p| p
            .spans
            .iter()
            .any(|s| s.text.contains("Our courier is two days late."))))
            .is_some()
    );
    Ok(())
}
