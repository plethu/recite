use recite_core::{
    CompiledArgument, CompiledChoice, CompiledChoiceEcho, CompiledEffect, CompiledEffectMode,
    CompiledMetadataEntry, LineIndex, LocaleId, MetadataRange, ScalarValue, SpeakerIndex,
};

use crate::DialogueError;
use crate::event::{
    ChoiceAvailability, ChoiceEchoMode, DialogueChoice, DialogueEffectArgument, DialogueEffectMode,
    DialogueEffectRequest, DialogueLine,
};
use crate::locale::{InterpolationValueProvider, LocaleProvider, TextDomain};

use super::asset::AssetView;
use super::trace::DialogueTrace;

/// Options used to resolve localised runtime output.
///
/// By default, runtime output uses the source text stored in the compiled
/// dialogue. Attach a [`LocaleProvider`] to look up text for the session locale;
/// attach a variant when the provider should resolve an explicit grammatical or
/// register variant such as formal/informal, masculine/feminine, or
/// polite/casual.
///
/// If no session locale is configured, or no provider is attached, source text
/// is emitted unchanged.
#[derive(Clone, Copy, Default)]
pub struct LocaleResolution<'a> {
    provider: Option<&'a dyn LocaleProvider>,
    variant: Option<&'a str>,
    values: Option<&'a dyn InterpolationValueProvider>,
    trace: Option<&'a DialogueTrace>,
    require_plural_arm_count: bool,
}

impl<'a> LocaleResolution<'a> {
    /// Creates locale resolution options that emit source text.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves text through the provided locale provider.
    #[must_use]
    pub fn with_provider(mut self, provider: &'a dyn LocaleProvider) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Requests an explicit grammatical or register variant.
    #[must_use]
    pub fn with_variant(mut self, variant: &'a str) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Supplies typed caller-owned values for named interpolation bindings.
    #[must_use]
    pub fn with_values(mut self, values: &'a dyn InterpolationValueProvider) -> Self {
        self.values = Some(values);
        self
    }

    /// Captures selected localized templates for trace/debug consumers.
    #[must_use]
    pub fn with_trace(mut self, trace: &'a DialogueTrace) -> Self {
        self.trace = Some(trace);
        self
    }

    pub(crate) fn with_preview_plural_arm_validation(mut self) -> Self {
        self.require_plural_arm_count = true;
        self
    }

    /// Returns the locale provider used for resolution, if any.
    #[must_use]
    pub fn provider(&self) -> Option<&'a dyn LocaleProvider> {
        self.provider
    }

    /// Returns the provider-specific variant used for resolution, if any.
    #[must_use]
    pub fn variant(&self) -> Option<&'a str> {
        self.variant
    }

    #[must_use]
    pub fn values(&self) -> Option<&'a dyn InterpolationValueProvider> {
        self.values
    }
}

#[derive(Clone, Copy)]
pub(super) struct LocaleLookup<'a> {
    pub(super) locale: Option<&'a LocaleId>,
    pub(super) variant: Option<&'a str>,
    pub(super) provider: Option<&'a dyn LocaleProvider>,
    pub(super) values: Option<&'a dyn InterpolationValueProvider>,
    pub(super) trace: Option<&'a super::trace::DialogueTrace>,
    pub(super) require_plural_arm_count: bool,
}

impl<'a> LocaleLookup<'a> {
    pub(super) fn source() -> Self {
        Self {
            locale: None,
            variant: None,
            provider: None,
            values: None,
            trace: None,
            require_plural_arm_count: false,
        }
    }

    pub(super) fn from_resolution(
        locale: Option<&'a LocaleId>,
        resolution: LocaleResolution<'a>,
    ) -> Self {
        Self {
            locale,
            variant: resolution.variant,
            provider: resolution.provider,
            values: resolution.values,
            trace: resolution.trace,
            require_plural_arm_count: resolution.require_plural_arm_count,
        }
    }
}

pub(super) fn dialogue_line(
    asset: AssetView<'_>,
    line_index: LineIndex,
    default_speaker: Option<SpeakerIndex>,
    locale: LocaleLookup<'_>,
) -> Result<DialogueLine, DialogueError> {
    let line = asset.line_at(line_index)?;
    let (text, source_text, plural) = if let (Some(plural), Some(authored_plural)) = (
        line.plural_source_text.as_deref(),
        line.authored_plural_source_text.as_deref(),
    ) {
        super::interpolation::localise_plural_text(
            line.id.as_str(),
            super::interpolation::PluralSource {
                authored_singular: &line.authored_source_text,
                authored_plural,
                decoded_singular: &line.source_text,
                decoded_plural: plural,
            },
            &line.interpolation_bindings,
            line.interpolation_mode,
            locale,
        )
        .map(|(text, source_text, plural)| (text, source_text, Some(plural)))?
    } else {
        let text = super::interpolation::localise_text(
            line.id.as_str(),
            &line.authored_source_text,
            TextDomain::Line,
            &line.interpolation_bindings,
            line.interpolation_mode,
            locale,
        )?;
        (text, line.source_text.clone(), None)
    };

    Ok(DialogueLine {
        id: line.id.clone(),
        source_text,
        text,
        speaker: line
            .speaker
            .or(default_speaker)
            .map(|speaker| asset.speaker_at(speaker).map(|speaker| speaker.id.clone()))
            .transpose()?,
        metadata: metadata(asset, line.metadata)?,
        plural,
    })
}

pub(super) fn dialogue_choice(
    asset: AssetView<'_>,
    choice: &CompiledChoice,
    availability: ChoiceAvailability,
    locale: LocaleLookup<'_>,
) -> Result<DialogueChoice, DialogueError> {
    let text = super::interpolation::localise_text(
        choice.id.as_str(),
        &choice.authored_source_text,
        TextDomain::Choice,
        &choice.interpolation_bindings,
        choice.interpolation_mode,
        locale,
    )?;

    Ok(DialogueChoice {
        id: choice.id.clone(),
        source_text: choice.source_text.clone(),
        text,
        metadata: metadata(asset, choice.metadata)?,
        availability,
        echo: choice_echo(&choice.echo),
    })
}

pub(crate) fn dialogue_effect_request(
    asset: AssetView<'_>,
    effect: &CompiledEffect,
) -> Result<DialogueEffectRequest, DialogueError> {
    Ok(DialogueEffectRequest {
        id: effect.id.clone(),
        mode: effect_mode(effect.mode),
        function: effect.function.clone(),
        args: effect.args.iter().map(effect_argument).collect(),
        source_span: asset.source_map_at(effect.source_map)?.span.clone(),
    })
}

pub(super) fn effect_mode(mode: CompiledEffectMode) -> DialogueEffectMode {
    match mode {
        CompiledEffectMode::Deferred => DialogueEffectMode::Deferred,
        CompiledEffectMode::Immediate => DialogueEffectMode::Immediate,
        CompiledEffectMode::Blocking => DialogueEffectMode::Blocking,
    }
}

fn choice_echo(echo: &CompiledChoiceEcho) -> ChoiceEchoMode {
    match echo {
        CompiledChoiceEcho::None => ChoiceEchoMode::None,
        CompiledChoiceEcho::SelectedText => ChoiceEchoMode::SelectedText,
        CompiledChoiceEcho::ExplicitLine(line_id) => ChoiceEchoMode::ExplicitLine(line_id.clone()),
    }
}

fn effect_argument(argument: &CompiledArgument) -> DialogueEffectArgument {
    match argument {
        CompiledArgument::Identifier(value) => DialogueEffectArgument::Identifier(value.clone()),
        CompiledArgument::Value(ScalarValue::String(value)) => {
            DialogueEffectArgument::String(value.clone())
        }
        CompiledArgument::Value(ScalarValue::Integer(value)) => {
            DialogueEffectArgument::Integer(*value)
        }
        CompiledArgument::Value(ScalarValue::Float(value)) => DialogueEffectArgument::Float(*value),
        CompiledArgument::Value(ScalarValue::Boolean(value)) => {
            DialogueEffectArgument::Boolean(*value)
        }
    }
}

fn metadata(
    asset: AssetView<'_>,
    range: MetadataRange,
) -> Result<Vec<recite_core::MetadataEntry>, DialogueError> {
    asset
        .metadata_entries(range)?
        .iter()
        .map(|entry| metadata_entry(asset, entry))
        .collect()
}

fn metadata_entry(
    asset: AssetView<'_>,
    entry: &CompiledMetadataEntry,
) -> Result<recite_core::MetadataEntry, DialogueError> {
    Ok(recite_core::MetadataEntry {
        key: entry.key.clone(),
        value: entry.value.clone(),
        source_span: entry
            .source_map
            .map(|source_map| {
                asset
                    .source_map_at(source_map)
                    .map(|entry| entry.span.clone())
            })
            .transpose()?,
        key_span: None,
        value_span: None,
    })
}
