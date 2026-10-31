use recite_writer_model::{Document, EditError, FIXTURE};
use std::sync::Arc;

fn matches_reopened(document: &Document) -> Result<(), Box<dyn std::error::Error>> {
    let reopened = Document::new(document.source())?;
    assert_eq!(document.passage_snapshot()?, reopened.passage_snapshot()?);
    assert_eq!(document.script_snapshot()?, reopened.script_snapshot()?);
    Ok(())
}

#[test]
fn prose_edits_preserve_nested_structure_and_relocate_later_headings()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        ":: start default\n",
        ":if ready()\n",
        "  > @ffee6p8\n",
        "    Café.\n",
        "  ! immediate mark_done()\n",
        ":else\n",
        "  ? @rthh7ab\n",
        "    Later.\n",
        "    -> finish\n",
        ":: finish\n",
        "> @bqyvpqq\n",
        "  Goodbye.\n",
        "-> END\n",
    );
    for ending in ["\n", "\r\n", "\r"] {
        for warm_script in [false, true] {
            let mut document = Document::new(source.replace('\n', ending))?;
            let original = document.passage_snapshot()?;
            let original_script = warm_script
                .then(|| document.script_snapshot())
                .transpose()?;
            for (id, text) in [
                ("ffee6p8", "Café 🐧\nمرحبا"),
                ("rthh7ab", "A different reply.\nAnother paragraph."),
                ("bqyvpqq", "Goodbye e\u{301}."),
            ] {
                document.replace_text(document.revision(), id, text)?;
                assert_eq!(
                    document.find_passage(id)?.ok_or("missing passage")?.text,
                    text
                );
                matches_reopened(&document)?;
            }
            // Previously returned snapshots still describe their original revision.
            assert_eq!(original[0].text, "Café.");
            if let Some(script) = original_script {
                assert_ne!(script, document.script_snapshot()?);
            }
            for _ in 0..3 {
                assert!(document.undo()?);
                matches_reopened(&document)?;
            }
            assert_eq!(document.passage_snapshot()?, original);
            for _ in 0..3 {
                assert!(document.redo()?);
                matches_reopened(&document)?;
            }
        }
    }
    Ok(())
}

#[test]
fn editing_each_authored_passage_matches_full_reprojection()
-> Result<(), Box<dyn std::error::Error>> {
    for example in recite_writer_model::WRITER_EXAMPLES {
        let mut document = example.open()?.document().source().to_owned();
        let ids: Vec<_> = Document::new(&document)?
            .passage_snapshot()?
            .iter()
            .map(|p| p.id.clone())
            .collect();
        let mut accepted = Document::new(&document)?;
        accepted.script_snapshot()?;
        for id in ids {
            let old = accepted.find_passage(&id)?.ok_or("missing passage")?;
            let text = format!("{}\nCafé 🐧 — مرحبا", old.text);
            accepted.replace_text(accepted.revision(), &id, &text)?;
            matches_reopened(&accepted)?;
            assert_eq!(
                accepted.find_passage(&id)?.ok_or("missing passage")?.text,
                text
            );
            assert!(accepted.undo()?);
            assert_eq!(accepted.source(), document);
            assert!(accepted.redo()?);
            matches_reopened(&accepted)?;
            document = accepted.source().to_owned();
        }
    }
    Ok(())
}

#[test]
fn rejected_and_unchanged_prose_preserve_revision_history_and_snapshots()
-> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::new(FIXTURE)?;
    let script = document.script_snapshot()?;
    let passages = document.passage_snapshot()?;
    let passage = &passages[0];
    let revision = document.revision();
    document.replace_text(revision, &passage.id, &passage.text)?;
    assert_eq!(document.revision(), revision);
    assert!(!document.can_undo());
    for text in [
        "",
        "Hello\n",
        "Hello\n-> END",
        "> @z0g6c80\nHello",
        "Hello\r:: injected\n> @z0g6c80\nOther",
    ] {
        assert!(matches!(
            document.replace_text(revision, &passage.id, text),
            Err(EditError::NotProse)
        ));
        assert_eq!(document.source(), FIXTURE);
        assert_eq!(document.revision(), revision);
        assert!(!document.can_undo());
        assert!(Arc::ptr_eq(&script, &document.script_snapshot()?));
        assert!(Arc::ptr_eq(&passages, &document.passage_snapshot()?));
    }
    document.replace_text(revision, &passage.id, "Changed prose.")?;
    assert!(matches!(
        document.replace_text(revision, &passage.id, "Lost update."),
        Err(EditError::Stale)
    ));
    matches_reopened(&document)?;
    document.replace_source(document.revision(), "broken syntax")?;
    assert!(document.script_snapshot().is_err());
    assert!(document.passage_snapshot().is_err());
    assert!(document.undo()?);
    matches_reopened(&document)?;
    assert_eq!(
        document
            .find_passage(&passage.id)?
            .ok_or("missing passage")?
            .text,
        "Changed prose."
    );
    Ok(())
}

#[test]
fn duplicate_block_names_do_not_confuse_projection_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "# Comment before the first beat.\n",
        ":: same default\n> @ffee6p8\n  First.\n-> END\n",
        ":: same\n! immediate mark_done()\n",
        "> @rthh7ab\n  Second.\n-> END\n",
        ":: last\n? @bqyvpqq\n  Reply.\n  -> END\n",
    );
    let mut document = Document::new(source)?;
    assert!(!document.diagnostics().is_empty());
    let previous = document.script_snapshot()?;
    for (id, text) in [
        ("ffee6p8", "First, longer.\nAnd another line."),
        ("rthh7ab", "Second, shorter."),
        ("bqyvpqq", "Reply 🐧."),
    ] {
        document.replace_text(document.revision(), id, text)?;
        matches_reopened(&document)?;
    }
    assert_eq!(previous[0].id, "same");
    assert_eq!(previous[1].id, "same");
    Ok(())
}

#[test]
fn plural_source_id_aliases_keep_full_script_reprojection() -> Result<(), Box<dyn std::error::Error>>
{
    let source = concat!(
        ":: first default\n",
        "> bind=(count:int=$count) @ffee6p8\n",
        "  One letter.\n  | Many letters.\n-> END\n",
        ":: second\n> @ffee6p8\n  Editable.\n-> END\n",
    );
    for warm_script in [false, true] {
        let mut document = Document::new(source)?;
        if warm_script {
            document.script_snapshot()?;
        }
        document.replace_text(document.revision(), "ffee6p8", "Changed 🐧.\nAnother line.")?;
        matches_reopened(&document)?;
        assert!(document.undo()?);
        matches_reopened(&document)?;
    }
    Ok(())
}

#[test]
fn one_restart_region_can_replace_multiple_blocks_and_shift_later_passages()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        ":: first default\n> @ffee6p8\n  First.\n-> END\n",
        ":: second\n> @rthh7ab\n  Second.\n-> END\n",
        "  :: indented\n> @bqyvpqq\n  Third.\n-> END\n",
        ":: last\n> @z0g6c80\n  Last.\n-> END\n",
    );
    assert_eq!(recite_parser::source_regions(source).len(), 3);
    let mut document = Document::new(source)?;
    assert_eq!(document.script_snapshot()?.len(), 4);
    for id in ["rthh7ab", "bqyvpqq"] {
        for text in ["Longer café 🐧.\nTwo lines.\nThree lines.", "Short."] {
            document.replace_text(document.revision(), id, text)?;
            matches_reopened(&document)?;
        }
    }
    let last = document
        .source()
        .find(":: last")
        .ok_or("missing last block")?;
    let reordered = format!(
        "{}{}",
        &document.source()[last..],
        &document.source()[..last]
    );
    document.replace_source(document.revision(), reordered)?;
    matches_reopened(&document)?;
    for passage in document.passage_snapshot()?.iter() {
        assert_eq!(document.find_passage(&passage.id)?, Some(passage.clone()));
    }
    Ok(())
}
