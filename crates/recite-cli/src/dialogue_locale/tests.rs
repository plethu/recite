use super::{DialogueCatalogProvider, DialogueCatalogSource};
use crate::error::CliError;
use crate::i18n::{Messages, UiLocale};
use recite_core::{DiagnosticArgumentValue, LocaleId};
use recite_runtime::{LocaleProvider, TextDomain};
use tempfile::TempDir;

#[path = "../tests/dialogue_locale_catalog.rs"]
mod catalog_resolution;

fn locale(value: &str) -> unic_langid::LanguageIdentifier {
    value.parse().expect("test locale")
}

fn alternate_resource(id: &str, replacement: &str) -> String {
    let mut resource = String::new();
    let mut replaced = false;
    let mut skip_original_selector = false;
    for line in recite_ui::DEFAULT_RESOURCE.lines() {
        if skip_original_selector {
            if line == "}" {
                skip_original_selector = false;
            }
            continue;
        }
        let Some((line_id, value)) = line.split_once(" = ") else {
            resource.push_str(line);
            resource.push('\n');
            continue;
        };
        if line_id == id {
            resource.push_str(line_id);
            resource.push_str(" = ");
            resource.push_str(replacement);
            resource.push('\n');
            replaced = true;
            if value.contains("->") {
                skip_original_selector = true;
            }
        } else {
            resource.push_str(line);
            resource.push('\n');
        }
    }
    assert!(replaced, "diagnostic resource {id} must exist");
    resource
}

fn assert_markup_presentation(
    source_text: &str,
    translation: &str,
    expected_id: &str,
    expected_args: &[(&str, DiagnosticArgumentValue)],
    replacement: &str,
    expected: &str,
) {
    let source = format!(
        "msgctxt \"11111111111111111111\"\nmsgid \"{source_text}\"\nmsgstr \"{translation}\"\n"
    );
    let error = match super::po::parse_po_catalog(std::path::Path::new("catalog.po"), &source) {
        Ok(_) => panic!("markup mismatch must reject the catalogue"),
        Err(error) => error,
    };
    let CliError::DialogueCatalogMalformed { reason, .. } = error else {
        panic!("expected malformed dialogue catalog error")
    };
    let super::DialogueCatalogMalformedReason::Markup {
        presentation,
        compatibility_message: _,
    } = reason
    else {
        panic!("expected structured markup presentation")
    };
    assert_alternate_markup_presentation(
        &presentation,
        expected_id,
        expected_args,
        replacement,
        expected,
    );
}

fn assert_alternate_markup_presentation(
    presentation: &recite_core::DiagnosticPresentation,
    expected_id: &str,
    expected_args: &[(&str, DiagnosticArgumentValue)],
    replacement: &str,
    expected: &str,
) {
    assert_eq!(presentation.id().as_str(), expected_id);
    assert_eq!(presentation.arguments().len(), expected_args.len());
    for (name, value) in expected_args {
        assert_eq!(presentation.arguments().get(*name), Some(value));
    }

    let default_messages = Messages::load(&UiLocale::default()).expect("default catalog");
    let default_text = default_messages.format_presentation(presentation);
    let resource = alternate_resource(expected_id, replacement);
    let alternate_messages =
        Messages::from_resources(locale("en-US"), [(locale("en-US"), resource)])
            .expect("alternate catalog");
    let localized = alternate_messages.format_presentation(presentation);
    assert_ne!(localized, default_text);
    assert_eq!(localized, expected);
}

#[test]
fn plural_lookup_uses_variant_then_locale_fallback_and_base_priority() {
    let temp = TempDir::new().expect("tempdir");
    let path = temp.path().join("fr.po");
    std::fs::write(
        &path,
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Language: fr\\n\"\n",
            "\"Plural-Forms: nplurals=3; plural=(n == 0 ? 0 : n == 1 ? 1 : 2);\\n\"\n",
            "\n",
            "msgctxt \"22222222222222222222&formal\"\n",
            "msgid \"one\"\n",
            "msgid_plural \"many {count}\"\n",
            "msgstr[0] \"variant one\"\n",
            "msgstr[1] \"variant one {count}\"\n",
            "msgstr[2] \"variant many {count}\"\n",
            "\n",
            "msgctxt \"22222222222222222222\"\n",
            "msgid \"one\"\n",
            "msgid_plural \"many {count}\"\n",
            "msgstr[0] \"base one\"\n",
            "msgstr[1] \"base one {count}\"\n",
            "msgstr[2] \"base many {count}\"\n",
        ),
    )
    .expect("write catalogue");
    let provider = DialogueCatalogProvider::load(vec![DialogueCatalogSource {
        locale: LocaleId::new("fr").expect("locale"),
        path,
    }])
    .expect("catalogue loads");
    let locale = LocaleId::new("fr-CA").expect("locale");
    let resolution = provider
        .resolve_plural(
            "22222222222222222222",
            "one",
            "many {count}",
            2,
            TextDomain::Line,
            &locale,
            Some("formal"),
        )
        .expect("variant lookup");
    assert_eq!(resolution.template, Some("variant many {count}".to_owned()));
    assert_eq!(resolution.selected_arm, Some(2));
    assert_eq!(resolution.matched_locale.as_deref(), Some("fr"));
    assert_eq!(
        resolution.matched_context.as_deref(),
        Some("22222222222222222222&formal")
    );
    assert_eq!(
        provider
            .resolve_plural(
                "22222222222222222222",
                "one",
                "many {count}",
                2,
                TextDomain::Line,
                &locale,
                Some("casual"),
            )
            .expect("base lookup")
            .template,
        Some("base many {count}".to_owned())
    );
}

#[test]
fn plural_resolution_uses_the_matching_catalogues_rule_only() {
    let temp = TempDir::new().expect("tempdir");
    let region = temp.path().join("fr-CA.po");
    std::fs::write(
        &region,
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n == 0 ? 1 : 0);\\n\"\n",
        ),
    )
    .expect("write region catalogue");
    let base = temp.path().join("fr.po");
    std::fs::write(
        &base,
        concat!(
            "msgid \"\"\n",
            "msgstr \"\"\n",
            "\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\n",
            "msgctxt \"22222222222222222222\"\n",
            "msgid \"one\"\n",
            "msgid_plural \"many\"\n",
            "msgstr[0] \"base singular\"\n",
            "msgstr[1] \"base plural\"\n",
        ),
    )
    .expect("write base catalogue");
    let provider = DialogueCatalogProvider::load(vec![
        DialogueCatalogSource {
            locale: LocaleId::new("fr-CA").expect("locale"),
            path: region,
        },
        DialogueCatalogSource {
            locale: LocaleId::new("fr").expect("locale"),
            path: base,
        },
    ])
    .expect("catalogues load");
    let resolution = provider
        .resolve_plural(
            "22222222222222222222",
            "one",
            "many",
            0,
            TextDomain::Line,
            &LocaleId::new("fr-CA").expect("locale"),
            None,
        )
        .expect("plural resolution");
    assert_eq!(resolution.template.as_deref(), Some("base singular"));
    assert_eq!(resolution.matched_locale.as_deref(), Some("fr"));
    assert_eq!(resolution.selected_arm, Some(0));
    assert_eq!(resolution.attempts.len(), 2);
    assert_eq!(resolution.attempts[0].locale, "fr-CA");
    assert_eq!(resolution.attempts[0].selected_arm, Some(1));
    assert!(matches!(
        resolution.attempts[0].outcome,
        recite_runtime::PluralResolutionOutcome::MissingEntry
    ));
    assert!(matches!(
        resolution.attempts[1].outcome,
        recite_runtime::PluralResolutionOutcome::Matched
    ));
}

#[test]
fn conflicting_plural_forms_headers_for_one_locale_are_rejected() {
    catalog_resolution::conflicting_plural_forms_headers_for_one_locale_are_rejected();
}

#[test]
fn lookup_exhausts_variant_locale_fallback_before_base_context() {
    catalog_resolution::lookup_exhausts_variant_locale_fallback_before_base_context();
}

#[test]
fn plural_resolution_exhausts_variant_locale_fallback_before_base_context() {
    catalog_resolution::plural_resolution_exhausts_variant_locale_fallback_before_base_context();
}

#[test]
fn po_markup_failures_route_structured_presentations_through_fluent() {
    assert_markup_presentation(
        "[slow]Hello[/slow]",
        "[slow]Bonjour[/slow] [ghost]now[/ghost]",
        "diagnostic-validate-048",
        &[("tag", DiagnosticArgumentValue::String("ghost".to_owned()))],
        "ALT unknown markup tag `{$tag}`",
        "ALT unknown markup tag `ghost`",
    );
    assert_markup_presentation(
        "[slow]Hello [em]world[/em][/slow]",
        "Hello [em]monde[/em]",
        "diagnostic-validate-049",
        &[("tag", DiagnosticArgumentValue::String("slow".to_owned()))],
        "ALT missing markup tag `{$tag}`",
        "ALT missing markup tag `slow`",
    );
    assert_markup_presentation(
        "[slow]Hello[/slow]",
        "[slow]Bonjour[/slow",
        "diagnostic-validate-023-bracket",
        &[],
        "ALT unbalanced markup bracket",
        "ALT unbalanced markup bracket",
    );
    assert_markup_presentation(
        "[slow]Hello[/slow]",
        "[/slow]",
        "diagnostic-validate-023-no-opening",
        &[("tag", DiagnosticArgumentValue::String("slow".to_owned()))],
        "ALT unbalanced markup tag `{$tag}` has no opening",
        "ALT unbalanced markup tag `slow` has no opening",
    );
    assert_markup_presentation(
        "[slow]Hello [em]world[/em][/slow]",
        "[slow]Bonjour [em]monde[/slow][/em]",
        "diagnostic-validate-023-mismatch",
        &[
            ("tag", DiagnosticArgumentValue::String("slow".to_owned())),
            (
                "expected_tag",
                DiagnosticArgumentValue::String("em".to_owned()),
            ),
        ],
        "ALT mismatched markup `{$tag}` before `{$expected_tag}`",
        "ALT mismatched markup `slow` before `em`",
    );
    assert_markup_presentation(
        "[slow mood=calm]Hello[/slow]",
        "[slow mood=angry]Bonjour[/slow]",
        "diagnostic-validate-047",
        &[
            ("tag", DiagnosticArgumentValue::String("slow".to_owned())),
            (
                "expected",
                DiagnosticArgumentValue::String("mood=calm".to_owned()),
            ),
            (
                "actual",
                DiagnosticArgumentValue::String("mood=angry".to_owned()),
            ),
        ],
        "ALT markup attributes `{$tag}`: {$expected} -> {$actual}",
        "ALT markup attributes `slow`: mood=calm -> mood=angry",
    );

    let standalone = recite_core::DiagnosticPresentation::from_arguments(
        recite_core::DiagnosticPresentationId::new_static("diagnostic-validate-023-standalone"),
        [("tag", DiagnosticArgumentValue::String("br".to_owned()))],
    )
    .expect("standalone markup presentation");
    assert_alternate_markup_presentation(
        &standalone,
        "diagnostic-validate-023-standalone",
        &[("tag", DiagnosticArgumentValue::String("br".to_owned()))],
        "ALT standalone markup tag `{$tag}`",
        "ALT standalone markup tag `br`",
    );
}
