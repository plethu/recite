use recite_core::po::{PoDocument, PoEdit, PoEditError, PoEntryField, PoEntryId, PoUpdate};

#[test]
fn final_review_flags_preserve_lossless_source_and_allow_unreviewed_placeholders()
-> Result<(), Box<dyn std::error::Error>> {
    for ending in ["\n", "\r\n"] {
        let source = "# note\n#, fuzzy, custom-flag\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello {name}\"\nmsgstr \"Old\"\n\n# untouched\nmsgctxt \"22222222222222222222\"\nmsgid \"Bye {name}\"\nmsgstr \"Bye {name}\"".replace('\n', ending);
        let mut document = PoDocument::parse(&source)?;
        document.apply_updates([
            PoUpdate::Fuzzy {
                entry: PoEntryId::new(0),
                fuzzy: false,
            },
            PoUpdate::Field(PoEdit::translation(PoEntryId::new(1), "Incomplete")),
            PoUpdate::Field(PoEdit::translation(PoEntryId::new(0), "Salut {name}")),
            PoUpdate::Fuzzy {
                entry: PoEntryId::new(1),
                fuzzy: true,
            },
        ])?;
        let expected = "# note\n#, custom-flag\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello {name}\"\nmsgstr \"Salut {name}\"\n\n#, fuzzy\n# untouched\nmsgctxt \"22222222222222222222\"\nmsgid \"Bye {name}\"\nmsgstr \"Incomplete\"".replace('\n', ending);
        assert_eq!(document.source(), expected);
        assert_eq!(
            document
                .entry(PoEntryId::new(1))
                .and_then(|e| e.translation()),
            Some("Incomplete")
        );
        let before = document.clone();
        assert!(document.set_fuzzy(PoEntryId::new(1), false).is_err());
        assert_eq!(document, before);
    }
    Ok(())
}

#[test]
fn plural_batches_validate_every_final_arm_and_roll_back_together()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "msgid \"\"\nmsgstr \"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n#, fuzzy\nmsgctxt \"33333333333333333333\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n";
    let id = PoEntryId::new(1);
    let mut document = PoDocument::parse(source)?;
    assert!(
        document
            .apply_updates([
                PoUpdate::Field(PoEdit::plural_translation(id, 0, "{count} billet")),
                PoUpdate::Field(PoEdit::plural_translation(id, 1, "invalid")),
                PoUpdate::Fuzzy {
                    entry: id,
                    fuzzy: false
                },
            ])
            .is_err()
    );
    assert_eq!(document.source(), source);
    document.apply_updates([
        PoUpdate::Field(PoEdit::plural_translation(id, 0, "{count} billet")),
        PoUpdate::Field(PoEdit::plural_translation(id, 1, "{count} billets")),
        PoUpdate::Fuzzy {
            entry: id,
            fuzzy: false,
        },
    ])?;
    assert_eq!(
        document.source(),
        source
            .replace("#, fuzzy\n", "")
            .replace("msgstr[0] \"\"", "msgstr[0] \"{count} billet\"")
            .replace("msgstr[1] \"\"", "msgstr[1] \"{count} billets\"")
    );
    Ok(())
}

#[test]
fn duplicate_fields_review_flags_and_same_offset_insertions_are_rejected_atomically()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "msgid \"\"\nmsgstr \"Language: fr\\n\"\n";
    let id = PoEntryId::new(0);
    let mut document = PoDocument::parse(source)?;
    for updates in [
        vec![
            PoUpdate::Field(PoEdit::translation(id, "one")),
            PoUpdate::Field(PoEdit::translation(id, "two")),
        ],
        vec![
            PoUpdate::Fuzzy {
                entry: id,
                fuzzy: true,
            },
            PoUpdate::Fuzzy {
                entry: id,
                fuzzy: true,
            },
        ],
        vec![
            PoUpdate::Fuzzy {
                entry: id,
                fuzzy: false,
            },
            PoUpdate::Fuzzy {
                entry: id,
                fuzzy: true,
            },
        ],
        vec![
            PoUpdate::Field(PoEdit::new(id, PoEntryField::Context, "new")),
            PoUpdate::Field(PoEdit::new(id, PoEntryField::SourceText, "different")),
        ],
    ] {
        assert_eq!(
            document.apply_updates(updates),
            Err(PoEditError::OverlappingEdits)
        );
        assert_eq!(document.source(), source);
    }
    assert!(matches!(
        document.apply_updates([PoUpdate::Fuzzy {
            entry: PoEntryId::new(99),
            fuzzy: true
        }]),
        Err(PoEditError::EntryNotFound(_))
    ));
    assert_eq!(document.source(), source);
    Ok(())
}

#[test]
fn batches_can_swap_keys_and_preserve_obsolete_multiline_entries()
-> Result<(), Box<dyn std::error::Error>> {
    let source = "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"Hello\"\nmsgstr \"Salut\"\n\n#~ , fuzzy, custom\n#~ msgctxt \"44444444444444444444\"\n#~ msgid \"Old\"\n#~ msgstr \"\"\n#~ \"Ancien\"\n";
    let mut document = PoDocument::parse(source)?;
    document.apply_updates([
        PoUpdate::Field(PoEdit::new(
            PoEntryId::new(0),
            PoEntryField::Context,
            "22222222222222222222",
        )),
        PoUpdate::Field(PoEdit::new(
            PoEntryId::new(1),
            PoEntryField::Context,
            "11111111111111111111",
        )),
        PoUpdate::Fuzzy {
            entry: PoEntryId::new(2),
            fuzzy: false,
        },
        PoUpdate::Field(PoEdit::translation(PoEntryId::new(2), "Révisé\nencore")),
    ])?;
    assert_eq!(
        document.entries()[0].context(),
        Some("22222222222222222222")
    );
    assert_eq!(
        document.entries()[1].context(),
        Some("11111111111111111111")
    );
    assert!(document.source().contains("#~ , custom\n"));
    assert!(document.source().contains("#~ \"Révisé\\nencore\""));
    assert!(document.entries()[2].is_obsolete());
    assert_eq!(document.entries()[2].translation(), Some("Révisé\nencore"));
    Ok(())
}

#[test]
fn source_snapshots_share_owned_input_and_survive_document_edits()
-> Result<(), Box<dyn std::error::Error>> {
    let source =
        String::from("msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n");
    let pointer = source.as_ptr();
    let mut document = PoDocument::parse(source)?;
    let before = document.source_snapshot();
    let cloned = document.clone();
    assert_eq!(before.as_ptr(), pointer);
    assert!(std::sync::Arc::ptr_eq(&before, &cloned.source_snapshot()));
    document.apply_edit(PoEdit::translation(PoEntryId::new(0), "Salut"))?;
    assert_eq!(before.as_str(), cloned.source());
    assert!(!std::sync::Arc::ptr_eq(
        &before,
        &document.source_snapshot()
    ));
    assert_eq!(document.entries()[0].translation(), Some("Salut"));
    let mut independent = before.clone();
    std::sync::Arc::make_mut(&mut independent).clear();
    assert!(!cloned.source().is_empty());
    Ok(())
}
