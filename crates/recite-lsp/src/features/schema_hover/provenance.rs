use recite_compiler::SchemaSummary;
use recite_core::{ContentFingerprintFreshness, ProducerFreshness};
use recite_ui::{MsgId, UiArg, UiArgs, UiCatalog};

pub(crate) fn hover_detail(
    origin: Option<&recite_core::ProducerOrigin>,
    schema: &SchemaSummary,
    scoped_fingerprints: &[recite_core::ProducerFingerprint],
    catalog: &UiCatalog,
) -> String {
    let origin = origin.map_or_else(String::new, |origin| origin_detail(catalog, origin));
    let mut detail = origin;
    let metadata = schema.producer_metadata();
    if let Some(producer) = metadata.and_then(|metadata| metadata.producer()) {
        detail.push_str(&catalog.format_args(
            MsgId::LspHoverSchemaProducer,
            &UiArgs::from([
                ("kind".to_owned(), UiArg::from(producer.kind().to_string())),
                ("id".to_owned(), UiArg::from(producer.id().to_string())),
            ]),
        ));
    }
    let compared = matches!(
        schema.freshness(),
        recite_compiler::SchemaFreshness::Compared(_)
    );
    let content_fingerprint = metadata.and_then(|metadata| metadata.content_fingerprint());
    let producer_fingerprints =
        metadata.map_or(0, |metadata| metadata.producer_fingerprints().len());
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
        || compared
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
    detail.push_str(&freshness_state_detail(schema, catalog));
    detail
}

fn freshness_state_detail(schema: &SchemaSummary, catalog: &UiCatalog) -> String {
    match schema.freshness() {
        recite_compiler::SchemaFreshness::Compared(comparison) => {
            let comparison = comparison.as_ref();
            catalog.format_args(
                MsgId::LspHoverSchemaFreshnessState,
                &UiArgs::from([
                    (
                        "state".to_owned(),
                        UiArg::from(if comparison.is_fresh() {
                            "fresh"
                        } else {
                            "stale"
                        }),
                    ),
                    (
                        "content".to_owned(),
                        UiArg::from(content_status(&comparison.content_fingerprint)),
                    ),
                    (
                        "manifest".to_owned(),
                        UiArg::from(producer_status(&comparison.manifest)),
                    ),
                    (
                        "registries".to_owned(),
                        UiArg::from(scope_status(&comparison.registries)),
                    ),
                    (
                        "metadata_domains".to_owned(),
                        UiArg::from(scope_status(&comparison.metadata_domains)),
                    ),
                ]),
            )
        }
        recite_compiler::SchemaFreshness::Unavailable { reason } => catalog.format_args(
            MsgId::LspHoverSchemaFreshnessUnavailable,
            &UiArgs::from([("reason".to_owned(), UiArg::from(freshness_reason(*reason)))]),
        ),
        _ => catalog.format_args(
            MsgId::LspHoverSchemaFreshnessUnavailable,
            &UiArgs::from([(
                "reason".to_owned(),
                UiArg::from("freshness state is not supported by this client"),
            )]),
        ),
    }
}

fn content_status(freshness: &ContentFingerprintFreshness) -> &'static str {
    if matches!(freshness, ContentFingerprintFreshness::Fresh) {
        "fresh"
    } else {
        "stale"
    }
}

fn producer_status(freshness: &ProducerFreshness) -> &'static str {
    if matches!(freshness, ProducerFreshness::Fresh) {
        "fresh"
    } else {
        "stale"
    }
}

fn scope_status(scopes: &std::collections::BTreeMap<String, ProducerFreshness>) -> String {
    if scopes.is_empty() {
        return "none".to_owned();
    }
    scopes
        .iter()
        .map(|(name, freshness)| format!("{name}:{}", producer_status(freshness)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn freshness_reason(reason: recite_compiler::SchemaFreshnessUnavailableReason) -> &'static str {
    match reason {
        recite_compiler::SchemaFreshnessUnavailableReason::NoComparisonSnapshot => {
            "no comparison snapshot"
        }
        recite_compiler::SchemaFreshnessUnavailableReason::NoProducerMetadata => {
            "no producer metadata"
        }
        _ => "freshness reason is not supported by this client",
    }
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

pub(crate) fn origin_detail(catalog: &UiCatalog, origin: &recite_core::ProducerOrigin) -> String {
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
