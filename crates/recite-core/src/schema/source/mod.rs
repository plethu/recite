//! Source-owning, editable schema declarations.
//!
//! A [`SchemaSource`] retains the TOML concrete syntax tree while exposing the
//! canonical [`ProjectSchema`] to compiler and tooling callers.  Generated
//! JSON is an export only; it is deliberately not used when lowering TOML.

mod declarations;
mod diagnostics;
mod edit;
mod enum_values;
mod export;
mod fingerprint;
mod lower;
mod plan;
mod raw;
mod spans;
mod toml;
mod types;

pub use plan::SchemaSourceEditPlan;
pub use toml::{
    SchemaDeclarationKind, SchemaSource, SchemaSourceEdit, SchemaSourceEditError,
    SchemaSourceLoadReport, SchemaSourceStaleDetails, load_schema_source_str,
};
