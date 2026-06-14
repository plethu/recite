use std::fmt;
use std::marker::PhantomData;

use serde::Deserialize;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde_json::Value;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawManifest {
    pub(crate) schema_version: Value,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) types: Vec<Named<RawTypeDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) registries: Vec<Named<RawRegistryDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) speakers: Vec<Named<RawSpeakerDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) conditions: Vec<Named<RawConditionDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) availability_reasons: Vec<Named<RawAvailabilityReasonDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) effects: Vec<Named<RawEffectDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) metadata_domains: Vec<Named<RawMetadataDomainDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) metadata: Vec<Named<RawMetadataDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) projection_queries: Vec<Named<RawProjectionQueryFunctionDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) presentation_projectors: Vec<Named<RawPresentationProjectorDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) markup: Vec<Named<RawMarkupDefinition>>,
}

#[derive(Debug)]
pub(crate) struct Named<T> {
    pub(crate) name: String,
    pub(crate) value: T,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawTypeDefinition {
    pub(crate) kind: String,
    pub(crate) values: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawRegistryDefinition {
    pub(crate) values: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) origin: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawSpeakerDefinition {
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawParameterDefinition {
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) type_ref: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConditionDefinition {
    #[serde(default)]
    pub(crate) params: Vec<RawParameterDefinition>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) returns: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) availability_reason: Option<RawConditionAvailabilityReasonMapping>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawConditionAvailabilityReasonMapping {
    pub(crate) reason: String,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) args: Vec<Named<Value>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawAvailabilityReasonDefinition {
    pub(crate) template: String,
    #[serde(default)]
    pub(crate) params: Vec<RawParameterDefinition>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) origin: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawEffectDefinition {
    pub(crate) modes: Vec<String>,
    #[serde(default)]
    pub(crate) params: Vec<RawParameterDefinition>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawMetadataDefinition {
    pub(crate) targets: Vec<String>,
    #[serde(rename = "type")]
    pub(crate) type_ref: String,
    #[serde(default)]
    pub(crate) repeatable: bool,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) domain: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawMetadataDomainDefinition {
    pub(crate) kind: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) values: Option<Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) selector: Option<String>,
    #[serde(default, deserialize_with = "deserialize_optional_named_entries")]
    pub(crate) values_by_context: Option<Vec<Named<Vec<String>>>>,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) missing_context: Option<RawMissingMetadataContext>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawProjectionQueryFunctionDefinition {
    #[serde(default)]
    pub(crate) params: Vec<RawParameterDefinition>,
    pub(crate) returns: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) max_calls_per_event: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPresentationProjectorDefinition {
    pub(crate) candidates: RawProjectionSelector,
    #[serde(default)]
    pub(crate) inputs: Vec<RawProjectionInput>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) queries: Vec<Named<RawProjectionQueryDefinition>>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) outputs: Vec<Named<RawPresentationAffordanceOutputDefinition>>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RawProjectionSelector {
    RuntimeEvent {
        event: String,
    },
    MetadataKey {
        target: String,
        key: String,
    },
    MetadataSet {
        target: String,
        required_keys: Vec<String>,
    },
    AvailabilityReason {
        reason: String,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawProjectionInput {
    pub(crate) name: String,
    pub(crate) source: RawProjectionInputSource,
    #[serde(rename = "type")]
    pub(crate) type_ref: String,
    #[serde(default)]
    pub(crate) required: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RawProjectionInputSource {
    EventKind,
    CandidateLineId,
    CandidateChoiceId,
    CandidateEffectRequestId,
    CandidateBlockId,
    CandidateProject,
    CandidateMetadata {
        key: String,
        #[serde(default, deserialize_with = "deserialize_optional_non_null")]
        occurrence: Option<RawMetadataOccurrence>,
    },
    AvailabilityReasonArg {
        name: String,
    },
    Literal {
        value: Value,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawMetadataOccurrence {
    Named(String),
    Index { index: u32 },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawProjectionQueryDefinition {
    pub(crate) function: String,
    #[serde(default)]
    pub(crate) args: Vec<RawProjectionInputRef>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawProjectionInputRef {
    Input { input: String },
    QueryResult { query_result: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPresentationAffordanceOutputDefinition {
    pub(crate) target: String,
    pub(crate) kind: String,
    pub(crate) slot: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) label: Option<RawPresentationLabelDefinition>,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) fields: Vec<Named<RawPresentationAffordanceFieldDefinition>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPresentationLabelDefinition {
    pub(crate) template_id: String,
    pub(crate) source_text: String,
    #[serde(default, deserialize_with = "deserialize_named_entries")]
    pub(crate) args: Vec<Named<RawPresentationLabelArgDefinition>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPresentationLabelArgDefinition {
    pub(crate) source: RawProjectionInputRef,
    #[serde(rename = "type")]
    pub(crate) type_ref: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawPresentationAffordanceFieldDefinition {
    pub(crate) source: RawPresentationAffordanceFieldSource,
    #[serde(rename = "type")]
    pub(crate) type_ref: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum RawPresentationAffordanceFieldSource {
    Input { name: String },
    QueryResult { name: String },
    Literal { value: Value },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawMissingMetadataContext {
    pub(crate) policy: String,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) domain: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawMarkupDefinition {
    pub(crate) requires_closing: bool,
    pub(crate) translatable: bool,
    #[serde(default, deserialize_with = "deserialize_optional_non_null")]
    pub(crate) allows_nesting: Option<bool>,
}

fn deserialize_optional_non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

fn deserialize_named_entries<'de, D, T>(deserializer: D) -> Result<Vec<Named<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct NamedEntriesVisitor<T> {
        marker: PhantomData<T>,
    }

    impl<'de, T> Visitor<'de> for NamedEntriesVisitor<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<Named<T>>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON object")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut entries = Vec::new();
            while let Some((name, value)) = map.next_entry::<String, T>()? {
                entries.push(Named { name, value });
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_map(NamedEntriesVisitor {
        marker: PhantomData,
    })
}

fn deserialize_optional_named_entries<'de, D, T>(
    deserializer: D,
) -> Result<Option<Vec<Named<T>>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    deserialize_named_entries(deserializer).map(Some)
}
