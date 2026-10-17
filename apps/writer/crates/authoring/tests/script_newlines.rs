use recite_writer_model::{Document, ScriptEntry};

#[test]
fn bare_cr_and_mixed_terminators_preserve_script_headings_across_prose_edits()
-> Result<(), Box<dyn std::error::Error>> {
    let prefix = ":: a default\r> a@11111111111111111111\r  A.\r-> END\r";
    let suffix = concat!(
        ":: b\n> b@22222222222222222222\n  B.\n",
        "! immediate mark_done()\n",
        ":if ready()\n  -> END\n:else\n  -> a\n",
    );
    for source in [
        format!("{prefix}{suffix}"),
        format!("{prefix}{suffix}").replace('\n', "\r"),
    ] {
        let mut document = Document::new(source)?;
        let initial = document.script_snapshot()?;
        assert!(
            matches!(&initial[1].entries[1], ScriptEntry::Effect(heading) if heading == "! immediate mark_done()")
        );
        assert!(
            matches!(&initial[1].entries[2], ScriptEntry::Group { heading, .. } if heading == ":if ready()")
        );
        document.replace_text(
            document.revision(),
            "22222222222222222222",
            "Café 🐧\nAnother line.",
        )?;
        let reopened = Document::new(document.source())?;
        assert_eq!(document.script_snapshot()?, reopened.script_snapshot()?);
        assert_eq!(document.passage_snapshot()?, reopened.passage_snapshot()?);
    }
    Ok(())
}
