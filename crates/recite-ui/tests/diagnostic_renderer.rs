use recite_core::{
    DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentation, DiagnosticPresentationId,
    DiagnosticRecord, DiagnosticRelatedPresentation, DiagnosticSeverity, SourcePosition,
    SourceSpan,
};
use recite_ui::{CatalogError, DEFAULT_RESOURCE, RenderedRelatedDiagnostic, UiCatalog, UiLocale};
use unic_langid::LanguageIdentifier;

fn locale(value: &str) -> LanguageIdentifier {
    value
        .parse()
        .unwrap_or_else(|error| panic!("test locale is valid: {error}"))
}

fn resource_with_primary(text: &str) -> String {
    DEFAULT_RESOURCE.replace(
        "diagnostic-parse-001 = expected a Recite statement header or indented prose",
        &format!("diagnostic-parse-001 = {text}"),
    )
}

fn presentation(id: &str) -> DiagnosticPresentation {
    DiagnosticPresentation::new(
        DiagnosticPresentationId::new(id)
            .unwrap_or_else(|error| panic!("test presentation ID is valid: {error}")),
    )
}

fn point(file: &str, line: u32, column: u32) -> SourceSpan {
    SourceSpan::point(
        file,
        SourcePosition::new(line, column)
            .unwrap_or_else(|error| panic!("test source position is valid: {error}")),
    )
}

fn record(primary: DiagnosticPresentation) -> DiagnosticRecord {
    DiagnosticRecord::new(
        DiagnosticCode::new_static("RECITE_PARSE001"),
        DiagnosticSeverity::Error,
        point("dialogue/intro.recite", 1, 1),
        primary,
    )
}

fn catalog(
    requested: &str,
    locales: impl IntoIterator<Item = (&'static str, &'static str)>,
) -> UiCatalog {
    let resources = std::iter::once((locale("en-US"), DEFAULT_RESOURCE.to_owned())).chain(
        locales
            .into_iter()
            .map(|(name, text)| (locale(name), resource_with_primary(text))),
    );
    UiCatalog::from_resources(locale(requested), resources)
        .unwrap_or_else(|error| panic!("complete test catalogs: {error}"))
}

#[test]
fn locale_resolution_uses_requested_then_language_then_english() {
    let requested = catalog("fr-CA", [("fr-CA", "requested"), ("fr", "language")]);
    assert_eq!(
        requested
            .render_diagnostic(&record(presentation("diagnostic-parse-001")))
            .expect("requested locale")
            .primary_text,
        "requested"
    );

    let language = catalog("fr-CA", [("fr", "language")]);
    assert_eq!(
        language
            .render_diagnostic(&record(presentation("diagnostic-parse-001")))
            .expect("language locale")
            .primary_text,
        "language"
    );

    let english = catalog("fr-CA", []);
    assert_eq!(
        english
            .render_diagnostic(&record(presentation("diagnostic-parse-001")))
            .expect("English fallback")
            .primary_text,
        "expected a Recite statement header or indented prose"
    );
}

#[test]
fn localized_primary_wins_over_compatibility_message() {
    let record = record(presentation("diagnostic-parse-001"))
        .with_compatibility_message("deliberately different compatibility text");
    let rendered = catalog("fr-FR", [("fr-FR", "localized primary")])
        .render_diagnostic(&record)
        .expect("localized diagnostic");
    assert_eq!(rendered.primary_text, "localized primary");
}

#[test]
fn missing_or_invalid_primary_uses_compatibility_message() {
    let missing = record(presentation("diagnostic-missing-resource"))
        .with_compatibility_message("missing compatibility text");
    assert_eq!(
        UiCatalog::load(&recite_ui::UiLocale::default())
            .expect("default catalog")
            .render_diagnostic(&missing)
            .expect("missing resource fallback")
            .primary_text,
        "missing compatibility text"
    );

    let invalid = record(
        presentation("diagnostic-parse-001")
            .with_argument(
                "unexpected",
                DiagnosticArgumentValue::String("value".to_owned()),
            )
            .unwrap_or_else(|error| panic!("argument name is valid: {error}")),
    )
    .with_compatibility_message("invalid compatibility text");
    assert_eq!(
        UiCatalog::load(&recite_ui::UiLocale::default())
            .expect("default catalog")
            .render_diagnostic(&invalid)
            .expect("invalid resource fallback")
            .primary_text,
        "invalid compatibility text"
    );
}

#[test]
fn no_presentable_primary_or_compatibility_is_an_error() {
    let result = UiCatalog::load(&recite_ui::UiLocale::default())
        .expect("default catalog")
        .render_diagnostic(&record(presentation("diagnostic-missing-resource")));
    assert!(matches!(
        result,
        Err(CatalogError::Resolution { id, .. }) if id == "diagnostic-missing-resource"
    ));
}

#[test]
fn related_and_help_fail_closed_while_preserving_order_and_spans() {
    let first_span = point("dialogue/first.recite", 2, 3);
    let second_span = point("dialogue/second.recite", 8, 13);
    let rendered = record(presentation("diagnostic-parse-001"))
        .with_related([
            DiagnosticRelatedPresentation::new(
                first_span.clone(),
                presentation("diagnostic-id-003-related"),
            ),
            DiagnosticRelatedPresentation::new(
                point("dialogue/invalid.recite", 4, 2),
                presentation("diagnostic-missing-related"),
            ),
            DiagnosticRelatedPresentation::new(
                point("dialogue/invalid-arguments.recite", 6, 5),
                presentation("diagnostic-id-003"),
            ),
            DiagnosticRelatedPresentation::new(
                second_span.clone(),
                presentation("diagnostic-id-004-related"),
            ),
        ])
        .with_help(Some(presentation("diagnostic-missing-help")));
    let rendered = UiCatalog::load(&UiLocale::default())
        .expect("default catalog")
        .render_diagnostic(&rendered)
        .expect("rendered diagnostic");

    assert_eq!(
        rendered.related,
        vec![
            RenderedRelatedDiagnostic {
                span: first_span,
                text: "first localisable ID is here".to_owned(),
            },
            RenderedRelatedDiagnostic {
                span: second_span,
                text: "first localisable ID is here".to_owned(),
            },
        ]
    );
    assert_eq!(rendered.help, None);
}

#[test]
fn rendering_same_record_against_different_catalogs_does_not_cache_strings() {
    let record = record(presentation("diagnostic-parse-001"));
    let first = catalog("fr-FR", [("fr-FR", "first")])
        .render_diagnostic(&record)
        .expect("first catalog");
    let second = catalog("fr-FR", [("fr-FR", "second")])
        .render_diagnostic(&record)
        .expect("second catalog");
    assert_eq!(first.primary_text, "first");
    assert_eq!(second.primary_text, "second");
}
