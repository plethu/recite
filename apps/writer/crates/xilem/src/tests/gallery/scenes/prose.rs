//! A project that checks prose: findings counted and listed with their fixes.
use super::*;
use recite_writer_app::prose::Checker;

// Two misspellings and one grammar slip in dialogue; the speaker and IDs stay unflagged.
const PROSE: &str = ":: harbour default\n> speaker=rhea @ffee6p8\n  The lantern flickerd above the pier.\n> speaker=rhea @rthh7ab\n  Bring the the lamp before the tide.\n> speaker=rhea @bqyvpqq\n  Nobody has stampd the ferry list.\n-> END\n";

/// A project with `[prose]` enabled, a project dictionary and one scene with findings.
fn with_prose() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("prose project folder");
    let path = directory.path();
    fs::write(
        path.join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"harbour\"\nversion = \"1\"\n[prose]\nenabled = true\ndictionaries = [\"words.txt\"]\n",
    )
    .expect("prose manifest");
    fs::write(path.join("harbour.recite"), PROSE).expect("prose scene");
    directory
}

/// Check the open document once, as the background worker does, before the stage mounts it.
fn checked(mut writer: Writer) -> Writer {
    let (sender, mut requests) = tokio::sync::mpsc::unbounded_channel();
    writer.prose.sender = Some(sender);
    writer.check_prose_if_needed();
    if let Ok(request) = requests.try_recv() {
        writer.accept_prose(request.execute(&mut Checker::default()));
    }
    writer
}

pub(in crate::tests::gallery) fn prose(record: &mut Recorder, variant: Variant) {
    record.scene(
        Meta {
            id: "script.prose-findings",
            group: "Script",
            title: "Script, prose findings with their fixes",
            intent: "Spelling and grammar findings join the problem count and the list under the manuscript, each offering its replacements and, for a spelling, adding the word to the project dictionary.",
        },
        |record| {
            let directory = with_prose();
            let writer = checked(open(directory.path(), "harbour.recite"));
            stage(App::with_writer(writer), variant).shot(record, "Prose findings");
        },
    );
}
