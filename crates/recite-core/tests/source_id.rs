#![cfg(test)]

use recite_core::{
    SOURCE_ID_ANCHOR_HEX_LEN, SourceAnchor, SourceId, SourceIdKind, is_valid_source_anchor,
};

#[test]
fn source_id_parse_classifies_missing_draft_and_frozen_headers() {
    assert_eq!(SourceId::parse(None), SourceId::Missing);

    assert_eq!(
        SourceId::parse(Some("line_label@")),
        SourceId::Draft {
            label: "line_label".to_owned()
        }
    );

    let parsed = SourceId::parse(Some("choice.label-1@7f3a9c2e4b6d8f019a2b"));
    assert_eq!(
        parsed,
        SourceId::Frozen {
            label: "choice.label-1".to_owned(),
            anchor: SourceAnchor::new("7f3a9c2e4b6d8f019a2b").expect("valid source anchor")
        }
    );
    assert_eq!(
        parsed.canonical_choice_id().as_ref().map(|id| id.as_str()),
        Some("7f3a9c2e4b6d8f019a2b")
    );
    assert_eq!(
        parsed.display_text().as_deref(),
        Some("choice.label-1@7f3a9c2e4b6d8f019a2b")
    );
}

#[test]
fn source_id_parse_accepts_unicode_identifier_labels() {
    let parsed = SourceId::parse(Some("café_返答@0123456789abcdef0123"));

    assert_eq!(
        parsed,
        SourceId::Frozen {
            label: "café_返答".to_owned(),
            anchor: SourceAnchor::new("0123456789abcdef0123").expect("valid source anchor")
        }
    );
    assert_eq!(parsed.label(), Some("café_返答"));
    assert_eq!(
        parsed.canonical_line_id().as_ref().map(|id| id.as_str()),
        Some("0123456789abcdef0123")
    );
}

#[test]
fn source_id_parse_rejects_plain_extra_at_and_malformed_label_headers() {
    for raw in [
        "",
        "plain_label",
        "line@@0123456789abcdef0123",
        "@0123456789abcdef0123",
        "line label@0123456789abcdef0123",
        "1line@0123456789abcdef0123",
    ] {
        assert_eq!(
            SourceId::parse(Some(raw)),
            SourceId::Malformed {
                raw: raw.to_owned()
            }
        );
    }
}

#[test]
fn source_id_parse_rejects_malformed_anchors() {
    for raw in [
        "line@0123456789abcdef012",
        "line@0123456789abcdef01234",
        "line@0123456789abcdef012g",
        "line@0123456789ABCDEF0123",
        "line@0123456789abcdef012-",
    ] {
        assert_eq!(
            SourceId::parse(Some(raw)),
            SourceId::Malformed {
                raw: raw.to_owned()
            }
        );
    }
}

#[test]
fn generated_anchors_are_fixed_width_lowercase_hex_and_deterministic() {
    let first = SourceId::generated_anchor(
        "dialogue/start.recite",
        SourceIdKind::Line,
        12,
        1,
        "line",
        0,
    );
    let second = SourceId::generated_anchor(
        "dialogue/start.recite",
        SourceIdKind::Line,
        12,
        1,
        "line",
        0,
    );
    let salted = SourceId::generated_anchor(
        "dialogue/start.recite",
        SourceIdKind::Line,
        12,
        1,
        "line",
        1,
    );

    assert_eq!(first, second);
    assert_ne!(first, salted);
    assert_eq!(first.as_str().len(), SOURCE_ID_ANCHOR_HEX_LEN);
    assert!(is_valid_source_anchor(first.as_str()));
}
