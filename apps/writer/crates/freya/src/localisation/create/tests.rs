use super::*;

#[test]
fn new_catalogues_have_target_plural_arms_and_no_translations() -> Result<(), String> {
    let template = "#. block: opening\n#. speaker: narrator\nmsgctxt \"11111111111111111111\"\nmsgid \"One letter\"\nmsgid_plural \"{count} letters\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"Hello\"\nmsgstr \"\"\n";
    for (tag, arms) in [("ja", 1), ("fr-CA", 2), ("ru", 3), ("pl", 3), ("en", 2)] {
        let locale = language(tag)?;
        let document = initialise(template, &locale, &AtomicBool::new(false))?;
        assert!(
            document
                .headers()
                .iter()
                .any(|h| h.key() == "Language" && h.value() == tag)
        );
        assert!(
            !document
                .headers()
                .iter()
                .any(|h| h.key() == "Last-Translator")
        );
        let plural = document
            .find("11111111111111111111", "One letter")
            .ok_or("plural entry")?;
        assert_eq!(plural.plural_translations().len(), arms, "{tag}");
        assert!(
            plural
                .plural_translations()
                .iter()
                .all(|t| t.text().is_empty())
        );
        assert_eq!(
            document
                .find("22222222222222222222", "Hello")
                .and_then(|e| e.translation()),
            Some("")
        );
        assert!(document.source().contains("#. speaker: narrator"));
    }
    Ok(())
}

#[test]
fn creation_never_overwrites_existing_files_or_symlinks() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("locale/fr.po");
    let document =
        PoDocument::parse("msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"\"\n")
            .map_err(|e| e.to_string())?;
    let created = persist(&document, &path)?;
    assert_eq!(created.path, path);
    assert_eq!(created.document.source(), document.source());
    assert!(persist(&document, &path).is_err());
    assert_eq!(
        fs::read_to_string(&path).map_err(|e| e.to_string())?,
        document.source()
    );
    #[cfg(unix)]
    {
        let link = dir.path().join("link.po");
        std::os::unix::fs::symlink(&path, &link).map_err(|e| e.to_string())?;
        assert!(persist(&document, &link).is_err());
        assert!(
            fs::symlink_metadata(link)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_symlink()
        );
    }
    Ok(())
}

#[test]
fn locale_is_explicit_and_cannot_escape_the_suggested_directory() {
    for invalid in [
        "",
        "und",
        "../../fr",
        "fr\nLanguage: en",
        "system",
        "burger",
        "fr-AB",
        "en-Qwer",
        "zz",
    ] {
        assert!(language(invalid).is_err(), "{invalid}");
    }
    assert_eq!(
        language(" fr-ca ").map(|l| l.to_string()),
        Ok("fr-CA".into())
    );
    assert_eq!(
        suggested_path(Path::new("project"), "fr-CA"),
        PathBuf::from("project/locale/fr-CA.po")
    );
}

#[test]
fn singular_catalogues_do_not_require_a_plural_database() -> Result<(), String> {
    let document = initialise(
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"\"\n",
        &language("ar")?,
        &AtomicBool::new(false),
    )?;
    assert!(
        document
            .headers()
            .iter()
            .any(|h| h.key() == "Language" && h.value() == "ar")
    );
    assert_eq!(
        document
            .find("11111111111111111111", "Hello")
            .and_then(|e| e.translation()),
        Some("")
    );
    Ok(())
}

#[test]
fn plural_metadata_is_required_and_cancelled_preparations_are_discarded() -> Result<(), String> {
    let template = "msgctxt \"11111111111111111111\"\nmsgid \"One\"\nmsgid_plural \"Many\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n";
    let neutral = PoDocument::parse(template).map_err(|e| e.to_string())?;
    assert_eq!(
        normalise(neutral, &language("fr")?).map(|_| ()),
        Err(wording(MsgId::WriterPluralUnknown))
    );
    assert!(initialise(template, &language("fr")?, &AtomicBool::new(true)).is_err());
    Ok(())
}

#[test]
fn cofi_catalogue_keeps_its_identity_and_uses_welsh_as_its_base() -> Result<(), String> {
    let template = "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"\"\n";
    let target = language("CY-x-COFI")?;
    assert_eq!(target.base().to_string(), "cy");
    let cofi = initialise(template, &target, &AtomicBool::new(false))?;
    assert!(
        cofi.headers()
            .iter()
            .any(|h| h.key() == "Language" && h.value() == "cy-x-cofi")
    );
    assert!(
        cofi.entries()
            .iter()
            .filter(|e| !e.is_header())
            .all(|e| e.translation() == Some(""))
    );
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = suggested_path(dir.path(), &language("cy-x-cofi")?.to_string());
    assert_eq!(
        path.file_name().and_then(|p| p.to_str()),
        Some("cy-x-cofi.po")
    );
    let opened = persist(&cofi, &path)?;
    assert_eq!(opened.document.source(), cofi.source());
    for unsupported in ["burger", "en-x-pirate", "cy-x-other"] {
        assert!(language(unsupported).is_err(), "{unsupported}");
    }
    Ok(())
}
