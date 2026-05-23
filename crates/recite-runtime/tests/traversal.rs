use recite_core::{
    BlockIndex, BlockLookupEntry, BlockLookupTable, ChoiceId, ChoiceRange, CompiledConditionCall,
    CompiledConditionExpression, CompiledDialogue, CompiledDivertTarget, CompiledStatementKind,
    EffectId, EffectIndex, LineIndex, LocaleId, MatchArmIndex, MatchArmRange,
};
use recite_runtime::{
    ConditionArgument, ConditionEvaluationError, ConditionQuery, DialogueEffectArgument,
    DialogueEffectMode, DialogueEffectRequest, DialogueError, DialogueEvent,
    DialogueSessionOptions, EffectAck, EmptyDialogueContext, LocaleProvider, TextDomain,
    UnsupportedStatementKind, acknowledge_effect, choose as runtime_choose,
    choose_with_locale_provider_and_variant, next as runtime_next, next_with_locale_provider,
    next_with_locale_provider_and_variant, start_scene, start_scene_with_options,
};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[path = "traversal/choices.rs"]
mod choices;
#[path = "traversal/conditions.rs"]
mod conditions;
#[path = "traversal/control_flow.rs"]
mod control_flow;
#[path = "traversal/effects/mod.rs"]
mod effects;
#[path = "traversal/localisation.rs"]
mod localisation;
#[path = "traversal/malformed_assets.rs"]
mod malformed_assets;
#[path = "support/shared.rs"]
mod shared_support;
#[path = "traversal/start_and_output.rs"]
mod start_and_output;
#[path = "traversal/support.rs"]
mod support;

use shared_support::*;
use support::*;
