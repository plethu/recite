//! Source-owning, editable schema declarations.
//!
//! A [`SchemaSource`] retains the TOML concrete syntax tree while exposing the
//! canonical [`ProjectSchema`] to compiler and tooling callers.  Generated
//! JSON is an export only; it is deliberately not used when lowering TOML.

mod diagnostics;
mod edit;
mod export;
mod fingerprint;
mod lower;
mod raw;
mod spans;
mod toml;

pub use toml::{
    SchemaDeclarationKind, SchemaSource, SchemaSourceEdit, SchemaSourceEditError,
    SchemaSourceLoadReport, load_schema_source_str,
};
