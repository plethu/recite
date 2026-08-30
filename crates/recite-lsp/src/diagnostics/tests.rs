use lsp_types::{Position, Uri};
use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentation,
    DiagnosticPresentationId, DiagnosticRelatedPresentation, SourcePosition, SourceSpan,
    auxiliary_contract_for, contract_for,
};
use recite_ui::{DEFAULT_RESOURCE, UiCatalog, UiLocale};
use tempfile::TempDir;

use super::{DiagnosticSource, publish_diagnostics};
use crate::paths::file_path_to_uri;
use crate::workspace::{LspWorkspace, WorkspaceConfig};

#[test]
fn malformed_optional_related_presentation_is_omitted_without_catalog_details() {
    let diagnostic = diagnostic_with_id(
        "compatibility message",
        SourceSpan::point(
            "dialogue/start.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
        "duplicate",
    )
    .with_related_presentations([DiagnosticRelatedPresentation::new(
        SourceSpan::point(
            "dialogue/start.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
        DiagnosticPresentation::new(DiagnosticPresentationId::new_static(
            "diagnostic-test-unknown",
        )),
    )]);
    let catalog = UiCatalog::load(&UiLocale::default()).expect("default catalog");

    let published = publish_diagnostics(
        "file:///workspace/dialogue/start.recite"
            .parse()
            .expect("valid test URI"),
        "source\n",
        Some(1),
        &[diagnostic],
        &catalog,
        &[],
    )
    .expect("recordable diagnostic");
    assert!(published.diagnostics[0].related_information.is_none());
}

#[test]
fn unresolved_related_sources_are_omitted_without_rebinding_to_primary() {
    let structured = diagnostic_with_id(
        "compatibility message",
        SourceSpan::point(
            "dialogue/primary.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
        "duplicate",
    )
    .with_related_presentations([DiagnosticRelatedPresentation::new(
        SourceSpan::point(
            "outside-project.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
        DiagnosticPresentation::new(DiagnosticPresentationId::new_static(
            "diagnostic-id-003-related",
        )),
    )]);
    let catalog = UiCatalog::load(&UiLocale::default()).expect("default catalog");
    let primary_uri = "file:///workspace/dialogue/primary.recite"
        .parse::<Uri>()
        .expect("valid test URI");

    let published = publish_diagnostics(
        primary_uri,
        "primary\n",
        Some(1),
        &[structured],
        &catalog,
        &[],
    )
    .expect("recordable diagnostic");
    assert!(published.diagnostics[0].related_information.is_none());
}
#[test]
fn unrecordable_diagnostics_are_reported_instead_of_dropped() {
    let legacy = recite_core::Diagnostic::error(
        DiagnosticCode::new_static("RECITE_ID003"),
        "compatibility message",
        SourceSpan::point(
            "dialogue/primary.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
    )
    .with_related([recite_core::RelatedSpan::new(
        SourceSpan::point(
            "outside-project.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
        "related compatibility message",
    )]);
    let catalog = UiCatalog::load(&UiLocale::default()).expect("default catalog");
    let primary_uri = "file:///workspace/dialogue/primary.recite"
        .parse::<Uri>()
        .expect("valid test URI");
    let error = publish_diagnostics(primary_uri, "primary\n", Some(1), &[legacy], &catalog, &[])
        .expect_err("legacy-only diagnostics must not be silently dropped");
    assert!(error.to_string().contains("RECITE_ID003"));
}

#[test]
fn related_spans_resolve_open_project_file_text() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first_path = temp.path().join("dialogue/first.recite");
    std::fs::create_dir_all(first_path.parent().expect("dialogue parent"))
        .expect("create dialogue directory");
    std::fs::write(
        &first_path,
        ":: first\n> shared@83709c28414d0ce4659c\n  First.\n",
    )
    .expect("write first source");
    let first_uri = file_path_to_uri(&first_path).expect("valid first URI");
    let second_uri =
        file_path_to_uri(&temp.path().join("dialogue/second.recite")).expect("valid second URI");
    let mut workspace = match UiCatalog::load(&UiLocale::default()) {
        Ok(catalog) => LspWorkspace::with_ui_catalog(
            WorkspaceConfig::for_roots(vec![temp.path().to_owned()]),
            catalog,
        )
        .unwrap_or_else(|error| panic!("test authoring state is invalid: {error}")),
        Err(error) => panic!("test default UI catalog is invalid: {error}"),
    };
    workspace.open(
        first_uri.clone(),
        1,
        ":: first\n> shared@83709c28414d0ce4659c\n  😀First.\n".to_owned(),
    );
    let diagnostic = diagnostic_with_id(
        "compatibility message",
        SourceSpan::point(
            "dialogue/second.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
        "shared",
    )
    .with_related_presentations([DiagnosticRelatedPresentation::new(
        SourceSpan::new(
            "dialogue/first.recite",
            SourcePosition::new(3, 3).expect("valid test position"),
            Some(SourcePosition::new(3, 8).expect("valid test position")),
        ),
        DiagnosticPresentation::new(DiagnosticPresentationId::new_static(
            "diagnostic-id-003-related",
        )),
    )]);
    let sources = workspace.diagnostic_sources();
    let published = publish_diagnostics(
        second_uri,
        ":: second\n",
        Some(1),
        &[diagnostic],
        &workspace.ui_catalog,
        &sources,
    )
    .expect("recordable diagnostic");
    let related = published.diagnostics[0]
        .related_information
        .as_ref()
        .expect("open target has related source");
    assert_eq!(related[0].location.uri, first_uri);
    assert_eq!(related[0].location.range.start, Position::new(2, 2));
    assert_eq!(related[0].location.range.end, Position::new(2, 9));
}
#[test]
fn localized_primary_related_and_help_use_shared_renderer_at_lsp_boundary() {
    let related_uri = "file:///workspace/dialogue/related.recite"
        .parse::<Uri>()
        .expect("valid related URI");
    let sources = [DiagnosticSource {
        path: "dialogue/related.recite".to_owned(),
        uri: &related_uri,
        text: "😀first\nsecond\n",
    }];
    let help = auxiliary_contract_for(&DiagnosticPresentationId::new_static(
        "diagnostic-id-003-help",
    ))
    .expect("duplicate ID help contract")
    .presentation(std::iter::empty::<(&str, DiagnosticArgumentValue)>())
    .expect("duplicate ID help has no arguments");
    let diagnostic = diagnostic_with_id(
        "compatibility primary",
        SourceSpan::point(
            "dialogue/primary.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
        "duplicate",
    )
    .with_related_presentations([
        DiagnosticRelatedPresentation::new(
            SourceSpan::new(
                "dialogue/related.recite",
                SourcePosition::new(1, 1).expect("valid test position"),
                Some(SourcePosition::new(1, 6).expect("valid test position")),
            ),
            DiagnosticPresentation::new(DiagnosticPresentationId::new_static(
                "diagnostic-id-003-related",
            )),
        ),
        DiagnosticRelatedPresentation::new(
            SourceSpan::point(
                "dialogue/related.recite",
                SourcePosition::new(2, 1).expect("valid test position"),
            ),
            DiagnosticPresentation::new(DiagnosticPresentationId::new_static(
                "diagnostic-id-003-related",
            )),
        ),
    ])
    .with_help_presentation(help);
    let resource = DEFAULT_RESOURCE
        .replace(
            "diagnostic-id-003 = duplicate localisable id `{$id}` on line",
            "diagnostic-id-003 = localized primary {$id}",
        )
        .replace(
            "diagnostic-id-003-related = first localisable ID is here",
            "diagnostic-id-003-related = localized related",
        )
        .replace(
            "diagnostic-id-003-help = rename one of the duplicate localisable IDs",
            "diagnostic-id-003-help = localized help",
        );
    let catalog = catalog_with_resource("fr", resource);

    let published = publish_diagnostics(
        "file:///workspace/dialogue/primary.recite"
            .parse()
            .expect("valid primary URI"),
        "primary\n",
        Some(1),
        &[diagnostic],
        &catalog,
        &sources,
    )
    .expect("recordable diagnostic");
    let published = &published.diagnostics[0];
    assert_eq!(published.message, "localized primary duplicate");
    assert!(!published.message.contains("localized help"));
    let related = published
        .related_information
        .as_ref()
        .expect("resolved related diagnostics");
    assert_eq!(related.len(), 2);
    assert_eq!(related[0].message, "localized related");
    assert_eq!(related[1].message, "localized related");
    assert_eq!(related[0].location.uri, related_uri);
    assert_eq!(related[0].location.range.start, Position::new(0, 0));
    assert_eq!(related[0].location.range.end, Position::new(0, 7));
    assert_eq!(related[1].location.range.start, Position::new(1, 0));
}
#[test]
fn primary_renderer_fallback_preserves_compatibility_message() {
    let diagnostic = Diagnostic::error(
        DiagnosticCode::new_static("RECITE_ID003"),
        "compatibility fallback",
        SourceSpan::point(
            "dialogue/start.recite",
            SourcePosition::new(1, 1).expect("valid test position"),
        ),
    )
    .with_presentation(DiagnosticPresentation::new(
        DiagnosticPresentationId::new_static("diagnostic-unknown-primary"),
    ));
    let catalog = UiCatalog::load(&UiLocale::default()).expect("default catalog");
    let published = publish_diagnostics(
        "file:///workspace/dialogue/start.recite"
            .parse()
            .expect("valid test URI"),
        "start\n",
        None,
        &[diagnostic],
        &catalog,
        &[],
    )
    .expect("compatibility fallback is recordable");
    assert_eq!(published.diagnostics[0].message, "compatibility fallback");
}

#[test]
fn ordering_uses_record_data_when_localized_text_reverses_lexical_order() {
    let resource = DEFAULT_RESOURCE.replace(
        "diagnostic-id-003 = duplicate localisable id `{$id}` on line",
        "diagnostic-id-003 = { $id ->\n    [first] zzz\n   *[second] aaa\n}",
    );
    let catalog = catalog_with_resource("fr", resource);
    let span = SourceSpan::point(
        "dialogue/start.recite",
        SourcePosition::new(1, 1).expect("valid test position"),
    );
    let first = diagnostic_with_id("first compatibility", span.clone(), "first");
    let second = diagnostic_with_id("second compatibility", span, "second");
    let published = publish_diagnostics(
        "file:///workspace/dialogue/start.recite"
            .parse()
            .expect("valid test URI"),
        "start\n",
        None,
        &[second, first],
        &catalog,
        &[],
    )
    .expect("recordable diagnostics");
    assert_eq!(
        published
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        ["zzz", "aaa"]
    );
}

fn catalog_with_resource(locale: &str, resource: String) -> UiCatalog {
    UiCatalog::from_resources(
        locale.parse().expect("test locale is valid"),
        [
            (
                "en-US".parse().expect("English locale is valid"),
                DEFAULT_RESOURCE.to_owned(),
            ),
            (locale.parse().expect("test locale is valid"), resource),
        ],
    )
    .expect("test catalog is complete")
}

fn diagnostic_with_id(message: &str, span: SourceSpan, id: &str) -> Diagnostic {
    let code = DiagnosticCode::new_static("RECITE_ID003");
    let contract = contract_for(
        &code,
        &DiagnosticPresentationId::new_static("diagnostic-id-003"),
    )
    .expect("duplicate ID contract");
    Diagnostic::error_from_contract(
        contract,
        message,
        span,
        [("id", DiagnosticArgumentValue::String(id.to_owned()))],
    )
    .expect("duplicate ID arguments match contract")
}
