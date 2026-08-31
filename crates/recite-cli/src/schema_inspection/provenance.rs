use std::str::FromStr;

use recite_compiler::SchemaDeclarationProvenance;
use recite_core::{ProducerIdentity, ProducerMetadataValue, ProducerOrigin};

use super::model::{
    IdentityProjection, OriginProjection, OwnershipProjection, ProvenanceProjection,
};

pub(super) fn ownership_json(ownership: &recite_compiler::SchemaOwnership) -> OwnershipProjection {
    match ownership {
        recite_compiler::SchemaOwnership::Standalone { producer } => {
            OwnershipProjection::Standalone {
                producer: identity_json(producer),
            }
        }
        recite_compiler::SchemaOwnership::Generated { producer } => {
            OwnershipProjection::Generated {
                producer: identity_json(producer),
            }
        }
        recite_compiler::SchemaOwnership::Unavailable => OwnershipProjection::Unavailable,
        _ => OwnershipProjection::Unavailable,
    }
}

pub(super) fn identity_json(identity: &ProducerIdentity) -> IdentityProjection {
    IdentityProjection {
        kind: identity.kind().to_owned(),
        id: identity.id().to_owned(),
    }
}

pub(super) fn provenance_json(value: &SchemaDeclarationProvenance) -> ProvenanceProjection {
    ProvenanceProjection {
        ownership: ownership_json(value.ownership()),
        origin: value.origin().map(origin_json),
    }
}

pub(super) fn origin_json(value: &ProducerOrigin) -> OriginProjection {
    OriginProjection {
        kind: value.kind.clone(),
        id: value.id.clone(),
        label: value.label.clone(),
        extensions: value
            .extensions
            .iter()
            .map(|(key, value)| (key.clone(), producer_metadata_value_json(value)))
            .collect(),
    }
}

fn producer_metadata_value_json(value: &ProducerMetadataValue) -> serde_json::Value {
    match value {
        ProducerMetadataValue::Null => serde_json::Value::Null,
        ProducerMetadataValue::Bool(value) => serde_json::json!(value),
        ProducerMetadataValue::Number(value) => serde_json::Number::from_str(value).map_or_else(
            |_| serde_json::Value::String(value.clone()),
            serde_json::Value::Number,
        ),
        ProducerMetadataValue::String(value) => serde_json::json!(value),
        ProducerMetadataValue::Array(values) => {
            serde_json::Value::Array(values.iter().map(producer_metadata_value_json).collect())
        }
        ProducerMetadataValue::Object(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), producer_metadata_value_json(value)))
                .collect(),
        ),
    }
}
