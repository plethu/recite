//! Compatibility decoding for evolved line and choice row shapes.

use serde::Deserialize;
use serde::de::{IgnoredAny, SeqAccess, Visitor, value::SeqAccessDeserializer};

use crate::compiled::{
    CompiledChoice, CompiledInterpolationMode, CompiledLine, SourceMapIndex, SpeakerIndex,
};
use crate::{AvailabilityReasonId, ChoiceId, LineId};

use super::super::CompiledAssetDecodeError;
use super::super::interpolation::MsgInterpolationBinding;
use super::super::tags::{MsgChoiceEcho, MsgConditionExpression, MsgDivertTarget};
use super::MsgRange;

pub(super) struct MsgLine(MsgLineRow);

enum MsgLineRow {
    Current(MsgLineCurrent),
    Legacy(MsgLineLegacy),
}

impl<'de> Deserialize<'de> for MsgLine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(MsgLineVisitor).map(Self)
    }
}

struct MsgLineVisitor;

impl<'de> Visitor<'de> for MsgLineVisitor {
    type Value = MsgLineRow;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("compiled line with 5, 7, or 9 fields")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // The typed prefix consumes only its five fields. The outer visitor owns
        // extension tails and row completion, without buffering the row.
        let prefix = MsgLineLegacy::deserialize(SeqAccessDeserializer::new(&mut seq))?;
        let Some(authored_source_text) = seq.next_element()? else {
            return Ok(MsgLineRow::Legacy(prefix));
        };
        let interpolation_bindings = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::custom("compiled line must have 5, 7, or 9 fields"))?;
        let Some(plural_source_text) = seq.next_element::<Option<String>>()? else {
            return Ok(MsgLineRow::Current(MsgLineCurrent(
                prefix.0,
                prefix.1,
                prefix.2,
                prefix.3,
                prefix.4,
                authored_source_text,
                interpolation_bindings,
                None,
                None,
            )));
        };
        let authored_plural_source_text = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::custom("compiled line must have 5, 7, or 9 fields"))?;
        finish_row(&mut seq, "compiled line must have 5, 7, or 9 fields")?;
        Ok(MsgLineRow::Current(MsgLineCurrent(
            prefix.0,
            prefix.1,
            prefix.2,
            prefix.3,
            prefix.4,
            authored_source_text,
            interpolation_bindings,
            plural_source_text,
            authored_plural_source_text,
        )))
    }
}

struct MsgLineCurrent(
    String,
    String,
    Option<u32>,
    MsgRange,
    u32,
    String,
    Vec<MsgInterpolationBinding>,
    Option<String>,
    Option<String>,
);

#[derive(Deserialize)]
struct MsgLineLegacy(String, String, Option<u32>, MsgRange, u32);

impl TryFrom<MsgLine> for CompiledLine {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgLine) -> Result<Self, Self::Error> {
        match value.0 {
            MsgLineRow::Current(value) => {
                let line = Self {
                    id: LineId::new(value.0)?,
                    source_text: value.1,
                    plural_source_text: value.7,
                    speaker: value.2.map(SpeakerIndex::new),
                    metadata: value.3.metadata(),
                    source_map: SourceMapIndex::new(value.4),
                    authored_source_text: value.5,
                    authored_plural_source_text: value.8,
                    interpolation_bindings: value
                        .6
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    interpolation_mode: CompiledInterpolationMode::Current,
                };
                Ok(line)
            }
            MsgLineRow::Legacy(value) => Ok(Self {
                id: LineId::new(value.0)?,
                source_text: value.1.clone(),
                plural_source_text: None,
                authored_source_text: value.1,
                authored_plural_source_text: None,
                interpolation_bindings: Vec::new(),
                interpolation_mode: CompiledInterpolationMode::Legacy,
                speaker: value.2.map(SpeakerIndex::new),
                metadata: value.3.metadata(),
                source_map: SourceMapIndex::new(value.4),
            }),
        }
    }
}

pub(super) struct MsgChoice(MsgChoiceRow);

enum MsgChoiceRow {
    Current(MsgChoiceCurrent),
    Legacy(MsgChoiceLegacy),
}

impl<'de> Deserialize<'de> for MsgChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(MsgChoiceVisitor).map(Self)
    }
}

struct MsgChoiceVisitor;

impl<'de> Visitor<'de> for MsgChoiceVisitor {
    type Value = MsgChoiceRow;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("compiled choice with 9 or 11 fields")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // Reuse the typed legacy prefix; only this visitor decides whether the
        // authored text and bindings tail is present and complete.
        let prefix = MsgChoiceLegacy::deserialize(SeqAccessDeserializer::new(&mut seq))?;
        let Some(authored_source_text) = seq.next_element()? else {
            return Ok(MsgChoiceRow::Legacy(prefix));
        };
        let interpolation_bindings = seq
            .next_element()?
            .ok_or_else(|| serde::de::Error::custom("compiled choice must have 11 fields"))?;
        finish_row(&mut seq, "compiled choice must have 11 fields")?;
        Ok(MsgChoiceRow::Current(MsgChoiceCurrent(
            prefix.0,
            prefix.1,
            prefix.2,
            prefix.3,
            prefix.4,
            prefix.5,
            prefix.6,
            prefix.7,
            prefix.8,
            authored_source_text,
            interpolation_bindings,
        )))
    }
}

struct MsgChoiceCurrent(
    String,
    String,
    MsgRange,
    Option<MsgConditionExpression>,
    Option<String>,
    Option<String>,
    MsgDivertTarget,
    MsgChoiceEcho,
    u32,
    String,
    Vec<MsgInterpolationBinding>,
);

#[derive(Deserialize)]
struct MsgChoiceLegacy(
    String,
    String,
    MsgRange,
    Option<MsgConditionExpression>,
    Option<String>,
    Option<String>,
    MsgDivertTarget,
    MsgChoiceEcho,
    u32,
);

impl TryFrom<MsgChoice> for CompiledChoice {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgChoice) -> Result<Self, Self::Error> {
        match value.0 {
            MsgChoiceRow::Current(value) => {
                let choice = Self {
                    id: ChoiceId::new(value.0)?,
                    source_text: value.1,
                    metadata: value.2.metadata(),
                    availability_requirement: value.3.map(|condition| condition.0),
                    availability_requirement_source_text: value.4,
                    availability_reason_override: value
                        .5
                        .map(AvailabilityReasonId::new)
                        .transpose()?,
                    target: value.6.0,
                    echo: value.7.0,
                    source_map: SourceMapIndex::new(value.8),
                    authored_source_text: value.9,
                    interpolation_bindings: value
                        .10
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    interpolation_mode: CompiledInterpolationMode::Current,
                };
                Ok(choice)
            }
            MsgChoiceRow::Legacy(value) => Ok(Self {
                id: ChoiceId::new(value.0)?,
                source_text: value.1.clone(),
                authored_source_text: value.1,
                interpolation_bindings: Vec::new(),
                interpolation_mode: CompiledInterpolationMode::Legacy,
                metadata: value.2.metadata(),
                availability_requirement: value.3.map(|condition| condition.0),
                availability_requirement_source_text: value.4,
                availability_reason_override: value.5.map(AvailabilityReasonId::new).transpose()?,
                target: value.6.0,
                echo: value.7.0,
                source_map: SourceMapIndex::new(value.8),
            }),
        }
    }
}

fn finish_row<'de, A>(seq: &mut A, message: &'static str) -> Result<(), A::Error>
where
    A: SeqAccess<'de>,
{
    if seq.next_element::<IgnoredAny>()?.is_some() {
        return Err(serde::de::Error::custom(message));
    }
    Ok(())
}
