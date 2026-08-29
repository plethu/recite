use std::fs;

use recite_core::{PoDiagnosticKind, PoDocument, PoEdit, PoEditError, PoEntryField, PoWriteError};
use tempfile::TempDir;

use super::REPRESENTATIVE;

#[test]
fn plural_evaluator_rejects_unbounded_expression_growth() {
    let expression = format!("{}n", "n || ".repeat(512));
    let header = format!("nplurals=2; plural=({expression});");
    assert!(matches!(
        recite_core::evaluate_plural_form(&header, 1),
        Err(recite_core::PluralRuleError::InvalidHeader)
    ));
}

#[test]
fn header_braces_are_not_dialogue_placeholders_and_field_order_is_checked() {
    let header = concat!("msgid \"\"\nmsgstr \"\"\n", "\"Project: {name}\\n\"\n",);
    assert!(PoDocument::parse(header).is_ok());
    let out_of_order = "msgid \"Hello\"\nmsgctxt \"11111111111111111111\"\nmsgstr \"\"\n";
    assert!(matches!(
        PoDocument::parse(out_of_order)
            .expect_err("gettext field order is checked")
            .kind(),
        PoDiagnosticKind::InvalidFieldOrder(_)
    ));
}

#[test]
fn duplicate_active_catalogue_keys_are_rejected() {
    let source = concat!(
        "msgctxt \"11111111111111111111\"\nmsgid \"same\"\nmsgstr \"one\"\n\n",
        "msgctxt \"11111111111111111111\"\nmsgid \"same\"\nmsgstr \"two\"\n",
    );
    assert!(matches!(
        PoDocument::parse(source)
            .expect_err("duplicate key is rejected")
            .kind(),
        PoDiagnosticKind::DuplicateKey(_)
    ));
}

#[test]
fn adjacent_records_and_gnu_control_escapes_round_trip() {
    let source = concat!(
        "msgctxt \"11111111111111111111\"\nmsgid \"first\"\nmsgstr \"one\"\n",
        "msgctxt \"22222222222222222222\"\nmsgid \"second\"\nmsgstr \"\\001\\177\"\n",
    );
    let mut document = PoDocument::parse(source).expect("adjacent records parse");
    assert_eq!(document.entries().len(), 2);
    assert_eq!(document.entries()[1].translation(), Some("\u{1}\u{7f}"));
    document
        .apply_edit(PoEdit::translation(
            document.entries()[1].id(),
            "\u{1}\u{1b}",
        ))
        .expect("control edit succeeds");
    assert!(document.source().contains("\\001\\033"));
    assert_eq!(
        PoDocument::parse(document.source())
            .expect("edited document reparses")
            .entries()[1]
            .translation(),
        Some("\u{1}\u{1b}")
    );
}

#[test]
fn gettext_msgfmt_fixture_with_adjacent_records_parses() {
    let source = include_str!("../fixtures/po-adjacent.po");
    let document = PoDocument::parse(source).expect("msgfmt-checked fixture parses");
    assert_eq!(document.entries().len(), 6);
    assert_eq!(
        document.entries()[2].plural_translation(1),
        Some("Deuxièmes")
    );
    assert!(!document.entries()[3].is_header());
    assert_eq!(
        document.entries()[3].context(),
        Some("33333333333333333333")
    );
    assert_eq!(document.entries()[3].source_text(), "");
    assert!(document.find("33333333333333333333", "").is_some());
    assert!(document.entries()[4].flags().contains(&"fuzzy".to_owned()));
    assert!(document.entries()[5].is_obsolete());
}

#[test]
fn batch_edits_validate_once_and_roll_back_on_failure() {
    let source = concat!(
        "msgctxt \"11111111111111111111\"\n",
        "msgid \"Hello {name}\"\nmsgstr \"Bonjour {name}\"\n",
    );
    let mut document = PoDocument::parse(source).expect("document parses");
    let id = document.entries()[0].id();
    document
        .apply_edits([
            PoEdit::new(id, PoEntryField::SourceText, "Hi {name}"),
            PoEdit::translation(id, "Salut {name}"),
        ])
        .expect("coupled edits validate as a batch");
    assert!(document.source().contains("msgid \"Hi {name}\""));
    assert!(document.source().contains("msgstr \"Salut {name}\""));
    let before = document.source().to_owned();
    let error = document
        .apply_edits([
            PoEdit::new(id, PoEntryField::SourceText, "Changed {name}"),
            PoEdit::translation(id, "Wrong {other}"),
        ])
        .expect_err("invalid batch is rejected");
    assert!(matches!(error, PoEditError::InvalidDocument(_)));
    assert_eq!(document.source(), before);
}

#[test]
fn semantic_diagnostics_point_at_the_offending_field_value() {
    let source = "msgctxt \"bad\"\nmsgid \"{name}\"\nmsgstr \"{other}\"\n";
    let error = PoDocument::parse(source).expect_err("bad context is rejected");
    assert_eq!(error.line(), 1);
    assert!(error.column() > 1);
    assert!(error.diagnostic().span.end.is_some());
}

#[cfg(unix)]
#[test]
fn writes_reject_symlinks_and_preserve_mode() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("catalog.po");
    let target = temp.path().join("target.po");
    let source = "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"\"\n";
    fs::write(&target, source).expect("write target");
    fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).expect("set mode");
    symlink(&target, &path).expect("create symlink");
    let document = PoDocument::read(&target).expect("read target");
    assert!(matches!(
        document.write_atomically(&path, &document.fingerprint()),
        Err(PoWriteError::Symlink { .. })
    ));
    let actual = fs::metadata(&target).expect("stat target");
    assert_eq!(actual.permissions().mode() & 0o777, 0o640);
}

#[test]
fn stale_write_is_structured_and_does_not_clobber_external_content() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("catalog.po");
    fs::write(&path, REPRESENTATIVE).expect("write fixture");
    let mut document = PoDocument::read(&path).expect("load fixture");
    let expected = document.fingerprint();
    fs::write(&path, "external change\n").expect("external edit");
    document
        .apply_edit(PoEdit::translation(
            document.entries()[0].id(),
            "Salut {name}\nmonde",
        ))
        .expect("edit remains valid");
    let error = document
        .write_atomically(&path, &expected)
        .expect_err("stale content conflicts");
    assert!(matches!(error, PoWriteError::Conflict { .. }));
    assert_eq!(
        fs::read_to_string(&path).expect("read file"),
        "external change\n"
    );
    assert_eq!(
        fs::read_dir(temp.path()).expect("read directory").count(),
        2,
        "failed writes clean up temporary files"
    );
}

#[test]
fn cooperative_writers_serialize_and_one_stale_writer_conflicts() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("catalog.po");
    let source = "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"\"\n";
    fs::write(&path, source).expect("write fixture");
    let first = PoDocument::read(&path).expect("load fixture");
    let expected = first.fingerprint();
    let mut second = first.clone();
    let id = first.entries()[0].id();
    let mut first = first;
    first
        .apply_edit(PoEdit::translation(id, "One"))
        .expect("first edit");
    second
        .apply_edit(PoEdit::translation(id, "Two"))
        .expect("second edit");
    let path_one = path.clone();
    let expected_one = expected.clone();
    let first_thread = std::thread::spawn(move || first.write_atomically(path_one, &expected_one));
    let path_two = path.clone();
    let second_thread = std::thread::spawn(move || second.write_atomically(path_two, &expected));
    let first_result = first_thread.join().expect("first writer joins");
    let second_result = second_thread.join().expect("second writer joins");
    assert_eq!(
        usize::from(first_result.is_ok()) + usize::from(second_result.is_ok()),
        1
    );
    assert!(matches!(
        (first_result, second_result),
        (Err(PoWriteError::Conflict { .. }), Ok(_)) | (Ok(_), Err(PoWriteError::Conflict { .. }))
    ));
}

#[test]
fn successful_write_replaces_the_file_and_returns_new_fingerprint() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("catalog.po");
    fs::write(
        &path,
        "msgctxt \"33333333333333333333\"\nmsgid \"Hello\"\nmsgstr \"\"\n",
    )
    .expect("write fixture");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).expect("set mode");
    }
    fs::write(
        temp.path().join(".catalog.po.recite.lock"),
        "stale lock marker",
    )
    .expect("seed stale lock marker");
    let mut document = PoDocument::read(&path).expect("load fixture");
    let expected = document.fingerprint();
    document
        .apply_edit(PoEdit::translation(document.entries()[0].id(), "Bonjour"))
        .expect("edit remains valid");
    let written = document
        .write_atomically(&path, &expected)
        .expect("atomic write succeeds");
    assert_eq!(written, document.fingerprint());
    assert_eq!(
        fs::read_to_string(&path).expect("read file"),
        document.source()
    );
    assert_eq!(
        fs::read_dir(temp.path()).expect("read directory").count(),
        2
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path)
                .expect("stat written file")
                .permissions()
                .mode()
                & 0o777,
            0o640
        );
    }
}

#[test]
fn malformed_po_reports_structured_kinds_without_flattening() {
    let cases = [
        (
            "msgctxt \"11111111111111111111\"\nmsgid \"{name}\"\nmsgstr \"{other}\"\n",
            PoDiagnosticKind::PlaceholderMismatch(String::new()),
        ),
        (
            "msgctxt \"bad&variant&extra\"\nmsgid \"x\"\nmsgstr \"\"\n",
            PoDiagnosticKind::InvalidStableId(String::new()),
        ),
        (
            "msgctxt \"bad&\"\nmsgid \"x\"\nmsgstr \"\"\n",
            PoDiagnosticKind::InvalidStableId(String::new()),
        ),
        (
            "msgid \"x\"\nmsgid_plural \"xs\"\nmsgstr[1] \"\"\n",
            PoDiagnosticKind::InvalidPluralArms(String::new()),
        ),
        (
            "msgid \"x\"\nmsgid_plural \"xs\"\nmsgstr[arm] \"\"\n",
            PoDiagnosticKind::InvalidPluralArms(String::new()),
        ),
        (
            "msgid \"\"\nmsgstr \"\"\n\"not a header\\n\"\n",
            PoDiagnosticKind::InvalidHeader(String::new()),
        ),
    ];
    for (source, expected) in cases {
        let error = PoDocument::parse(source).expect_err("malformed PO is rejected");
        assert!(std::mem::discriminant(error.kind()) == std::mem::discriminant(&expected));
        assert_eq!(error.diagnostic().span.file, "<po>");
    }
}
