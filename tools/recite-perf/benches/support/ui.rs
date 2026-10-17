//! Labels share a catalog and prepared arguments; returned text stays live through counters.
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog, UiLocale};

pub(super) struct Input {
    catalog: UiCatalog,
    arguments: Option<UiArgs>,
    expected: String,
}

pub(super) fn static_labels() -> Input {
    Input {
        catalog: UiCatalog::load(&UiLocale::default())
            .unwrap_or_else(|error| panic!("catalog: {error}")),
        arguments: None,
        expected: "Insert missing stable ID".into(),
    }
}

pub(super) fn parameterized_labels() -> Input {
    Input {
        catalog: UiCatalog::load(&UiLocale::default())
            .unwrap_or_else(|error| panic!("catalog: {error}")),
        arguments: Some(UiArgs::from([
            ("action".into(), UiArg::from("invoke")),
            ("producer".into(), UiArg::from("godot/écrivain")),
            ("producer_state".into(), UiArg::from("present")),
            ("declaration_kind".into(), UiArg::from("condition")),
            ("declaration_name".into(), UiArg::from("gate_000001")),
        ])),
        expected: "Schema capability (condition gate_000001): invoke producer (godot/écrivain)"
            .into(),
    }
}

pub(super) fn format(input: Input) -> (Input, Vec<String>) {
    let result = (0..1_000)
        .map(|_| match &input.arguments {
            Some(args) => input
                .catalog
                .format_args(MsgId::LspCodeActionSchemaAction, args),
            None => input.catalog.text(MsgId::LspCodeActionInsertMissingId),
        })
        .collect();
    (input, result)
}

pub(super) fn validate((input, labels): (Input, Vec<String>)) {
    assert_eq!(labels.len(), 1_000);
    assert!(labels.iter().all(|label| label == &input.expected));
}
