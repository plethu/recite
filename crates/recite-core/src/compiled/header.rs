use super::{CompiledValueError, SchemaFingerprint};

pub const COMPILED_ASSET_FORMAT_VERSION_V0: u16 = 0;
pub const COMPILER_COMPATIBILITY_VERSION_V0: u16 = 0;

/// Header data used for format compatibility and freshness checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledAssetHeader {
    pub format_version: u16,
    pub compiler_compatibility_version: u16,
    pub primary_encoding: CompiledAssetEncoding,
    pub inspection_encoding: CompiledInspectionEncoding,
    pub compiler_version: CompilerVersion,
    pub asset_id: CompiledAssetId,
    pub source_map_id: SourceMapId,
    pub schema_fingerprint: SchemaFingerprint,
}

impl CompiledAssetHeader {
    #[must_use]
    pub fn messagepack_v0(
        compiler_version: CompilerVersion,
        asset_id: CompiledAssetId,
        source_map_id: SourceMapId,
        schema_fingerprint: SchemaFingerprint,
    ) -> Self {
        Self {
            format_version: COMPILED_ASSET_FORMAT_VERSION_V0,
            compiler_compatibility_version: COMPILER_COMPATIBILITY_VERSION_V0,
            primary_encoding: CompiledAssetEncoding::MessagePack,
            inspection_encoding: CompiledInspectionEncoding::CompactJson,
            compiler_version,
            asset_id,
            source_map_id,
            schema_fingerprint,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CompiledAssetEncoding {
    MessagePack,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CompiledInspectionEncoding {
    CompactJson,
}

macro_rules! define_non_empty_string {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, CompiledValueError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(CompiledValueError::EmptyValue {
                        kind: stringify!($name),
                    });
                }

                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

define_non_empty_string!(CompilerVersion);
define_non_empty_string!(CompiledAssetId);
define_non_empty_string!(SourceMapId);
