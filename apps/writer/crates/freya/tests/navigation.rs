mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn has_text(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.contains(text)))
        .is_some()
}
fn has_input(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| {
        Paragraph::try_downcast(e).filter(|p| p.spans.iter().any(|s| s.text.contains(text)))
    })
    .is_some()
}

#[test]
fn initial_links_and_browser_history_cross_scenes_and_modes()
-> Result<(), Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1400., 1000.),
        |runner| {
            runner.provide_root_context(|| {
                recite_writer::InitialRoute(
                    "recite://writer/write?scene=last_tram_linear.recite&beat=last_tram".into(),
                )
            });
        },
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    assert!(has_input(&test, "Is this the last tram?"));
    support::click(&mut test, "Localize")?;
    assert!(has_text(&test, "Localisation is optional"));
    support::click(&mut test, "Back")?;
    assert!(!has_text(&test, "Localisation is optional"));
    support::click(&mut test, "Forward")?;
    assert!(has_text(&test, "Localisation is optional"));
    support::click(&mut test, "Write")?;
    support::click(&mut test, "Relay Hub")?;
    support::click(&mut test, "Back")?;
    assert!(has_input(&test, "Is this the last tram?"));
    Ok(())
}

#[test]
fn queue_is_a_screen_and_back_restores_its_search() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    let example = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let passages = example.document().passage_snapshot()?;
    let first = &passages[0];
    let query = first.text.chars().take(16).collect::<String>();
    let mut source = (0..40)
        .map(|i| {
            format!(
                "msgctxt \"{i:020}\"\nmsgid {}\nmsgstr \"\"\n\n",
                serde_json::to_string(&format!("{query} synthetic {i}")).unwrap_or_default()
            )
        })
        .collect::<String>();
    source.push_str(&format!(
        "msgctxt {:?}\nmsgid {}\nmsgstr \"\"\n\n",
        first.id,
        serde_json::to_string(&first.text)?
    ));
    std::fs::write(&path, source)?;
    let params = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("scene", "relay_hub.recite")
        .append_pair("catalogue", &path.to_string_lossy())
        .append_pair("q", &query)
        .append_pair("page", "1")
        .finish();
    let link = format!("/translations?{params}");
    let mut test = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1400., 1000.),
        |runner| {
            runner.provide_root_context(move || recite_writer::InitialRoute(link));
        },
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    assert!(has_text(&test, "Matching entries: 41"));
    assert!(has_input(&test, &query));
    assert!(!has_text(&test, &format!("{query} synthetic 0")));
    assert!(
        test.find(|_, e| Rect::try_downcast(e)
            .filter(|r| r.accessibility.builder.role() == AccessibilityRole::Dialog))
            .is_none()
    );
    let count = test
        .find(|node, e| {
            Label::try_downcast(e)
                .filter(|label| label.text == "Matching entries: 41")
                .map(|_| node.layout().area)
        })
        .ok_or("queue count")?;
    assert!(count.height() < 40., "count must fit beside the filter");
    support::click(&mut test, &first.text.chars().take(180).collect::<String>())?;
    assert!(!has_text(&test, "Matching entries"));
    assert!(has_text(&test, "Edit translation"));
    support::click(&mut test, "Back")?;
    assert!(has_input(&test, &query));
    assert!(!has_text(&test, &format!("{query} synthetic 0")));
    assert!(has_text(&test, "Matching entries: 41"));
    support::click(&mut test, "Forward")?;
    assert!(!has_text(&test, "Matching entries"));
    if let Ok(path) = std::env::var("RECITE_WRITER_QUEUE_SCREENSHOT") {
        support::click(&mut test, "Back")?;
        test.render_to_file(path);
    }
    Ok(())
}

#[test]
fn unavailable_link_keeps_the_current_document_and_reports_the_problem() {
    let mut test = TestingRunner::new(
        recite_writer::app,
        Size2D::new(1200., 800.),
        |runner| {
            runner.provide_root_context(|| {
                recite_writer::InitialRoute("/write?scene=missing.recite&beat=gone".into())
            });
        },
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(16), 15);
    assert!(has_text(&test, "The linked scene is not open"));
    assert!(has_text(&test, "Relay Hub"));
}

fn fill(
    test: &mut TestingRunner,
    contains: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.contains(contains)))
                .map(|_| node.layout().area)
        })
        .ok_or("input")?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: Modifiers::CONTROL,
    });
    test.sync_and_update();
    test.write_text(text);
    test.sync_and_update();
    Ok(())
}

#[test]
fn back_and_forward_preserve_edits_without_requiring_a_save()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    std::fs::write(dir.path().join("a.recite"), recite_writer_model::FIXTURE)?;
    let second = dir.path().join("b.recite");
    std::fs::write(
        &second,
        ":: other default\n> other@33333333333333333333\n  Other scene.\n-> END\n",
    )?;
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(1400., 1000.),
        |_| {},
        1.,
    )
    .0;
    fill(&mut test, "Project folder", &dir.path().to_string_lossy())?;
    support::click(&mut test, "Open project")?;
    support::open_beat(&mut test)?;
    support::click(&mut test, "B")?;
    fill(&mut test, "Other scene.", "Keep my changed scene.")?;
    support::click(&mut test, "Back")?;
    assert!(!has_input(&test, "Keep my changed scene."));
    assert!(std::fs::read_to_string(&second)?.contains("Other scene."));
    support::click(&mut test, "Forward")?;
    assert!(has_input(&test, "Keep my changed scene."));
    support::click(&mut test, "Save")?;
    assert!(std::fs::read_to_string(&second)?.contains("Keep my changed scene."));
    Ok(())
}
