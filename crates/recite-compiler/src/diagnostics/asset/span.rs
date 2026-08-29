/// Compiler-owned names for every source span validated by the compiler.
/// These also select the stable RECITE_VALIDATE008 Fluent token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceSpanOwner {
    Block,
    Comment,
    Line,
    LineSourceText,
    Choice,
    ChoiceSourceText,
    ChoiceAvailabilityRequirement,
    ConditionExpression,
    ConditionCall,
    ConditionFunction,
    ConditionArgument,
    ChoiceAvailabilityReason,
    ChoiceAvailabilityReasonId,
    ChoiceAvailabilityReasonArguments,
    ChoiceTarget,
    Divert,
    IfBranch,
    MatchBranch,
    MatchArm,
    Effect,
    EffectMode,
    EffectFunction,
    EffectCall,
    EffectArgument,
    PluralSourceText,
    MetadataEntry,
    MetadataKey,
    MetadataValue,
}

impl SourceSpanOwner {
    pub(crate) const fn compatibility_label(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Comment => "comment",
            Self::Line => "line",
            Self::LineSourceText => "line source text",
            Self::Choice => "choice",
            Self::ChoiceSourceText => "choice source text",
            Self::ChoiceAvailabilityRequirement => "choice availability requirement",
            Self::ConditionExpression => "condition expression",
            Self::ConditionCall => "condition call",
            Self::ConditionFunction => "condition function",
            Self::ConditionArgument => "condition argument",
            Self::ChoiceAvailabilityReason => "choice availability reason",
            Self::ChoiceAvailabilityReasonId => "choice availability reason id",
            Self::ChoiceAvailabilityReasonArguments => "choice availability reason arguments",
            Self::ChoiceTarget => "choice target",
            Self::Divert => "divert",
            Self::IfBranch => "if branch",
            Self::MatchBranch => "match branch",
            Self::MatchArm => "match arm",
            Self::Effect => "effect",
            Self::EffectMode => "effect mode",
            Self::EffectFunction => "effect function",
            Self::EffectCall => "effect call",
            Self::EffectArgument => "effect argument",
            Self::PluralSourceText => "plural source text",
            Self::MetadataEntry => "metadata entry",
            Self::MetadataKey => "metadata key",
            Self::MetadataValue => "metadata value",
        }
    }

    pub(crate) const fn presentation_token(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Comment => "comment",
            Self::Line => "line",
            Self::LineSourceText => "line-source-text",
            Self::Choice => "choice",
            Self::ChoiceSourceText => "choice-source-text",
            Self::ChoiceAvailabilityRequirement => "choice-availability-requirement",
            Self::ConditionExpression => "condition-expression",
            Self::ConditionCall => "condition-call",
            Self::ConditionFunction => "condition-function",
            Self::ConditionArgument => "condition-argument",
            Self::ChoiceAvailabilityReason => "choice-availability-reason",
            Self::ChoiceAvailabilityReasonId => "choice-availability-reason-id",
            Self::ChoiceAvailabilityReasonArguments => "choice-availability-reason-arguments",
            Self::ChoiceTarget => "choice-target",
            Self::Divert => "divert",
            Self::IfBranch => "if-branch",
            Self::MatchBranch => "match-branch",
            Self::MatchArm => "match-arm",
            Self::Effect => "effect",
            Self::EffectMode => "effect-mode",
            Self::EffectFunction => "effect-function",
            Self::EffectCall => "effect-call",
            Self::EffectArgument => "effect-argument",
            Self::PluralSourceText => "plural-source-text",
            Self::MetadataEntry => "metadata-entry",
            Self::MetadataKey => "metadata-key",
            Self::MetadataValue => "metadata-value",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArgumentOwner {
    Condition,
    Effect,
}
