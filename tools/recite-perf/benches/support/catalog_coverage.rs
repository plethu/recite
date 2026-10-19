use recite_compiler::authoring::{CatalogCoverageSummary, CatalogInput, CatalogResolutionPolicy};
use recite_core::{
    LocaleId,
    po::{PoDocument, PotDocument, PotEntry},
};

pub(crate) struct Input {
    pub(crate) expected: PotDocument,
    pub(crate) catalog: CatalogInput,
    pub(crate) policy: CatalogResolutionPolicy,
}
pub(crate) fn prepare(count: usize) -> Input {
    let mut po = String::from("msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n");
    let mut expected = PotDocument {
        entries: Vec::new(),
    };
    for index in 0..count {
        let context = format!("{index:020x}");
        let source_text = format!("Choice {index}.");
        po.push_str(&format!(
            "msgctxt \"{context}\"\nmsgid \"{source_text}\"\nmsgstr \"Traduit {index}.\"\n\n"
        ));
        expected.entries.push(PotEntry {
            context,
            source_text,
            plural_source_text: None,
            comments: Vec::new(),
            reference: None,
        });
    }
    let locale = LocaleId::new("fr").unwrap_or_else(|e| panic!("fixture locale: {e}"));
    Input {
        expected,
        catalog: CatalogInput::from_document(
            locale.clone(),
            PoDocument::parse(po).unwrap_or_else(|e| panic!("fixture PO: {e}")),
        )
        .unwrap_or_else(|e| panic!("fixture catalogue: {e}")),
        policy: CatalogResolutionPolicy::new(Some(locale)),
    }
}
pub(crate) fn summarize(input: &Input) -> CatalogCoverageSummary {
    CatalogCoverageSummary::build(
        &input.expected,
        [input.catalog.clone()],
        input.policy.clone(),
    )
    .unwrap_or_else(|e| panic!("fixture coverage: {e}"))
}
pub(crate) fn validate((input, summary): (Input, CatalogCoverageSummary)) {
    let count = input.expected.entries.len();
    assert_eq!(summary.expected_count(), count);
    assert_eq!(summary.catalogs()[0].coverage().translated_count(), count);
    assert!(
        summary
            .entries()
            .iter()
            .all(|entry| entry.matched().is_some())
    );
    assert_eq!(
        summary.catalogs()[0].fingerprint(),
        &input.catalog.document().fingerprint()
    );
}
