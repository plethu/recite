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

fn team_project() -> Result<TempDir, Box<dyn Error>> {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/realistic/team-pack");
    let temp = TempDir::new()?;
    for directory in ["scenes", "schema/generated", "locales"] {
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

#[test]
fn status_reports_changed_and_new_source_and_check_fails_until_reviewed()
-> Result<(), Box<dyn Error>> {
    let project = team_project()?;
    let root = project.path();
    let clean = recite(root, &["catalog", "status", "--check"])?;
    assert!(clean.status.success(), "{clean:?}");
    assert_eq!(
        String::from_utf8(clean.stdout)?,
        "locales/fr.po (fr): 10 reviewed, 0 entries need attention\n"
    );

    edit(
        root,
        "scenes/deck.recite",
        "Keep the lantern low.",
        "Keep the lantern low and steady.",
    )?;
    edit(
        root,
        "scenes/deck.recite",
        "! blocking",
        "> @deck_gulls\n  Gulls settle on the rail.\n! blocking",
    )?;
    let text = recite(root, &["catalog", "status"])?;
    assert!(text.status.success(), "{text:?}");
    assert_eq!(
        String::from_utf8(text.stdout)?,
        concat!(
            "locales/fr.po (fr): 9 reviewed, 2 entries need attention\n",
            "  out of date with the source; run `recite catalog refresh`\n",
            "  source changed: `deck_crossing` at scenes/deck.recite:3:3\n",
            "  missing: `deck_gulls` at scenes/deck.recite:5:3\n",
        )
    );

    let checked = recite(root, &["catalog", "status", "--check", "--format", "json"])?;
    assert_eq!(checked.status.code(), Some(1), "{checked:?}");
    let report: Value = serde_json::from_slice(&checked.stdout)?;
    let catalog = &report["catalogs"][0];
    assert_eq!(catalog["path"], "locales/fr.po");
    assert_eq!(catalog["locale"], "fr");
    assert_eq!(catalog["refresh_needed"], true);
    assert_eq!(catalog["counts"]["source_changed"], 1);
    assert_eq!(catalog["counts"]["missing"], 1);
    assert_eq!(catalog["attention"][0]["context"], "deck_crossing");
    assert_eq!(catalog["attention"][0]["state"], "source_changed");
    assert_eq!(
        catalog["attention"][0]["reference"],
        "scenes/deck.recite:3:3"
    );
    Ok(())
}

#[test]
fn refresh_marks_changed_source_for_review_and_is_then_current() -> Result<(), Box<dyn Error>> {
    let project = team_project()?;
    let root = project.path();
    edit(
        root,
        "scenes/deck.recite",
        "Keep the lantern low.",
        "Keep the lantern low and steady.",
    )?;
    let refreshed = recite(root, &["catalog", "refresh"])?;
    assert!(refreshed.status.success(), "{refreshed:?}");
    assert_eq!(
        String::from_utf8(refreshed.stderr)?,
        "refreshed locales/fr.po\n"
    );
    let catalog = fs::read_to_string(root.join("locales/fr.po"))?;
    assert!(catalog.contains(concat!(
        "#, fuzzy\n",
        "#| msgid \"Keep the lantern low. The current pulls hardest near the old pilings.\"\n",
        "msgctxt \"deck_crossing\"\n",
        "msgid \"Keep the lantern low and steady. The current pulls hardest near the old pilings.\"\n",
        "msgstr \"Garde la lanterne basse. Le courant tire le plus fort près des vieux pilotis.\"\n",
    )));

    let again = recite(root, &["catalog", "refresh"])?;
    assert!(again.status.success(), "{again:?}");
    assert_eq!(
        String::from_utf8(again.stderr)?,
        "locales/fr.po is already current\n"
    );
    assert_eq!(fs::read_to_string(root.join("locales/fr.po"))?, catalog);

    // Refreshed but still unreviewed: the check keeps failing until a person reviews it.
    let status = recite(root, &["catalog", "status", "--check"])?;
    assert_eq!(status.status.code(), Some(1));
    assert!(String::from_utf8(status.stdout)?.contains("source changed: `deck_crossing`"));
    Ok(())
}

#[test]
fn malformed_catalogue_is_refused_without_writing_anything() -> Result<(), Box<dyn Error>> {
    let project = team_project()?;
    let root = project.path();
    fs::write(root.join("locales/broken.po"), "msgid \"unterminated\n")?;
    let before = fs::read_to_string(root.join("locales/fr.po"))?;
    edit(
        root,
        "scenes/deck.recite",
        "Keep the lantern low.",
        "Keep it low.",
    )?;
    let refused = recite(root, &["catalog", "refresh"])?;
    assert!(!refused.status.success(), "{refused:?}");
    assert!(
        String::from_utf8(refused.stderr)?.contains("catalogue locales/broken.po is not valid PO")
    );
    assert_eq!(fs::read_to_string(root.join("locales/fr.po"))?, before);
    Ok(())
}

#[test]
fn catalog_commands_are_listed_under_localisation_help() -> Result<(), Box<dyn Error>> {
    let temp = TempDir::new()?;
    let help = recite(temp.path(), &["--help"])?;
    let text = String::from_utf8(help.stdout)?;
    let localisation = text.split("Localisation:").nth(1).ok_or("heading")?;
    assert!(localisation.contains("catalog"), "{text}");
    let status = recite(temp.path(), &["catalog", "status", "--help"])?;
    let text = String::from_utf8(status.stdout)?;
    assert!(
        text.contains("Report catalogue entries that are missing, untranslated or awaiting review")
    );
    assert!(text.contains("Fail when any entry needs attention or any catalogue needs refreshing"));
    Ok(())
}
