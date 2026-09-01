//! MessagePack v0 tagged-value encoders owned by recite-core.
//!
//! This module is the compiler encode half of the same wire format decoded by
//! `tags.rs`. Both halves are keyed
//! by the shared `crate::V0_*` tag constants; add, remove, or renumber
//! tags in the encoder, decoder, and constant definitions together. From the
//! first tagged release onward, tag changes also require the versioning
//! policy in `docs/recite-production-spec.md` §12.2.

use crate::{
    CompiledArgument, CompiledAssetEncoding, CompiledChoiceEcho, CompiledConditionCall,
    CompiledConditionExpression, CompiledDivertTarget, CompiledEffectMode,
    CompiledInspectionEncoding, CompiledMatchPattern, CompiledStatementKind, ScalarValue,
    SchemaFingerprint, SourceSpan, Value,
};
pub(super) struct MsgAssetEncoding(pub(super) CompiledAssetEncoding);
pub(super) struct MsgInspectionEncoding(pub(super) CompiledInspectionEncoding);
pub(super) struct MsgSchemaFingerprint<'a>(pub(super) &'a SchemaFingerprint);
pub(super) struct MsgFingerprint<'a>(pub(super) &'a crate::ContentFingerprint);
pub(super) struct MsgStatementKind<'a>(pub(super) &'a CompiledStatementKind);
pub(super) struct MsgMatchPattern<'a>(pub(super) &'a CompiledMatchPattern);
pub(crate) struct MsgDivertTarget<'a>(pub(crate) &'a CompiledDivertTarget);
pub(crate) struct MsgChoiceEcho<'a>(pub(crate) &'a CompiledChoiceEcho);
pub(super) struct MsgEffectMode(pub(super) CompiledEffectMode);
pub(crate) struct MsgConditionExpression<'a>(pub(crate) &'a CompiledConditionExpression);
pub(super) struct MsgConditionCall<'a>(pub(super) &'a CompiledConditionCall);
pub(super) struct MsgArgument<'a>(pub(super) &'a CompiledArgument);
pub(super) struct MsgValue<'a>(pub(super) &'a Value);
pub(super) struct MsgScalarValue<'a>(pub(super) &'a ScalarValue);
pub(super) struct MsgSourceSpan<'a>(pub(super) &'a SourceSpan);

macro_rules! serialize_tagged {
    ($serializer:expr, $tag:expr, $payload:expr) => {{
        let mut tuple = $serializer.serialize_tuple(crate::V0_TAGGED_VALUE_FIELDS as usize)?;
        tuple.serialize_element(&$tag)?;
        tuple.serialize_element(&$payload)?;
        tuple.end()
    }};
}

mod control;
mod values;
