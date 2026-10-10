use recite_writer_model::{Document, ScriptEntry, View, WRITER_EXAMPLES};

#[test]
fn hub_script_edits_prose_without_losing_effects_or_return_links()
-> Result<(), Box<dyn std::error::Error>> {
    let mut workbench = WRITER_EXAMPLES[0].open()?;
    assert!(matches!(workbench.view(), View::Passage(_)));
    let script = workbench.document().script()?;
    let accept = script
        .iter()
        .find(|block| block.id == "accept_search")
        .ok_or("accept block")?;
    assert!(
        matches!(&accept.entries[0], ScriptEntry::Effect(text) if text.contains("set_flag(courier_search, true)"))
    );
    assert!(
        matches!(accept.entries.last(), Some(ScriptEntry::Jump(target)) if target == "relay_desk")
    );
    let original = workbench.draft().to_owned();
    let source = workbench.document().source().to_owned();
    workbench.set_draft("The transmitter is working again.".into());
    workbench.apply()?;
    assert_eq!(
        workbench.document().source(),
        source.replacen(&original, "The transmitter is working again.", 1)
    );
    workbench.select(View::Block("accept_search".into()))?;
    workbench.start_preview()?;
    assert_eq!(
        workbench.preview_page().ok_or("effect page")?.effects[0].function,
        "set_flag"
    );
    workbench.advance_preview(None)?;
    assert_eq!(
        workbench.preview_page().ok_or("effect page")?.effects[0].function,
        "change_disposition"
    );
    workbench.advance_preview(None)?;
    assert!(
        workbench
            .preview_page()
            .ok_or("line")?
            .text
            .starts_with("Bring her home.")
    );
    Ok(())
}

#[test]
fn conditional_prose_remains_editable_with_structure_visible()
-> Result<(), Box<dyn std::error::Error>> {
    let source = ":: start default\n:if has_flag(ready)\n  > ready@44000000000000000001 speaker=mara\n    Ready to go.\n  -> finish\n:else\n  > waiting@44000000000000000002 speaker=mara\n    Not yet.\n  -> END\n:: finish\n! immediate set_flag(done, true)\n-> END\n";
    let mut document =
        Document::open(recite_core::DocumentKey::new("conditional.recite")?, source)?;
    let script = document.script()?;
    assert!(
        matches!(&script[0].entries[0], ScriptEntry::Group { heading, entries } if heading == ":if has_flag(ready)" && entries.len() == 2)
    );
    document.replace_text(document.revision(), "44000000000000000001", "Let's go.")?;
    assert_eq!(
        document.source(),
        source.replace("Ready to go.", "Let's go.")
    );
    Ok(())
}

#[test]
fn effects_only_scene_can_open_and_return_to_script() -> Result<(), Box<dyn std::error::Error>> {
    let mut workbench = recite_writer_model::Workbench::new(
        ":: start default\n! immediate set_flag(done, true)\n-> END\n",
    )?;
    assert_eq!(workbench.view(), &View::Block("start".into()));
    workbench.select(View::Source)?;
    workbench.show_script()?;
    assert_eq!(workbench.view(), &View::Block("start".into()));
    Ok(())
}

#[test]
fn nested_script_headings_follow_source_lines_across_unicode_crlf_edit_and_undo()
-> Result<(), Box<dyn std::error::Error>> {
    let source = concat!(
        "# Café — an authored heading fixture.\n",
        ":: relay default\n",
        ":if has_flag(ready)\n",
        "  ! immediate set_flag(done, true)\n",
        "  :match mood(mara)\n",
        "    :case calm\n",
        "      ! immediate change_disposition(mara, 1)\n",
        "    :case _\n",
        "      ! immediate change_disposition(mara, -1)\n",
        ":else\n",
        "  ! immediate change_disposition(mara, 0)\n",
        "-> END\n",
    );
    let expected = vec![
        ScriptEntry::Group {
            heading: ":if has_flag(ready)".into(),
            entries: vec![
                ScriptEntry::Effect("! immediate set_flag(done, true)".into()),
                ScriptEntry::Group {
                    heading: ":match mood(mara)".into(),
                    entries: vec![
                        ScriptEntry::Group {
                            heading: ":case calm".into(),
                            entries: vec![ScriptEntry::Effect(
                                "! immediate change_disposition(mara, 1)".into(),
                            )],
                        },
                        ScriptEntry::Group {
                            heading: ":case _".into(),
                            entries: vec![ScriptEntry::Effect(
                                "! immediate change_disposition(mara, -1)".into(),
                            )],
                        },
                    ],
                },
            ],
        },
        ScriptEntry::Group {
            heading: "Otherwise".into(),
            entries: vec![ScriptEntry::Effect(
                "! immediate change_disposition(mara, 0)".into(),
            )],
        },
        ScriptEntry::Jump("END".into()),
    ];
    for newline in ["\n", "\r\n"] {
        let source = source.replace('\n', newline);
        let mut document = Document::new(&source)?;
        let original = document.script_snapshot()?;
        assert_eq!(original[0].entries, expected);
        assert!(std::sync::Arc::ptr_eq(
            &original,
            &document.script_snapshot()?
        ));
        let edited = format!("# An extra line shifts every heading.{newline}{source}");
        document.replace_source(document.revision(), edited)?;
        let changed = document.script_snapshot()?;
        assert_eq!(changed[0].entries, expected);
        assert!(!std::sync::Arc::ptr_eq(&original, &changed));
        assert!(document.undo()?);
        assert_eq!(document.source(), source);
        assert_eq!(document.script_snapshot()?, original);
    }
    Ok(())
}

#[test]
fn final_multiline_prose_remains_editable_without_a_trailing_newline()
-> Result<(), Box<dyn std::error::Error>> {
    for newline in ["\n", "\r\n"] {
        let source = [
            "# Café — preserve byte positions after Unicode.",
            ":: relay default",
            "> courier@44000000000000000001",
            "\tCafé 🐧",
            "\tمرحبا",
        ]
        .join(newline);
        let mut document = Document::new(&source)?;
        let original = document.script_snapshot()?;
        let ScriptEntry::Passage(passage) = &original[0].entries[0] else {
            return Err("final prose passage missing".into());
        };
        assert_eq!(passage.text, "Café 🐧\nمرحبا");
        document.replace_text(document.revision(), &passage.id, "Bonsoir 🐧\nأهلا")?;
        assert_eq!(
            document.source(),
            source
                .replace("Café 🐧", "Bonsoir 🐧")
                .replace("مرحبا", "أهلا")
        );
        assert!(document.undo()?);
        assert_eq!(document.source(), source);
        assert_eq!(document.script_snapshot()?, original);
    }
    Ok(())
}
