#![cfg(test)]

use std::collections::BTreeSet;

use recite_core::{
    GENERATED_STABLE_ID_LEN, SourceId, StableId, StableIdGenerator, is_valid_stable_id,
};

#[test]
fn trailing_id_fields_parse_as_frozen_ids() {
    let parsed = SourceId::parse_field("@desk_greeting");
    assert_eq!(
        parsed,
        SourceId::Frozen(StableId::new("desk_greeting").expect("valid stable ID"))
    );
    assert_eq!(
        parsed.canonical_line_id().as_ref().map(|id| id.as_str()),
        Some("desk_greeting")
    );
    assert_eq!(parsed.display_text().as_deref(), Some("@desk_greeting"));
    assert_eq!(SourceId::Missing.display_text(), None);
}

#[test]
fn stable_ids_are_lowercase_ascii_without_reserved_separators() {
    for valid in ["k7q3m9x", "_0e3dc4b", "relay.desk-greeting_2"] {
        assert!(is_valid_stable_id(valid), "{valid}");
    }
    for invalid in [
        "", "Desk", "1abc", "café", "a&b", "a:b", "a/b", "a@b", "a b", "-a", ".a",
    ] {
        assert!(!is_valid_stable_id(invalid), "{invalid}");
    }
}

#[test]
fn malformed_id_fields_keep_their_source_text() {
    for raw in [
        "@", "@Desk", "@1abc", "@a&b", "@a:b", "@a/b", "@a@b", "desk",
    ] {
        assert_eq!(
            SourceId::parse_field(raw),
            SourceId::Malformed {
                raw: raw.to_owned()
            },
            "{raw}"
        );
    }
}

#[test]
fn generated_ids_follow_the_authored_grammar_without_ambiguous_letters() {
    let mut generator = StableIdGenerator::seeded(7);
    let ids = (0..10_000)
        .map(|_| generator.next_id())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), 10_000);
    for id in &ids {
        assert_eq!(id.as_str().len(), GENERATED_STABLE_ID_LEN);
        assert!(is_valid_stable_id(id.as_str()), "{id}");
        assert!(id.as_str().as_bytes()[0].is_ascii_lowercase(), "{id}");
        assert!(!id.as_str().contains(['i', 'l', 'o', 'u']), "{id}");
    }
}

#[test]
fn seeded_generation_is_reproducible_and_system_generation_varies() {
    let mut first = StableIdGenerator::seeded(42);
    let mut second = StableIdGenerator::seeded(42);
    assert_eq!(first.next_id(), second.next_id());

    let system = (0..8)
        .map(|_| StableIdGenerator::from_system().next_id())
        .collect::<BTreeSet<_>>();
    assert!(
        system.len() > 1,
        "system-seeded generators repeat: {system:?}"
    );
}
