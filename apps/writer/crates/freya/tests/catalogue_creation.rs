mod support;
use freya::prelude::*;
use freya_testing::prelude::*;

fn has_text(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| Label::try_downcast(e).filter(|l| l.text.contains(text)))
        .is_some()
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
fn catalogue_creation_revalidates_other_scenes_before_writing()
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
    test.sync_and_update();
    assert!(has_text(&test, "Preparing the catalogue"));
    std::fs::write(
        dir.path().join("other.recite"),
        ":: another\n> added@99999999999999999999\n  Added while preparing.\n-> END\n",
    )?;
    for _ in 0..100 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        test.poll_n(std::time::Duration::from_millis(16), 5);
        if !has_text(&test, "Preparing the catalogue") {
            break;
        }
    }
    assert!(!path.exists(), "stale catalogue must not be persisted");
    assert!(has_text(&test, "changed"));
    Ok(())
}
