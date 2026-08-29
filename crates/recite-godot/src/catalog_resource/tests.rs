use std::panic::{AssertUnwindSafe, catch_unwind};

use godot::builtin::{GString, VarArray, VarDictionary};
use godot::obj::Gd;
use godot::prelude::ToGodot;
use recite_runtime::LocaleProvider;

use super::ReciteDialogueCatalogResource;
use crate::catalog::ReciteDialogueCatalog;

fn resource_with(
    entries: VarArray,
    plural_forms: VarDictionary,
) -> Gd<ReciteDialogueCatalogResource> {
    Gd::from_init_fn(|base| ReciteDialogueCatalogResource {
        base,
        catalog: ReciteDialogueCatalog::new(),
        serialized_entries: entries,
        serialized_plural_forms: plural_forms,
    })
}

fn singular_record(translation: &str) -> VarDictionary {
    let mut record = VarDictionary::new();
    record.set("kind", "singular");
    record.set("locale", "fr");
    record.set("id", "line");
    record.set("source_text", "Source.");
    record.set("translation", translation);
    record.set("domain", 0_i64);
    record.set("variant", "");
    record
}

fn plural_record(translations: VarArray) -> VarDictionary {
    let mut record = VarDictionary::new();
    record.set("kind", "plural");
    record.set("locale", "fr");
    record.set("id", "letters");
    record.set("source_singular", "One letter.");
    record.set("source_plural", "{count} letters.");
    record.set("translations", &translations.to_variant());
    record.set("variant", "");
    record
}

fn plural_forms() -> VarDictionary {
    let mut forms = VarDictionary::new();
    forms.set("fr", "nplurals=2; plural=(n != 1);");
    forms
}

#[test]
#[ignore = "requires an initialized Godot host"]
fn malformed_persisted_variants_are_rejected_without_panicking() {
    let cases = [
        {
            let mut entries = VarArray::new();
            entries.push(42_i64);
            (entries, VarDictionary::new())
        },
        {
            let mut record = singular_record("Bonjour.");
            record.set("domain", "line");
            let mut entries = VarArray::new();
            entries.push(&record.to_variant());
            (entries, VarDictionary::new())
        },
        {
            let mut record = singular_record("Bonjour.");
            record.set(1_i64, "unexpected");
            let mut entries = VarArray::new();
            entries.push(&record.to_variant());
            (entries, VarDictionary::new())
        },
        {
            let mut arms = VarArray::new();
            arms.push(1_i64);
            let mut entries = VarArray::new();
            entries.push(&plural_record(arms).to_variant());
            (entries, plural_forms())
        },
        {
            let mut forms = VarDictionary::new();
            forms.set(1_i64, "nplurals=2; plural=(n != 1);");
            (VarArray::new(), forms)
        },
    ];

    for (entries, forms) in cases {
        let resource = resource_with(entries, forms);
        let result = catch_unwind(AssertUnwindSafe(|| resource.bind().cloned_catalog()));
        assert!(result.is_ok(), "malformed persisted data must not panic");
        assert!(result.expect("panic was checked").is_err());
    }
}

#[test]
#[ignore = "requires an initialized Godot host"]
fn persisted_plural_shape_is_validated_during_reload() {
    let mut arms = VarArray::new();
    arms.push("Une lettre.");
    let mut entries = VarArray::new();
    entries.push(&plural_record(arms).to_variant());
    let resource = resource_with(entries, plural_forms());

    let result = resource.bind().cloned_catalog();
    assert!(
        result.is_err(),
        "wrong nplurals arm count must reject on reload"
    );
}

#[test]
#[ignore = "requires an initialized Godot host"]
fn persisted_fields_are_reloaded_before_mutation() {
    let mut resource = resource_with(VarArray::new(), VarDictionary::new());
    {
        let mut bound = resource.bind_mut();
        let _ = bound.add_translation(
            GString::from("fr"),
            GString::from("line"),
            GString::from("Source."),
            GString::from("Old."),
            GString::new(),
        );
    }

    {
        let mut bound = resource.bind_mut();
        let mut record: VarDictionary = bound
            .serialized_entries
            .at(0)
            .try_to()
            .expect("record dictionary");
        record.set("translation", "New.");
        bound.serialized_entries.set(0, &record.to_variant());
    }

    {
        let mut bound = resource.bind_mut();
        let _ = bound.add_translation(
            GString::from("fr"),
            GString::from("line"),
            GString::from("Source."),
            GString::from("New."),
            GString::new(),
        );
    }

    let catalog = resource.bind().cloned_catalog().expect("reloaded catalog");
    let locale = recite_core::LocaleId::new("fr").expect("locale");
    assert_eq!(
        catalog
            .lookup(
                "line",
                "Source.",
                recite_runtime::TextDomain::Line,
                &locale,
                None,
            )
            .expect("lookup"),
        Some("New.".to_owned())
    );
}
