use std::error::Error;
use std::fs;
use std::process::Command;

use tempfile::TempDir;

#[test]
fn run_separates_an_unavailable_choice_marker_from_its_text() -> Result<(), Box<dyn Error>> {
    let temp = TempDir::new()?;
    let source = temp.path().join("dialogue.recite");
    fs::write(
        &source,
        ":: start default\n> @h0hekng\n  Welcome.\n  ? requires=(ready()) @answer\n    Answer.\n    -> END\n  ? @vs56zr5\n    Leave.\n    -> END\n",
    )?;
    let asset = temp.path().join("dialogue.recitec");
    let compiled = Command::new(env!("CARGO_BIN_EXE_recite"))
        .arg("compile")
        .arg(&source)
        .arg("-o")
        .arg(&asset)
        .output()?;
    assert!(compiled.status.success(), "{compiled:?}");
    let fixture = temp.path().join("fixture.toml");
    fs::write(
        &fixture,
        "[conditions]\n\"ready()\" = false\n\n[choices]\nh0hekng = \"vs56zr5\"\n",
    )?;
    let run = Command::new(env!("CARGO_BIN_EXE_recite"))
        .args(["run", "--block", "start", "--fixture"])
        .arg(&fixture)
        .arg(&asset)
        .output()?;
    assert!(run.status.success(), "{run:?}");
    let stdout = String::from_utf8(run.stdout)?;
    assert!(
        stdout.contains("  [1] answer: Answer. (unavailable)\n"),
        "{stdout}"
    );
    Ok(())
}
