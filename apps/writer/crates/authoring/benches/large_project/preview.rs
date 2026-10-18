//! Prepared catalogue startup; fixture construction stays outside operation counters.
use recite_compiler::authoring::{CatalogInput, CatalogResolutionPolicy};
use recite_core::{LocaleId, po::PoDocument};
use recite_writer_model::{Document, Preview, PreviewSetup};
use serde_json::Value;

pub(super) fn catalogue(rows: &mut Vec<Value>) -> Result<(), Box<dyn std::error::Error>> {
    let document = Document::new(concat!(
        ":: start default\n> prompt@000000000000000003e6\n  Wording 998.\n",
        "? reply@000000000000000003e7\n  Wording 999.\n  -> END\n",
    ))?;
    assert!(document.diagnostics().is_empty());
    let mut po = String::from("msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n");
    for n in 0..1_000 {
        po.push_str(&format!(
            "msgctxt \"{n:020x}\"\nmsgid \"Wording {n}.\"\nmsgstr \"Traduit {n}.\"\n\n"
        ));
    }
    let locale = LocaleId::new("fr")?;
    let setup = PreviewSetup {
        policy: CatalogResolutionPolicy::new(Some(locale.clone())),
        catalogues: vec![CatalogInput::from_document(
            locale,
            PoDocument::parse(&po)?,
        )?],
        ..PreviewSetup::default()
    };
    let mut preview = super::measure("preview_catalogue_start", rows, || {
        Preview::configured(&document, None, setup.clone())
            .map_err(Box::<dyn std::error::Error>::from)
    })?;
    let page = preview.advance(None)?;
    assert_eq!(page.text, "Traduit 998.");
    assert!(page.choices.is_empty());
    assert!(page.effects.is_empty());
    let page = preview.advance(None)?;
    assert_eq!(page.choices.len(), 1);
    assert_eq!(page.choices[0].text, "Traduit 999.");
    assert!(page.effects.is_empty());
    Ok(())
}

/// Repeated bindings are compact accepted facts, not a reason to parse again.
pub(super) fn bindings(rows: &mut Vec<Value>) -> Result<(), Box<dyn std::error::Error>> {
    let mut source = String::from(":: start default\n");
    for n in 0..1_000 {
        source.push_str(&format!(
            "> line_{n} bind=(name:string=$person)\n  Hello {{name}}.\n"
        ));
    }
    let document = Document::new(source)?;
    let bindings = super::measure("first_preview_bindings", rows, || {
        document.preview_bindings()
    });
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].value, "person");
    assert_eq!(bindings[0].name, "name");
    assert_eq!(
        document
            .summary_snapshot()
            .ok_or("summary")?
            .interpolation_bindings()
            .len(),
        1
    );
    Ok(())
}
