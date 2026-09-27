mod support;
use freya::prelude::*;
use freya_testing::prelude::*;
fn fill(
    test: &mut TestingRunner,
    contains: &str,
    text: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let area = test
        .find(|n, e| {
            Paragraph::try_downcast(e)
                .filter(|p| p.spans.iter().any(|s| s.text.contains(contains)))
                .map(|_| n.layout().area)
        })
        .ok_or_else(|| format!("Missing field {contains}"))?;
    test.click_cursor((f64::from(area.min_x() + 4.), f64::from(area.min_y() + 4.)));
    test.sync_and_update();
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
fn has(test: &TestingRunner, text: &str) -> bool {
    test.find(|_, e| {
        Label::try_downcast(e)
            .is_some_and(|l| l.text.contains(text))
            .then_some(())
            .or_else(|| {
                Paragraph::try_downcast(e)
                    .filter(|p| p.spans.iter().any(|s| s.text.contains(text)))
                    .map(|_| ())
            })
    })
    .is_some()
}

fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\nschema = \"schema.json\"\n[[scenes]]\nid = \"relay\"\nasset = \"build/relay.recitec\"\nblock = \"start\"\nparticipants = [\"mara\"]\n",
    )?;
    let schema = "schema_version = 1\n[producer]\nid = \"dialogue\"\n[speakers.mara]\ndisplay_name = \"Mara\"\n";
    std::fs::write(dir.path().join("schema.toml"), schema)?;
    let source = recite_core::schema::SchemaSource::load_str("schema.toml", schema)
        .source
        .ok_or("schema")?;
    std::fs::write(dir.path().join("schema.json"), source.export_json())?;
    std::fs::write(
        dir.path().join("a.recite"),
        ":: start default speaker=mara\n> line@11111111111111111111\n  Hello.\n-> END\n",
    )?;
    std::fs::write(
        dir.path().join("b.recite"),
        ":: elsewhere\n-> a.recite::start\n",
    )?;
    Ok(dir)
}
fn open(dir: &std::path::Path) -> Result<TestingRunner, Box<dyn std::error::Error>> {
    let mut test = TestingRunner::new(
        recite_writer::editor_app,
        Size2D::new(
            if std::env::var_os("RECITE_NARROW_CAPTURE").is_some() {
                1000.
            } else {
                1400.
            },
            1000.,
        ),
        |_| {},
        1.,
    )
    .0;
    fill(&mut test, "Project folder", &dir.to_string_lossy())?;
    support::click(&mut test, "Open project")?;
    test.poll_n(std::time::Duration::from_millis(25), 30);
    assert!(has(&test, "Saved to disk"));
    if std::env::var_os("RECITE_DARK_CAPTURE").is_some() {
        support::dark_theme(&mut test)?;
    }
    Ok(test)
}
#[test]
fn project_tools_keep_navigation_and_generated_output_read_only()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "Project")?;
    support::click(&mut test, "Declarations")?;
    assert!(has(&test, "dialogue"));
    assert!(has(&test, "Generated declarations are read-only"));
    if let Ok(path) = std::env::var("RECITE_DECLARATIONS_SCREENSHOT") {
        test.render_to_file(path);
    }
    support::click(&mut test, "Back")?;
    assert!(!has(&test, "Generated declarations are read-only"));
    support::click(&mut test, "Forward")?;
    assert!(has(&test, "Generated declarations are read-only"));
    fill(
        &mut test,
        "/path/to/schema.toml",
        &dir.path().join("schema.toml").to_string_lossy(),
    )?;
    support::click(&mut test, "Open standalone source")?;
    assert!(has(&test, "Save source and regenerate"));
    if let Ok(path) = std::env::var("RECITE_SCHEMA_SOURCE_SCREENSHOT") {
        test.render_to_file(path);
    }
    let changed_schema = "# author comment\nschema_version = 1\n[producer]\nid = \"dialogue\"\n[speakers.mara]\ndisplay_name = \"Mari\"\n";
    fill(&mut test, "schema_version", changed_schema)?;
    assert!(matches!(
        recite_writer::request_close(),
        CloseDecision::KeepOpen
    ));
    test.sync_and_update();
    assert!(has(&test, "Before closing Recite"));
    assert!(has(&test, "Unsaved declaration source"));
    support::click(&mut test, "Save and regenerate declarations")?;
    support::click(&mut test, "Keep editing")?;
    assert!(std::fs::read_to_string(dir.path().join("schema.json"))?.contains("Mari"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("schema.toml"))?,
        changed_schema
    );
    support::click(&mut test, "Back to declarations")?;
    assert!(has(&test, "Generated declarations match"));
    support::click(&mut test, "Return to writing")?;
    support::click(&mut test, "Project")?;
    support::click(&mut test, "Build scenes")?;
    assert!(has(&test, "relay · build/relay.recitec"));
    if let Ok(path) = std::env::var("RECITE_BUILD_SCREENSHOT") {
        test.render_to_file(path);
    }
    support::click(&mut test, "Build saved source")?;
    test.poll_n(std::time::Duration::from_millis(25), 60);
    assert!(dir.path().join("build/relay.recitec").is_file());
    assert!(has(&test, "Built build/relay.recitec"));
    Ok(())
}
#[test]
fn rename_review_changes_both_documents_and_undo_restores_them()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "Source")?;
    support::click(&mut test, "Rename beat")?;
    support::click(&mut test, "Beat to rename")?;
    support::click(&mut test, "start")?;
    fill(&mut test, "New beat name", "renamed")?;
    support::click(&mut test, "Review rename")?;
    assert!(has(&test, "recite.project.toml:"));
    assert!(has(&test, "a.recite:"));
    assert!(has(&test, "b.recite:"));
    if let Ok(path) = std::env::var("RECITE_RENAME_SCREENSHOT") {
        test.render_to_file(path);
    }
    support::click(&mut test, "Apply rename")?;
    assert!(has(&test, "Rename applied"));
    let manifest = dir.path().join("recite.project.toml");
    let before_settings = std::fs::read_to_string(&manifest)?;
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Project settings")?;
    support::click(&mut test, "Apply project changes")?;
    assert!(has(&test, "Save or undo the pending project rename"));
    assert_eq!(std::fs::read_to_string(&manifest)?, before_settings);
    support::click(&mut test, "Close settings")?;
    support::click(&mut test, "Save project")?;
    assert!(std::fs::read_to_string(dir.path().join("a.recite"))?.contains(":: renamed"));
    assert!(std::fs::read_to_string(dir.path().join("b.recite"))?.contains("a.recite::renamed"));
    assert!(
        std::fs::read_to_string(dir.path().join("recite.project.toml"))?
            .contains("block = \"renamed\"")
    );
    support::click(&mut test, "Undo")?;
    support::click(&mut test, "Save project")?;
    assert!(
        std::fs::read_to_string(dir.path().join("recite.project.toml"))?
            .contains("block = \"start\"")
    );
    assert!(std::fs::read_to_string(dir.path().join("a.recite"))?.contains(":: start"));
    assert!(std::fs::read_to_string(dir.path().join("b.recite"))?.contains("a.recite::start"));
    Ok(())
}

#[test]
fn save_before_build_failure_does_not_start_a_build() -> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    std::fs::write(dir.path().join("a.recite"), "# changed outside Recite\n")?;
    support::click(&mut test, "Project")?;
    support::click(&mut test, "Build scenes")?;
    support::click(&mut test, "Save changes and build")?;
    test.poll_n(std::time::Duration::from_millis(25), 30);
    assert!(!dir.path().join("build/relay.recitec").exists());
    assert!(has(&test, "changed on disk"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.recite"))?,
        "# changed outside Recite\n"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn registered_producer_runs_only_on_request_and_refreshes_declarations()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut generated: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.path().join("schema.json"))?)?;
    generated["producer"]["kind"] = "bevy".into();
    let original = serde_json::to_string(&generated)?;
    std::fs::write(dir.path().join("schema.json"), &original)?;
    std::fs::write(dir.path().join("adapter.rs"), "// adapter declaration\n")?;
    std::fs::write(
        dir.path().join("next.json"),
        original.replace("Mara", "New name"),
    )?;
    std::fs::write(
        dir.path().join("recite.producer.toml"),
        r#"
version=1
[producer]
kind="bevy"
id="dialogue"
[generate]
program="/bin/sh"
args=["-c", "cp next.json \"$1\"", "producer", "{output}"]
[editor]
program="/bin/sh"
args=["-c", "printf '%s' \"$1\" > editor-location", "editor", "{file}:{line}:{column}"]
[[sources]]
kind="speaker"
name="mara"
file="adapter.rs"
line=4
"#,
    )?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "Project")?;
    support::click(&mut test, "Declarations")?;
    assert!(has(&test, "adapter.rs:4:1"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("schema.json"))?,
        original
    );
    assert!(!has(&test, "Open standalone source"));
    assert!(!dir.path().join("editor-location").exists());
    support::click(&mut test, "Open declaration source")?;
    test.poll_n(std::time::Duration::from_millis(25), 30);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("editor-location"))?,
        format!("{}:4:1", dir.path().join("adapter.rs").display())
    );
    support::click(&mut test, "Regenerate declarations")?;
    test.poll_n(std::time::Duration::from_millis(25), 30);
    assert!(has(&test, "New name"));
    assert!(std::fs::read_to_string(dir.path().join("schema.json"))?.contains("New name"));
    Ok(())
}

#[test]
fn watched_external_change_returns_an_unsaved_editable_draft()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "Source")?;
    let original = std::fs::read_to_string(dir.path().join("a.recite"))?;
    let external = original.replace("Hello.", "Changed externally.");
    std::fs::write(dir.path().join("a.recite"), &external)?;
    test.poll_n(std::time::Duration::from_millis(25), 30);
    assert!(has(&test, "changed outside Recite"));
    support::click(&mut test, "Compare external changes")?;
    assert!(has(&test, "Changed externally."));
    support::click(&mut test, "Your draft")?;
    support::click(&mut test, "Use as editable draft")?;
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.recite"))?,
        external
    );
    assert!(has(&test, "Hello."));
    Ok(())
}

#[test]
fn project_settings_cannot_exclude_the_active_scene() -> Result<(), Box<dyn std::error::Error>> {
    let dir = fixture()?;
    let manifest = dir.path().join("recite.project.toml");
    let original = std::fs::read_to_string(&manifest)?;
    let mut test = open(dir.path())?;
    support::click(&mut test, "Settings")?;
    support::click(&mut test, "Project settings")?;
    fill(
        &mut test,
        "format_version",
        &format!("{original}\n[discovery]\nexcludes = ['a.recite']\n"),
    )?;
    support::click(&mut test, "Apply project changes")?;
    assert_eq!(std::fs::read_to_string(&manifest)?, original);
    assert!(has(&test, "Project changes would remove an open document"));
    support::click(&mut test, "Close settings")?;
    assert!(has(&test, "Hello."));
    Ok(())
}
