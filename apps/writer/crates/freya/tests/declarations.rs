mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn project() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\nschema = 'schema.json'\n",
    )?;
    let source =
        "schema_version = 1\n[producer]\nid = 'dialogue'\n[speakers.mara]\ndisplay_name = 'Mara'\n";
    std::fs::write(dir.path().join("schema.toml"), source)?;
    let schema = recite_core::schema::SchemaSource::load_str("schema.toml", source)
        .source
        .ok_or("schema")?;
    std::fs::write(dir.path().join("schema.json"), schema.export_json())?;
    std::fs::write(
        dir.path().join("scene.recite"),
        ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n",
    )?;
    Ok(dir)
}

fn open(root: &std::path::Path) -> Result<TestingRunner, Box<dyn std::error::Error>> {
    let root = root.to_owned();
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(1200., 1100.),
        |runner| {
            runner.provide_root_context(move || recite_writer::InitialProject(Some(root)));
        },
        1.,
    )
    .0;
    test.poll_n(std::time::Duration::from_millis(25), 30);
    support::click(&mut test, "Project")?;
    support::click(&mut test, "Declarations")?;
    Ok(test)
}

fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| {
        Label::try_downcast(e)
            .filter(|l| l.text.contains(text))
            .map(|_| ())
            .or_else(|| {
                Paragraph::try_downcast(e)
                    .filter(|p| p.spans.iter().any(|s| s.text.contains(text)))
                    .map(|_| ())
            })
    })
    .is_some()
}

fn replace(test: &mut TestingRunner, text: &str, value: &str) -> TestResult {
    let area = test
        .find(|node, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.contains(text)))
                .map(|_| node.layout().area)
        })
        .ok_or_else(|| format!("Missing field {text}"))?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.send_event(PlatformEvent::Keyboard {
        name: KeyboardEventName::KeyDown,
        key: Key::Character("a".into()),
        code: Code::KeyA,
        modifiers: if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CONTROL
        },
    });
    test.write_text(value);
    test.sync_and_update();
    Ok(())
}

#[test]
fn declaration_source_failure_discard_and_reload_preserve_authored_and_generated_bytes()
-> TestResult {
    let dir = project()?;
    let path = dir.path().join("schema.toml");
    let original = std::fs::read_to_string(&path)?;
    let generated = std::fs::read_to_string(dir.path().join("schema.json"))?;
    let mut test = open(dir.path())?;
    replace(&mut test, "/path/to/schema.toml", &path.to_string_lossy())?;
    support::click(&mut test, "Open standalone source")?;
    replace(&mut test, "schema_version", "invalid TOML [")?;
    support::click(&mut test, "Save source and regenerate")?;
    assert_eq!(std::fs::read_to_string(&path)?, original);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("schema.json"))?,
        generated
    );
    assert!(matches!(
        recite_writer::request_close(),
        CloseDecision::KeepOpen
    ));
    test.sync_and_update();
    assert!(has(&test, "Unsaved declaration source"));
    support::click(&mut test, "Save and regenerate declarations")?;
    assert_eq!(std::fs::read_to_string(&path)?, original);
    support::click(&mut test, "Discard declaration draft")?;
    support::click(&mut test, "Keep editing")?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("schema_version"))))
            .is_some()
    );
    let changed = original.replace("Mara", "External");
    std::fs::write(&path, &changed)?;
    support::click(&mut test, "Keep recovery copy and reload source")?;
    assert!(has(&test, "Previous draft retained"));
    support::click(&mut test, "Save source and regenerate")?;
    assert!(std::fs::read_to_string(dir.path().join("schema.json"))?.contains("External"));
    support::click(&mut test, "Back to declarations")?;
    assert!(has(&test, "Generated declarations match"));
    support::click(&mut test, "Edit declaration source")?;
    support::dark_theme(&mut test)?;
    assert!(
        test.find(|_, e| Paragraph::try_downcast(e)
            .filter(|p| p.spans.iter().any(|s| s.text.contains("External"))))
            .is_some()
    );
    Ok(())
}

#[test]
fn producer_registration_errors_remain_visible_and_reload_retries_the_disk_configuration()
-> TestResult {
    let dir = project()?;
    let registration = dir.path().join("recite.producer.toml");
    std::fs::write(&registration, "invalid producer [")?;
    let mut test = open(dir.path())?;
    assert!(!has(&test, "Regenerate declarations"));
    std::fs::write(
        &registration,
        "version=1\n[producer]\nkind='standalone'\nid='wrong-owner'\n[generate]\nprogram='/bin/true'\nargs=['{output}']\n",
    )?;
    support::click(&mut test, "Reload producer registration")?;
    assert!(has(&test, "different schema producer"));
    std::fs::remove_file(&registration)?;
    support::click(&mut test, "Reload producer registration")?;
    assert!(!has(&test, "different schema producer"));
    assert!(!has(&test, "Regenerate declarations"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn cancelling_a_registered_producer_does_not_replace_the_generated_manifest() -> TestResult {
    let dir = project()?;
    let original = std::fs::read_to_string(dir.path().join("schema.json"))?;
    let replacement = original.replace("Mara", "Changed");
    assert_ne!(replacement, original);
    assert!(
        recite_core::schema::load_schema_manifest_str("next.json", &replacement)
            .schema
            .is_some()
    );
    std::fs::write(dir.path().join("next.json"), replacement)?;
    std::fs::write(
        dir.path().join("recite.producer.toml"),
        "version=1\n[producer]\nkind='standalone'\nid='dialogue'\n[generate]\nprogram='/bin/sh'\nargs=['-c', 'sleep 10; cp next.json \"$1\"', 'producer', '{output}']\n",
    )?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "Regenerate declarations")?;
    assert!(has(&test, "Generating declarations"));
    assert!(matches!(
        recite_writer::request_close(),
        CloseDecision::KeepOpen
    ));
    test.sync_and_update();
    assert!(has(&test, "Before closing Recite"));
    support::click(&mut test, "Cancel running jobs")?;
    support::click(&mut test, "Keep editing")?;
    for _ in 0..100 {
        test.poll_n(std::time::Duration::from_millis(25), 5);
        if !has(&test, "Generating declarations") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!has(&test, "Generating declarations"));
    assert!(has(&test, "Generation cancelled"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("schema.json"))?,
        original
    );
    support::click(&mut test, "Reload generated declarations")?;
    assert!(has(&test, "Mara"));
    Ok(())
}
