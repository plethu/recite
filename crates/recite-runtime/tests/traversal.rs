use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    BlockIndex, BlockLookupEntry, BlockLookupTable, ChoiceId, ChoiceRange, CompiledAssetId,
    CompiledConditionCall, CompiledConditionExpression, CompiledDialogue, CompiledDivertTarget,
    CompiledStatementKind, CompilerVersion, EffectIndex, LineIndex, MatchArmIndex, MatchArmRange,
    SchemaFingerprint, SourceMapId,
};
use recite_runtime::{
    ConditionArgument, ConditionEvaluationError, ConditionQuery, DialogueEffectArgument,
    DialogueEffectMode, DialogueEffectRequest, DialogueError, DialogueEvent, EmptyDialogueContext,
    UnsupportedStatementKind, choose as runtime_choose, next as runtime_next, start_scene,
};
use std::cell::RefCell;
use std::collections::BTreeMap;

#[path = "traversal/choices.rs"]
mod choices;
#[path = "traversal/conditions.rs"]
mod conditions;
#[path = "traversal/control_flow.rs"]
mod control_flow;
#[path = "traversal/effects.rs"]
mod effects;
#[path = "traversal/malformed_assets.rs"]
mod malformed_assets;
#[path = "traversal/start_and_output.rs"]
mod start_and_output;
#[path = "traversal/support.rs"]
mod support;

use support::*;
