mod asset;
mod conditions;
mod effects;
mod ids;
mod markup;
mod metadata;
mod project;

use recite_core::SchemaTypeRef;

pub(crate) use asset::*;
pub(crate) use conditions::*;
pub(crate) use effects::*;
pub(crate) use ids::*;
pub(crate) use markup::*;
pub(crate) use metadata::*;
pub(crate) use project::*;

pub(crate) fn display_schema_type_ref(type_ref: &SchemaTypeRef) -> String {
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
        SchemaTypeRef::Array(inner) => format!("array:{}", display_schema_type_ref(inner)),
    }
}
