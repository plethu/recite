#![cfg(test)]

use recite_core::{PoCommentKind, PoDiagnosticKind, PoDocument, PoEdit, PoEntryField};

#[path = "po/document_operations.rs"]
mod document_operations;
#[path = "po/plural_rule_validation.rs"]
mod plural_rule_validation;

const REPRESENTATIVE: &str = concat!(
    "# translator note\n",
    "#. extracted context\n",
    "#. source id: abc@11111111111111111111\n",
    "#: dialogue/start.recite:4\n",
    "#, fuzzy, c-format\n",
    "msgctxt \"abc12345678901234567&formal\"\n",
    "msgid \"Hello {name}\\nworld\"\n",
    "msgstr \"Bonjour {name}\\nmonde\"\n",
    "x-recite \"unknown\"\n",
    "#| msgid \"Old {name}\"\n",
    "\n",
    "#~ msgctxt \"obsolete\"\n",
    "#~ msgid \"Gone\"\n",
    "#~ msgstr \"Parti\"\n",
    "\n",
    "msgid \"\"\n",
    "msgstr \"\"\n",
    "\"Language: fr-FR\\n\"\n",
    "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
    "\n",
    "msgctxt \"22222222222222222222\"\n",
    "msgid \"One {count}\"\n",
    "msgid_plural \"Many {count}\"\n",
    "msgstr[0] \"Un {count}\"\n",
    "msgstr[1] \"Plusieurs {count}\"\n",
);

#[test]
fn representative_po_round_trips_and_exposes_source_records() {
    let document = PoDocument::parse_with_path("fixture.po", REPRESENTATIVE)
        .expect("representative PO parses");
    assert_eq!(document.source(), REPRESENTATIVE);
    assert_eq!(document.entries().len(), 4);

    let entry = &document.entries()[0];
    assert_eq!(entry.context(), Some("abc12345678901234567&formal"));
    assert_eq!(entry.stable_id_metadata(), Some("abc@11111111111111111111"));
    assert_eq!(entry.variant(), Some("formal"));
    assert_eq!(entry.source_text(), "Hello {name}\nworld");
    assert_eq!(entry.translation(), Some("Bonjour {name}\nmonde"));
    assert!(entry.flags().contains(&"fuzzy".to_owned()));
    assert_eq!(entry.unknown_fields()[0].keyword(), "x-recite");
    assert_eq!(entry.previous()[0].value(), "Old {name}");
    assert_eq!(entry.comments()[0].kind(), &PoCommentKind::Translator);
    assert!(document.entries()[1].is_obsolete());
    assert_eq!(document.headers()[0].key(), "Language");
    assert_eq!(
        document.entries()[3].plural_translation(1),
        Some("Plusieurs {count}")
    );
}

#[test]
fn targeted_edits_preserve_unrelated_source_bytes_and_order() {
    let mut document = PoDocument::parse(REPRESENTATIVE).expect("representative PO parses");
    let before = document.source().to_owned();
    document
        .apply_edit(PoEdit::new(
            document.entries()[0].id(),
            PoEntryField::Translation,
            "Salut {name}\nmonde",
        ))
        .expect("translation edit succeeds");
    let expected = before.replace(
        "msgstr \"Bonjour {name}\\nmonde\"",
        "msgstr \"\"\n\"Salut {name}\\nmonde\"",
    );
    assert_eq!(document.source(), expected);
    assert!(
        document
            .source()
            .contains("msgstr \"\"\n\"Salut {name}\\nmonde\"")
    );
    assert!(document.source().contains("#~ msgid \"Gone\""));
    assert!(
        document
            .source()
            .contains("msgstr[1] \"Plusieurs {count}\"")
    );
    assert_eq!(
        document.entries()[0].context(),
        Some("abc12345678901234567&formal")
    );
    assert_ne!(before, document.source());
}

#[test]
fn variants_and_plural_previous_values_are_structured_edits() {
    let source = concat!(
        "msgid \"\"\n",
        "msgstr \"\"\n",
        "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
        "msgctxt \"11111111111111111111&formal\"\n",
        "msgid \"One {count}\"\n",
        "msgid_plural \"Many {count}\"\n",
        "#| msgid_plural \"Older {count}\"\n",
        "#| msgstr[0] \"Ancien {count}\"\n",
        "msgstr[0] \"Un {count}\"\n",
        "msgstr[1] \"Plusieurs {count}\"\n",
    );
    let mut document = PoDocument::parse(source).expect("plural PO parses");
    assert_eq!(
        document.entries()[1].previous()[1].field().to_owned(),
        recite_core::PoPreviousField::PluralTranslation(0)
    );
    document
        .apply_edit(PoEdit::variant(document.entries()[1].id(), "casual"))
        .expect("variant edit succeeds");
    assert_eq!(
        document.entries()[1].context(),
        Some("11111111111111111111&casual")
    );
}

#[test]
fn obsolete_previous_values_support_standard_multiline_records() {
    let source = concat!(
        "#~ msgid \"Current\"\n",
        "#~| msgid \"\"\n",
        "#~| \"Previous \\\"value\\\"\"\n",
        "#~ msgstr \"\"\n",
    );
    let mut document = PoDocument::parse(source).expect("obsolete multiline PO parses");
    assert!(document.entries()[0].is_obsolete());
    assert_eq!(
        document.entries()[0].previous()[0].value(),
        "Previous \"value\""
    );
    document
        .apply_edit(PoEdit::translation(document.entries()[0].id(), "Updated"))
        .expect("obsolete translation edit succeeds");
    assert!(document.source().contains("#~ msgstr \"Updated\""));
}

#[test]
fn plural_placeholder_validation_uses_the_corresponding_source_arm() {
    let source = concat!(
        "msgctxt \"11111111111111111111\"\n",
        "msgid \"{one}\"\n",
        "msgid_plural \"{many}\"\n",
        "msgstr[0] \"{many}\"\n",
        "msgstr[1] \"{many}\"\n",
    );
    let error = PoDocument::parse(source).expect_err("mismatched singular arm is rejected");
    assert!(matches!(
        error.kind(),
        PoDiagnosticKind::PlaceholderMismatch(detail) if detail.contains("missing {one}")
    ));
}

#[test]
fn stale_entries_are_preserved_without_semantic_validation() {
    let source = concat!(
        "#, fuzzy\n",
        "msgctxt \"not-a-stable-id\"\n",
        "msgid \"{source}\"\n",
        "msgid_plural \"{sources}\"\n",
        "msgstr[1] \"{wrong}\"\n",
    );
    let document = PoDocument::parse(source).expect("stale entry is retained");
    assert!(document.entries()[0].flags().contains(&"fuzzy".to_owned()));
    assert_eq!(document.source(), source);
}

#[test]
fn active_plural_entries_require_header_and_matching_arms() {
    let missing = concat!(
        "msgctxt \"11111111111111111111\"\n",
        "msgid \"one\"\n",
        "msgid_plural \"many\"\n",
        "msgstr[0] \"un\"\n",
    );
    assert!(matches!(
        PoDocument::parse(missing)
            .expect_err("header is required")
            .kind(),
        PoDiagnosticKind::InvalidHeader(_)
    ));
    let partial = concat!(
        "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
        "msgctxt \"11111111111111111111\"\nmsgid \"one\"\n",
        "msgid_plural \"many\"\nmsgstr[0] \"un\"\n",
    );
    assert!(matches!(
        PoDocument::parse(partial)
            .expect_err("partial header is rejected")
            .kind(),
        PoDiagnosticKind::InvalidPluralArms(_)
    ));
    let malformed = concat!(
        "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=2; plural=foo;\\n\"\n\n",
        "msgctxt \"11111111111111111111\"\nmsgid \"one\"\n",
        "msgid_plural \"many\"\nmsgstr[0] \"un\"\n",
        "msgstr[1] \"many\"\n",
    );
    assert!(matches!(
        PoDocument::parse(malformed)
            .expect_err("malformed plural expression is rejected")
            .kind(),
        PoDiagnosticKind::InvalidHeader(_)
    ));
    for duplicate in [
        "nplurals=2; nplurals=2; plural=(n != 1);",
        "nplurals=2; plural=(n != 1); plural=(n != 1);",
        "nplurals=garbage; nplurals=2; plural=(n != 1);",
        "nplurals=2; plural=not-an-expression; plural=(n != 1);",
    ] {
        let source = format!("msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: {duplicate}\\n\"\n");
        assert!(matches!(
            PoDocument::parse(source)
                .expect_err("duplicate plural metadata is rejected")
                .kind(),
            PoDiagnosticKind::InvalidHeader(_)
        ));
    }
}

#[test]
fn active_plural_entries_support_two_three_and_more_locale_arms() {
    for (nplurals, arms) in [(2, 2), (3, 3), (4, 4)] {
        let translations = (0..arms)
            .map(|arm| format!("msgstr[{arm}] \"arm {arm}\"\n"))
            .collect::<String>();
        let source = format!(
            "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals={nplurals}; plural=(n != 1);\\n\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"one\"\nmsgid_plural \"many\"\n{translations}"
        );
        let document = PoDocument::parse(&source).expect("locale plural arm count parses");
        assert_eq!(document.entries()[1].plural_translations().len(), arms);
    }
}

#[test]
fn plural_evaluator_handles_common_locale_rules_with_short_circuiting() {
    let rule = "nplurals=3; plural=(n == 0 ? 0 : n == 1 ? 1 : 2);";
    assert_eq!(recite_core::evaluate_plural_form(rule, 0), Ok(0));
    assert_eq!(recite_core::evaluate_plural_form(rule, 1), Ok(1));
    assert_eq!(recite_core::evaluate_plural_form(rule, 8), Ok(2));
    assert_eq!(
        recite_core::evaluate_plural_form("nplurals=2; plural=(n == 0 || 1 / 0);", 0),
        Ok(1)
    );
}

#[test]
fn plural_evaluator_rejects_negative_count_and_invalid_arm() {
    assert!(matches!(
        recite_core::evaluate_plural_form("nplurals=2; plural=(n == 1 ? 3 : 1);", 1),
        Err(recite_core::PluralRuleError::ArmOutOfRange { .. })
    ));
    assert!(matches!(
        recite_core::evaluate_plural_form("nplurals=2; plural=(n != 1);", -1),
        Err(recite_core::PluralRuleError::NegativeCount)
    ));
}

#[test]
fn plural_evaluator_rejects_identifier_prefixes_as_numbers() {
    assert_eq!(
        recite_core::evaluate_plural_form("nplurals=2; plural=n1;", 1),
        Err(recite_core::PluralRuleError::InvalidHeader)
    );
    assert_eq!(
        recite_core::evaluate_plural_form("nplurals=2; plural= n1;", 1),
        Err(recite_core::PluralRuleError::InvalidHeader)
    );
}

#[test]
fn locale_neutral_pot_plural_templates_do_not_require_locale_metadata() {
    let source = concat!(
        "msgctxt \"11111111111111111111\"\n",
        "msgid \"one\"\n",
        "msgid_plural \"many\"\n",
        "msgstr[0] \"\"\n",
        "msgstr[1] \"\"\n",
    );
    let document = PoDocument::parse(source).expect("empty plural POT template parses");
    assert!(document.headers().is_empty());
    assert_eq!(document.entries()[0].plural_translations().len(), 2);
    let translated = concat!(
        "msgid \"\"\nmsgstr \"\"\n",
        "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
        "msgctxt \"11111111111111111111\"\n",
        "msgid \"one\"\n",
        "msgid_plural \"many\"\n",
        "msgstr[0] \"un\"\n",
        "msgstr[1] \"plusieurs\"\n",
    );
    assert!(PoDocument::parse(translated).is_ok());
}
