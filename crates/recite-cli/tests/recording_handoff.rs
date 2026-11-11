use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn recite(root: &Path, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    Ok(Command::new(env!("CARGO_BIN_EXE_recite"))
        .args(args)
        .current_dir(root)
        .env_remove("RECITE_CONFIG")
        .output()?)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn team_project() -> Result<TempDir, Box<dyn Error>> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/realistic/team-pack");
    let temp = TempDir::new()?;
    for directory in ["scenes", "schema", "schema/generated", "locales"] {
        fs::create_dir_all(temp.path().join(directory))?;
        for entry in fs::read_dir(fixture.join(directory))? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::copy(
                    entry.path(),
                    temp.path().join(directory).join(entry.file_name()),
                )?;
            }
        }
    }
    fs::copy(
        fixture.join("recite.project.toml"),
        temp.path().join("recite.project.toml"),
    )?;
    Ok(temp)
}

fn edit(root: &Path, path: &str, old: &str, new: &str) -> Result<(), Box<dyn Error>> {
    let file = root.join(path);
    let text = fs::read_to_string(&file)?;
    assert_eq!(text.matches(old).count(), 1, "{path}: {old}");
    fs::write(file, text.replace(old, new))?;
    Ok(())
}

fn status(root: &Path, locale: &str) -> Result<Value, Box<dyn Error>> {
    let output = recite(
        root,
        &[
            "recording",
            "status",
            "--locale",
            locale,
            "--format",
            "json",
        ],
    )?;
    assert!(output.status.success(), "{output:?}");
    Ok(serde_json::from_slice(&output.stdout)?)
}

/// Export a French handoff, then fill in takes for the named lines as a studio would.
fn returned_sheet(root: &Path, recorded: &[&str]) -> Result<String, Box<dyn Error>> {
    let exported = recite(
        root,
        &[
            "recording",
            "export",
            "--locale",
            "fr",
            "--handoff",
            "fr-1",
            "--output",
            "handoff.csv",
        ],
    )?;
    assert!(exported.status.success(), "{exported:?}");
    let sheet = fs::read_to_string(root.join("handoff.csv"))?;
    fs::create_dir_all(root.join("audio"))?;
    let mut lines = sheet.lines().map(str::to_owned).collect::<Vec<_>>();
    for line in &mut lines[1..] {
        let id = line.split(',').next().unwrap_or_default().to_owned();
        if recorded.contains(&id.as_str()) {
            let take = format!("audio/{id}.wav");
            fs::write(root.join(&take), b"RIFF")?;
            // Columns: ..., take, by, date, note.
            *line = line.trim_end_matches(",,,,").to_owned() + &format!(",{take},Studio A,,");
        }
    }
    Ok(lines.join("\r\n") + "\r\n")
}

#[test]
fn export_writes_only_ready_items_and_only_when_asked() -> Result<(), Box<dyn Error>> {
    let project = team_project()?;
    let root = project.path();
    let preview = recite(root, &["recording", "export", "--locale", "fr"])?;
    assert!(preview.status.success(), "{preview:?}");
    assert!(stderr(&preview).starts_with("5 items selected for fr\n"));
    assert!(stderr(&preview).ends_with("nothing written; pass --output to write the handoff\n"));

    let written = recite(
        root,
        &[
            "recording",
            "export",
            "--locale",
            "fr",
            "--block",
            "dock",
            "--format",
            "json",
            "--output",
            "dock.json",
        ],
    )?;
    assert!(written.status.success(), "{written:?}");
    let handoff: Value = serde_json::from_str(&fs::read_to_string(root.join("dock.json"))?)?;
    let row = &handoff["rows"][0];
    assert_eq!(handoff["rows"].as_array().map(Vec::len), Some(1));
    assert_eq!(row["id"], "dock_greeting");
    assert_eq!(row["locale"], "fr");
    assert_eq!(row["speaker"], "pilot");
    assert_eq!(row["direction"], "tone=weary");
    assert_eq!(
        row["source_text"],
        "Fog on the river again. The ferry leaves when the bell stops."
    );
    assert!(
        row["text"]
            .as_str()
            .unwrap_or_default()
            .starts_with("Encore du brouillard")
    );
    assert!(
        row["revision"]
            .as_str()
            .unwrap_or_default()
            .starts_with("r1-")
    );

    let missing = recite(root, &["recording", "status", "--locale", "de"])?;
    assert!(!missing.status.success());
    assert!(stderr(&missing).contains("no catalogue declares Language `de`"));
    Ok(())
}

#[test]
fn import_validates_then_applies_once_and_direction_changes_make_recordings_stale()
-> Result<(), Box<dyn Error>> {
    let project = team_project()?;
    let root = project.path();
    let check = recite(root, &["recording", "status", "--locale", "fr", "--check"])?;
    assert_eq!(check.status.code(), Some(1));

    fs::write(
        root.join("returned.csv"),
        returned_sheet(root, &["dock_greeting", "deck_crossing"])?,
    )?;
    let preview = recite(
        root,
        &["recording", "import", "returned.csv", "--locale", "fr"],
    )?;
    assert!(preview.status.success(), "{preview:?}");
    assert_eq!(
        stderr(&preview),
        "2 new takes, 0 already recorded\nnothing applied; pass --apply to update recording/fr.toml\n"
    );
    assert!(!root.join("recording/fr.toml").exists());

    let applied = recite(
        root,
        &[
            "recording",
            "import",
            "returned.csv",
            "--locale",
            "fr",
            "--apply",
        ],
    )?;
    assert!(applied.status.success(), "{applied:?}");
    let sidecar = fs::read_to_string(root.join("recording/fr.toml"))?;
    assert!(sidecar.contains("[[items.dock_greeting.takes]]"));
    assert!(sidecar.contains("reference = \"audio/dock_greeting.wav\""));
    assert!(sidecar.contains("handoff = \"fr-1\""));
    assert!(sidecar.contains("by = \"Studio A\""));
    let report = status(root, "fr")?;
    assert_eq!(report["counts"]["recorded"], 2);
    assert_eq!(report["counts"]["to_record"], 3);

    let again = recite(
        root,
        &[
            "recording",
            "import",
            "returned.csv",
            "--locale",
            "fr",
            "--apply",
        ],
    )?;
    assert!(again.status.success(), "{again:?}");
    assert!(stderr(&again).starts_with("0 new takes, 2 already recorded\n"));
    assert_eq!(fs::read_to_string(root.join("recording/fr.toml"))?, sidecar);

    // A new direction changes the performance: the recording is stale, the translation is not.
    edit(
        root,
        "scenes/dock.recite",
        "tone=\"weary\"",
        "tone=\"urgent\"",
    )?;
    let report = status(root, "fr")?;
    assert_eq!(report["counts"]["stale"], 1);
    assert_eq!(report["attention"][0]["id"], "dock_greeting");
    assert_eq!(report["attention"][0]["state"], "stale");
    let catalog = recite(root, &["catalog", "status", "--check"])?;
    assert!(catalog.status.success(), "{catalog:?}");

    // The old sheet was handed off before the change; nothing from it is applied.
    let stale = recite(
        root,
        &[
            "recording",
            "import",
            "returned.csv",
            "--locale",
            "fr",
            "--apply",
        ],
    )?;
    assert_eq!(stale.status.code(), Some(1));
    assert!(stderr(&stale).contains("`dock_greeting` was handed off before the line changed"));
    assert_eq!(fs::read_to_string(root.join("recording/fr.toml"))?, sidecar);
    Ok(())
}

#[test]
fn reworded_source_leaves_its_translation_not_ready_for_recording() -> Result<(), Box<dyn Error>> {
    let project = team_project()?;
    let root = project.path();
    edit(
        root,
        "scenes/deck.recite",
        "Keep the lantern low.",
        "Keep the lantern lower.",
    )?;
    let report = status(root, "fr")?;
    assert_eq!(report["counts"]["not_ready"], 1);
    let source = recite(root, &["recording", "status", "--format", "json"])?;
    let source: Value = serde_json::from_slice(&source.stdout)?;
    assert_eq!(source["counts"]["to_record"], 5);
    assert_eq!(source["locale"], Value::Null);
    assert_eq!(source["sidecar"], "recording/source.toml");
    Ok(())
}

#[test]
fn import_refuses_rows_with_unknown_ids_and_missing_media() -> Result<(), Box<dyn Error>> {
    let project = team_project()?;
    let root = project.path();
    let mut sheet = returned_sheet(root, &["dock_greeting"])?;
    fs::remove_file(root.join("audio/dock_greeting.wav"))?;
    sheet.push_str("ghost_line,fr,,,,0,,,,,r1-0000000000000000,,audio/ghost.wav,,,\r\n");
    fs::write(root.join("returned.csv"), sheet)?;
    let refused = recite(
        root,
        &[
            "recording",
            "import",
            "returned.csv",
            "--locale",
            "fr",
            "--apply",
        ],
    )?;
    assert_eq!(refused.status.code(), Some(1));
    let text = stderr(&refused);
    assert!(
        text.contains(
            "`dock_greeting` names take audio/dock_greeting.wav, which is not in the project"
        ),
        "{text}"
    );
    assert!(
        text.contains("`ghost_line` is not a line in the source"),
        "{text}"
    );
    assert!(!root.join("recording/fr.toml").exists());
    Ok(())
}
