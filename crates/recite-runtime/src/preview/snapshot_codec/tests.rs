use super::span::SpanWire;
use super::wire::{PluralOutcomeWire, PluralResolutionWire};

#[test]
fn nested_source_span_unknown_field_is_rejected() {
    let bytes = [
        0x84, 0xa4, b'f', b'i', b'l', b'e', 0xa1, b'f', 0xa5, b's', b't', b'a', b'r', b't', 0x82,
        0xa4, b'l', b'i', b'n', b'e', 1, 0xa6, b'c', b'o', b'l', b'u', b'm', b'n', 1, 0xa3, b'e',
        b'n', b'd', 0xc0, 0xa7, b'u', b'n', b'k', b'n', b'o', b'w', b'n', 0xc0,
    ];
    assert!(rmp_serde::from_slice::<SpanWire>(&bytes).is_err());
}

#[test]
fn plural_resolution_rejects_attempt_only_outcome() {
    let wire = PluralResolutionWire {
        attempts: Vec::new(),
        matched_locale: None,
        matched_context: None,
        matched_key: None,
        matched_arm: None,
        source_fallback_arm: None,
        outcome: PluralOutcomeWire::Matched,
    };
    assert!(wire.into_resolution().is_err());
}
