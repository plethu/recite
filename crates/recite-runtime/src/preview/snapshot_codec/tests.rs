use super::span::SpanWire;
use super::wire::{
    EffectWire, PluralOutcomeWire, PluralResolutionWire, PromptWire, StatusWire,
    WaitingForChoiceWire, WaitingForEffectWire,
};
use crate::{DialogueEffectArgument, DialogueEffectMode, DialogueEffectRequest};
use recite_core::{EffectId, SourcePosition, SourceSpan};
use serde::Serialize;

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

#[test]
fn waiting_for_choice_rejects_unknown_variant_payload_fields() {
    let payload = WaitingForChoiceWire {
        prompt: Box::new(PromptWire {
            block: "start".to_owned(),
            line: None,
            choices: Vec::new(),
            plural_arm_count: None,
            line_projection: None,
            choice_projection: Vec::new(),
        }),
    };
    let valid_payload = rmp_serde::to_vec_named(&payload).expect("choice payload");
    assert_eq!(
        rmp_serde::to_vec_named(&LegacyStatusWire::WaitingForChoice {
            prompt: payload.prompt.clone(),
        })
        .expect("legacy choice payload"),
        rmp_serde::to_vec_named(&StatusWire::WaitingForChoice(payload.clone()))
            .expect("new choice payload")
    );
    assert!(
        rmp_serde::from_slice::<StatusWire>(&tagged_status(
            "WaitingForChoice",
            valid_payload.clone(),
        ))
        .is_ok()
    );
    let mut invalid_payload = valid_payload;
    assert_eq!(invalid_payload.first(), Some(&0x81));
    invalid_payload[0] = 0x82;
    invalid_payload.extend_from_slice(&[
        0xa8, b'u', b'n', b'e', b'x', b'p', b'e', b'c', b't', b'e', b'd', 0xc0,
    ]);
    let bytes = tagged_status("WaitingForChoice", invalid_payload);

    assert!(rmp_serde::from_slice::<StatusWire>(&bytes).is_err());
}

#[test]
fn status_rejects_multiple_outer_variant_entries() {
    let payload = WaitingForChoiceWire {
        prompt: Box::new(PromptWire {
            block: "start".to_owned(),
            line: None,
            choices: Vec::new(),
            plural_arm_count: None,
            line_projection: None,
            choice_projection: Vec::new(),
        }),
    };
    let mut bytes = tagged_status(
        "WaitingForChoice",
        rmp_serde::to_vec_named(&payload).expect("choice payload"),
    );
    assert!(rmp_serde::from_slice::<StatusWire>(&bytes).is_ok());

    bytes[0] = 0x82;
    bytes.extend_from_slice(&[
        0xa5, b'E', b'n', b'd', b'e', b'd', 0xa5, b'E', b'n', b'd', b'e', b'd',
    ]);
    assert!(rmp_serde::from_slice::<StatusWire>(&bytes).is_err());
}

#[test]
fn waiting_for_effect_rejects_unknown_variant_payload_fields() {
    let effect = DialogueEffectRequest {
        id: EffectId::new("effect").expect("effect id"),
        mode: DialogueEffectMode::Blocking,
        function: "f".to_owned(),
        args: vec![DialogueEffectArgument::Identifier("start".to_owned())],
        source_span: SourceSpan::point(
            "dialogue.recite",
            SourcePosition::new(1, 1).expect("source position"),
        ),
    };
    let payload = WaitingForEffectWire {
        effect: EffectWire::from_effect(&effect),
    };
    let valid_payload = rmp_serde::to_vec_named(&payload).expect("effect payload");
    assert_eq!(
        rmp_serde::to_vec_named(&LegacyStatusWire::WaitingForEffect {
            effect: payload.effect.clone(),
        })
        .expect("legacy effect payload"),
        rmp_serde::to_vec_named(&StatusWire::WaitingForEffect(payload.clone()))
            .expect("new effect payload")
    );
    assert!(
        rmp_serde::from_slice::<StatusWire>(&tagged_status(
            "WaitingForEffect",
            valid_payload.clone(),
        ))
        .is_ok()
    );
    let mut invalid_payload = valid_payload;
    assert_eq!(invalid_payload.first(), Some(&0x81));
    invalid_payload[0] = 0x82;
    invalid_payload.extend_from_slice(&[
        0xa8, b'u', b'n', b'e', b'x', b'p', b'e', b'c', b't', b'e', b'd', 0xc0,
    ]);
    let status = tagged_status("WaitingForEffect", invalid_payload);

    assert!(rmp_serde::from_slice::<StatusWire>(&status).is_err());
}

#[derive(Serialize)]
enum LegacyStatusWire {
    WaitingForChoice { prompt: Box<PromptWire> },
    WaitingForEffect { effect: EffectWire },
}

fn tagged_status(variant: &str, payload: Vec<u8>) -> Vec<u8> {
    let mut bytes = vec![0x81, 0xa0 + variant.len() as u8];
    bytes.extend_from_slice(variant.as_bytes());
    bytes.extend(payload);
    bytes
}
