use recite_core::po::PoDocument;
use recite_writer_session::localisation::target::document_language;

#[test]
fn catalogue_language_normalises_gettext_tags_and_rejects_malformed_metadata() {
    let po = |tag: &str| {
        PoDocument::parse(format!("msgid \"\"\nmsgstr \"Language: {tag}\\n\"\n")).unwrap()
    };
    assert_eq!(
        document_language(&po("fr_CA")).unwrap().unwrap().as_str(),
        "fr-CA"
    );
    for tag in [
        "cy-x-cofi",
        "en-x-harbour",
        "en-US-u-ca-gregory",
        "i-klingon",
    ] {
        assert_eq!(document_language(&po(tag)).unwrap().unwrap().as_str(), tag);
    }
    assert_eq!(
        document_language(&po(" fr_CA ")).unwrap().unwrap().as_str(),
        "fr-CA"
    );
    assert!(document_language(&po("not/a/locale")).is_err());
    assert!(
        document_language(&PoDocument::parse("").unwrap())
            .unwrap()
            .is_none()
    );
}
