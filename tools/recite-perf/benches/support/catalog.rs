//! Incoming PO rows must not copy all prior translations.
use recite_adapter::ReciteDialogueCatalog;
use recite_core::LocaleId;
use recite_runtime::localisation::{LocaleProvider, TextDomain};

pub(super) struct Import {
    pub catalog: ReciteDialogueCatalog,
    pub po: String,
    existing: usize,
}

pub(super) fn prepare(existing: usize) -> Import {
    let mut catalog = ReciteDialogueCatalog::new();
    for i in 0..existing {
        catalog
            .insert("fr", &format!("{i:020x}"), "Hello.", "Bonjour.")
            .unwrap_or_else(|error| panic!("valid catalog: {error}"));
    }
    let po = (existing..existing + 100)
        .map(|i| {
            format!("msgctxt \"{i:020x}\"\nmsgid \"New line.\"\nmsgstr \"Nouvelle ligne.\"\n\n")
        })
        .collect();
    Import {
        catalog,
        po,
        existing,
    }
}

pub(super) fn validate(workload: Import) {
    let locale = LocaleId::new("fr").unwrap_or_else(|error| panic!("locale: {error}"));
    for i in 0..workload.existing + 100 {
        let (source, translation) = if i < workload.existing {
            ("Hello.", "Bonjour.")
        } else {
            ("New line.", "Nouvelle ligne.")
        };
        let actual = workload
            .catalog
            .lookup(
                &format!("{i:020x}"),
                source,
                TextDomain::Line,
                &locale,
                None,
            )
            .unwrap_or_else(|error| panic!("valid lookup: {error}"));
        assert_eq!(actual.as_deref(), Some(translation));
    }
}
