use std::collections::BTreeMap;

use crate::i18n::{Messages, UiLocale};
use crate::tui::{Keymap, PromptMode, TuiInteractionState};

use super::interaction::enum_condition_variant;
use super::*;

#[test]
fn condition_answer_cache_defaults_true_and_returns_prior_answer() {
    let mut cache = BTreeMap::new();

    assert!(cached_condition_answer(&cache, "trusts(mira)"));
    cache.insert("trusts(mira)".to_owned(), false);

    assert!(!cached_condition_answer(&cache, "trusts(mira)"));
    assert!(cached_condition_answer(&cache, "knows(mira)"));
}

#[test]
fn condition_prompt_uses_expected_type_specific_state() {
    let boolean = condition_prompt(
        ConditionExpectedType::Bool,
        "trusts(mira)".to_owned(),
        false,
        Keymap::Standard,
    );
    assert_eq!(
        boolean,
        TuiPrompt::Condition {
            query: "trusts(mira)".to_owned(),
            selected: false,
            interaction: TuiInteractionState::new(PromptMode::Insert),
        }
    );

    let enumeration = condition_prompt(
        ConditionExpectedType::Enum,
        "memory_pressure(hazel, music_shop)".to_owned(),
        true,
        Keymap::Vim,
    );
    assert_eq!(
        enumeration,
        TuiPrompt::EnumCondition {
            query: "memory_pressure(hazel, music_shop)".to_owned(),
            interaction: TuiInteractionState::new(PromptMode::Normal),
            input: TextBuffer::default(),
        }
    );
}

#[test]
fn enum_condition_variant_trims_non_empty_input() {
    let messages = Messages::load(&UiLocale::default()).expect("messages");

    assert_eq!(
        enum_condition_variant("  high  ", &messages).expect("variant"),
        "high"
    );
}

#[test]
fn enum_condition_variant_rejects_empty_input() {
    let messages = Messages::load(&UiLocale::default()).expect("messages");

    assert_eq!(
        enum_condition_variant("  ", &messages)
            .expect_err("empty input")
            .to_string(),
        "invalid play input: enter an enum variant"
    );
}
