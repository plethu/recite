use recite_core::ProjectSchema;
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

pub(crate) fn hover_detail(
    origin: Option<&recite_core::ProducerOrigin>,
    schema: &ProjectSchema,
    scoped_fingerprints: &[recite_core::ProducerFingerprint],
    catalog: &UiCatalog,
) -> String {
    let origin = origin.map_or_else(String::new, |origin| origin_detail(catalog, origin));
    let mut detail = origin;
    let metadata = schema.producer_metadata.as_ref();
    if let Some(producer) = metadata.and_then(|metadata| metadata.producer.as_ref()) {
        detail.push_str(&catalog.format_args(
            MsgId::LspHoverSchemaProducer,
            &UiArgs::from([
                ("kind".to_owned(), UiArg::from(producer.kind.to_string())),
                ("id".to_owned(), UiArg::from(producer.id.to_string())),
            ]),
        ));
    }
    let content_fingerprint = metadata.and_then(|metadata| metadata.content_fingerprint.as_ref());
    let producer_fingerprints = metadata.map_or(0, |metadata| metadata.producer_fingerprints.len());
    let scope = if scoped_fingerprints.is_empty() {
        String::new()
    } else {
        catalog.format_args(
            MsgId::LspHoverSchemaScopedFingerprints,
            &UiArgs::from([(
                "fingerprints".to_owned(),
                UiArg::from(format_scoped_fingerprints(scoped_fingerprints)),
            )]),
        )
    };
    if content_fingerprint.is_some()
        || producer_fingerprints != 0
        || !scoped_fingerprints.is_empty()
    {
        detail.push_str(&catalog.format_args(
            MsgId::LspHoverSchemaFreshness,
            &UiArgs::from([
                (
                    "fingerprint".to_owned(),
                    UiArg::from(
                        content_fingerprint.map_or_else(|| "none".to_owned(), format_fingerprint),
                    ),
                ),
                ("inputs".to_owned(), UiArg::from(producer_fingerprints)),
                ("scope".to_owned(), UiArg::from(scope)),
            ]),
        ));
    }
    detail
}

fn format_fingerprint(fingerprint: &recite_core::ContentFingerprint) -> String {
    format!(
        "{}:{}",
        fingerprint.algorithm().as_str(),
        fingerprint
            .digest()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn format_scoped_fingerprints(fingerprints: &[recite_core::ProducerFingerprint]) -> String {
    let mut values = fingerprints
        .iter()
        .map(|fingerprint| {
            format!(
                "{}:{}:{}={}",
                fingerprint.kind, fingerprint.id, fingerprint.algorithm, fingerprint.value
            )
        })
        .collect::<Vec<_>>();
    values.sort();
    values.join(", ")
}

pub(super) fn origin_detail(catalog: &UiCatalog, origin: &recite_core::ProducerOrigin) -> String {
    catalog.format_args(
        MsgId::LspHoverProducedBy,
        &UiArgs::from([
            ("kind".to_owned(), UiArg::from(origin.kind.to_string())),
            ("id".to_owned(), UiArg::from(origin.id.to_string())),
            (
                "label".to_owned(),
                UiArg::from(
                    origin
                        .label
                        .as_ref()
                        .map_or_else(String::new, |label| format!(" ({label})")),
                ),
            ),
        ]),
    )
}
