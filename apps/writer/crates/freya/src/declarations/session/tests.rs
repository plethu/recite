use super::*;
const SOURCE: &str =
    "schema_version = 1\n[producer]\nid = \"dialogue\"\n[speakers.mara]\ndisplay_name = \"Mara\"\n";
fn session() -> Result<(tempfile::TempDir, Session), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let parsed = parse(Path::new("schema.toml"), SOURCE)?;
    let output = dir.path().join("schema.json");
    let output_baseline = parsed.export_json();
    std::fs::write(&output, &output_baseline)?;
    std::fs::write(dir.path().join("schema.toml"), SOURCE)?;
    let mut session = Session {
        registration: Ok(None),
        job: None,
        root: dir.path().to_owned(),
        output,
        output_baseline,
        schema: parsed.schema().clone(),
        source: None,
    };
    session.bind(&dir.path().join("schema.toml"))?;
    Ok((dir, session))
}
#[test]
fn source_validation_and_external_conflicts_preserve_generated_output()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut session) = session()?;
    let output = std::fs::read_to_string(&session.output)?;
    session.source.as_mut().ok_or("source")?.draft = "invalid TOML [".into();
    assert!(session.save_and_generate().is_err());
    assert_eq!(
        std::fs::read_to_string(dir.path().join("schema.toml"))?,
        SOURCE
    );
    assert_eq!(std::fs::read_to_string(&session.output)?, output);
    session.source.as_mut().ok_or("source")?.draft = SOURCE.replace("Mara", "Mari");
    std::fs::write(&session.output, "external")?;
    assert!(matches!(
        session.save_and_generate(),
        Err(FileError::Conflict)
    ));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("schema.toml"))?,
        SOURCE
    );
    assert!(session.dirty());
    Ok(())
}
#[test]
fn generation_keeps_source_comments_and_enforces_producer_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut session) = session()?;
    session.source.as_mut().ok_or("source")?.draft = SOURCE.replace("dialogue", "other");
    assert!(matches!(
        session.save_and_generate(),
        Err(FileError::SchemaOwnership)
    ));
    session.source.as_mut().ok_or("source")?.draft =
        format!("# authored comment\n{}", SOURCE.replace("Mara", "Mari"));
    assert!(!session.current());
    session.save_and_generate()?;
    assert!(session.current());
    assert!(!session.dirty());
    assert!(
        std::fs::read_to_string(&session.source.as_ref().ok_or("source")?.path)?
            .starts_with("# authored comment")
    );
    assert!(std::fs::read_to_string(&session.output)?.contains("Mari"));
    Ok(())
}

#[test]
fn reload_keeps_the_draft_before_accepting_external_source()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut session) = session()?;
    let draft = SOURCE.replace("Mara", "Draft");
    session.source.as_mut().ok_or("source")?.draft = draft.clone();
    let disk = SOURCE.replace("Mara", "External");
    std::fs::write(dir.path().join("schema.toml"), &disk)?;
    let recovery = session.reload_source()?;
    assert_eq!(std::fs::read_to_string(recovery)?, draft);
    assert_eq!(session.source.as_ref().ok_or("source")?.draft, disk);
    assert!(!session.dirty());
    session.save_and_generate()?;
    assert!(std::fs::read_to_string(&session.output)?.contains("External"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn explicit_generation_publishes_only_when_live_schema_is_unchanged()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut session) = session()?;
    let output = parse(Path::new("schema.toml"), &SOURCE.replace("Mara", "Updated"))?.export_json();
    std::fs::write(dir.path().join("fixture.json"), &output)?;
    std::fs::write(
        dir.path().join(recite_config::PRODUCER_REGISTRATION_FILE),
        r#"
version=1
[producer]
kind="standalone"
id="dialogue"
[generate]
program="/bin/sh"
args=["-c", "cp fixture.json \"$1\"", "producer", "{output}"]
"#,
    )?;
    session.reload_registration();
    assert!(!std::fs::read_to_string(&session.output)?.contains("Updated"));
    session.start_generation()?;
    std::fs::write(&session.output, "external edit")?;
    let result = loop {
        if let Some(result) = poll_generation(&mut session) {
            break result;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert!(result.is_err());
    assert_eq!(std::fs::read_to_string(&session.output)?, "external edit");
    session.reload_source()?;
    session.start_generation()?;
    loop {
        if let Some(result) = poll_generation(&mut session) {
            result?;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(std::fs::read_to_string(&session.output)?, output);
    Ok(())
}

fn poll_generation(session: &mut Session) -> Option<Result<(), String>> {
    let result = session.job.as_ref()?.poll()?;
    Some(session.finish_generation(result))
}

#[test]
fn declaration_recovery_restores_invalid_drafts_and_keeps_disk_conflicts()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut session) = session()?;
    let path = dir.path().join("schema.toml");
    let source = session.source.as_mut().ok_or("source")?;
    source.draft = "unfinished TOML [".into();
    source.queue_recovery()?;
    source.flush_recovery()?;
    session.source.take();
    std::fs::write(&path, SOURCE.replace("Mara", "External"))?;
    session.bind(&path)?;
    assert_eq!(
        session.source.as_ref().ok_or("source")?.draft,
        "unfinished TOML ["
    );
    session.source.as_mut().ok_or("source")?.draft = SOURCE.replace("Mara", "Recovered");
    assert!(matches!(
        session.save_and_generate(),
        Err(FileError::Conflict)
    ));
    assert!(std::fs::read_to_string(&path)?.contains("External"));
    session.discard()?;
    session.source.take();
    session.bind(&path)?;
    assert!(!session.dirty());
    assert!(
        session
            .source
            .as_ref()
            .ok_or("source")?
            .draft
            .contains("External")
    );
    Ok(())
}
