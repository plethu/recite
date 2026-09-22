use recite_compiler::{CatalogInput, CatalogResolutionPolicy};
use recite_core::{LocaleId, PoDocument, ScalarValue};
use recite_writer_model::{Document, Preview, PreviewSetup};

#[test]
fn trial_uses_fixed_plural_values_and_records_variant_locale_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let document = Document::new(
        ":: start default\n> tickets@12345678901234567890 bind=(count:int=$tickets)\n  {count} ticket\n  | {count} tickets\n-> END\n",
    )?;
    assert_eq!(document.preview_bindings()[0].value, "tickets");
    let po = PoDocument::parse(
        "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"12345678901234567890\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"{count} billet\"\nmsgstr[1] \"{count} billets\"\n",
    )?;
    let mut setup = PreviewSetup {
        policy: CatalogResolutionPolicy::new(Some(LocaleId::new("fr-CA")?))
            .with_variant("formal")?,
        catalogues: vec![CatalogInput::from_document(LocaleId::new("fr")?, po)?],
        ..PreviewSetup::default()
    };
    setup
        .values
        .insert("tickets".into(), ScalarValue::Integer(2));
    let mut preview = Preview::configured(&document, None, setup.clone())?;
    setup
        .values
        .insert("tickets".into(), ScalarValue::Integer(1));
    assert_eq!(preview.advance(None)?.text, "2 billets");
    let trace = preview
        .trace()
        .plural_line("12345678901234567890")
        .ok_or("plural trace")?;
    assert_eq!(trace.matched_locale.as_deref(), Some("fr"));
    assert_eq!(
        trace.matched_context.as_deref(),
        Some("12345678901234567890")
    );
    assert_eq!(trace.matched_arm, Some(1));
    assert_eq!(trace.attempts.len(), 4);
    let mut next = Preview::configured(&document, None, setup)?;
    assert_eq!(next.advance(None)?.text, "1 billet");
    Ok(())
}
