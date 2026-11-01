use super::*;
use crate::localisation::catalogue::Draft;
use std::fs;

const PO: &str = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"ffee6p8\"\nmsgid \"One letter\"\nmsgid_plural \"Many letters\"\nmsgstr[0] \"Une lettre\"\nmsgstr[1] \"Plusieurs lettres\"\n\nmsgctxt \"ffee6p8&formal\"\nmsgid \"One letter\"\nmsgid_plural \"Many letters\"\nmsgstr[0] \"Un courrier\"\nmsgstr[1] \"Plusieurs courriers\"\n\nmsgctxt \"rthh7ab\"\nmsgid \"Other\"\nmsgstr \"Autre\"\n";

fn fixture(source: &str) -> Result<(tempfile::TempDir, Catalogue), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("fr.po");
    fs::write(&path, source)?;
    Ok((directory, Catalogue::open(&path)?))
}

#[test]
fn wording_family_and_target_language_plural_examples_preserve_entry_identity() {
    let (_directory, catalogue) = fixture(PO).unwrap();
    let id = catalogue
        .document
        .entries()
        .iter()
        .find(|entry| entry.context() == Some("ffee6p8&formal"))
        .unwrap()
        .id();
    let details = catalogue.entry_details(id).unwrap();
    assert_eq!(
        details
            .wordings
            .iter()
            .map(|wording| wording.context.as_str())
            .collect::<Vec<_>>(),
        ["ffee6p8", "ffee6p8&formal"]
    );
    assert_eq!(details.wordings[1].entry, id);
    assert_eq!(details.wordings[1].caption, "formal");
    let PluralGuide::Valid { examples, rule } = &details.plural else {
        panic!("valid French rule")
    };
    assert_eq!(examples, &[vec![0, 1], vec![2, 3, 4, 5, 6]]);
    assert_eq!(rule, "nplurals=2; plural=(n > 1);");
    assert!(details.plural.form_caption(0).ends_with("0, 1"));
    assert!(details.plural.form_caption(1).ends_with("2, 3, 4, 5, 6"));
}

#[test]
fn repeated_typing_reuses_one_projection_and_changed_form_counts_invalidate_it() {
    let (directory, mut catalogue) = fixture(PO).unwrap();
    let id = catalogue
        .document
        .entries()
        .iter()
        .find(|entry| entry.context() == Some("ffee6p8"))
        .unwrap()
        .id();
    let original = catalogue.entry_details(id).unwrap();
    for index in 0..128 {
        catalogue.update(
            id,
            Draft {
                forms: vec![format!("Lettre {index}"), "Lettres".into()],
                reviewed: false,
            },
        );
        assert!(
            Arc::ptr_eq(&original, &catalogue.entry_details(id).unwrap()),
            "typing reuses guidance instead of scanning and reparsing it"
        );
    }
    assert_eq!(catalogue.forms(id).next(), Some("Lettre 127"));
    catalogue.update(
        id,
        Draft {
            forms: vec!["Un".into(), "Deux".into(), "Trois".into()],
            reviewed: true,
        },
    );
    let invalid = catalogue.entry_details(id).unwrap();
    assert!(!Arc::ptr_eq(&original, &invalid));
    assert!(!invalid.plural.valid());
    assert!(original.plural.valid(), "retained snapshots are immutable");
    assert!(catalogue.save(id).is_err());
    assert_eq!(
        fs::read_to_string(directory.path().join("fr.po")).unwrap(),
        PO
    );
}

#[test]
fn saved_metadata_replaces_guidance_and_missing_or_huge_rules_fail_without_header_sized_allocation()
{
    let (directory, mut catalogue) = fixture(PO).unwrap();
    let id = catalogue
        .document
        .entries()
        .iter()
        .find(|entry| entry.context() == Some("ffee6p8"))
        .unwrap()
        .id();
    let original = catalogue.entry_details(id).unwrap();
    let updated = PO.replace(
        "nplurals=2; plural=(n > 1);",
        "nplurals=2; plural=(n != 1);",
    );
    fs::write(directory.path().join("fr.po"), updated).unwrap();
    catalogue.reload().unwrap();
    let next = catalogue.entry_details(id).unwrap();
    assert!(!Arc::ptr_eq(&original, &next));
    assert!(next.plural.form_caption(0).ends_with("1"));
    assert!(next.plural.form_caption(1).ends_with("0, 2, 3, 4, 5"));
    assert!(!PluralGuide::prepare(None, 2).valid());
    assert!(!PluralGuide::prepare(Some("nplurals=999999999; plural=0;"), 2).valid());
    assert!(original.plural.form_caption(0).ends_with("0, 1"));
    assert!(
        catalogue.entry_details(PoEntryId::new(0)).is_none(),
        "headers cannot be edited as entries"
    );
}
