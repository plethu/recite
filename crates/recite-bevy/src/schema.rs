use std::collections::{BTreeSet, btree_map::Entry};

use recite_core::Diagnostic;
use recite_core::ast::EffectMode;
use recite_core::schema::{
    ConditionDefinition, ConditionReturnType, EffectDefinition, ParameterDefinition,
    ProducerIdentity, ProjectSchema, SchemaTypeRef, SpeakerDefinition, export_schema_manifest_json,
    export_schema_manifest_json_with_producer,
};

/// Maps Rust declarations to canonical Recite parameter types. Host newtypes
/// can implement this with a registered enum or registry name.
pub trait ReciteType {
    fn schema_type() -> SchemaTypeRef;
}

impl ReciteType for String {
    fn schema_type() -> SchemaTypeRef {
        SchemaTypeRef::String
    }
}
impl ReciteType for bool {
    fn schema_type() -> SchemaTypeRef {
        SchemaTypeRef::Bool
    }
}
impl ReciteType for i32 {
    fn schema_type() -> SchemaTypeRef {
        SchemaTypeRef::Int
    }
}
impl ReciteType for i64 {
    fn schema_type() -> SchemaTypeRef {
        SchemaTypeRef::Int
    }
}
impl ReciteType for f32 {
    fn schema_type() -> SchemaTypeRef {
        SchemaTypeRef::Float
    }
}
impl ReciteType for f64 {
    fn schema_type() -> SchemaTypeRef {
        SchemaTypeRef::Float
    }
}

/// Named enum return domain. Register the matching enum in the schema.
pub trait ReciteEnum {
    const NAME: &'static str;
}

/// Duplicate native registration. The earlier declaration is left intact.
#[derive(Debug, thiserror::Error, Eq, PartialEq)]
#[error("duplicate {kind} declaration `{name}`")]
pub struct SchemaRegistrationError {
    pub kind: &'static str,
    pub name: String,
}

/// Rust host declarations lowered to the canonical, validated Recite model.
pub struct ReciteSchema {
    schema: ProjectSchema,
}

impl Default for ReciteSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl ReciteSchema {
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema: ProjectSchema::empty_v1(),
        }
    }

    pub fn condition(
        &mut self,
        name: impl Into<String>,
    ) -> Result<ConditionBuilder<'_>, SchemaRegistrationError> {
        let name = name.into();
        let definition = match self.schema.conditions.entry(name.clone()) {
            Entry::Vacant(entry) => entry.insert(ConditionDefinition {
                params: Vec::new(),
                returns: ConditionReturnType::Bool,
                availability_reason: None,
            }),
            Entry::Occupied(_) => {
                return Err(SchemaRegistrationError {
                    kind: "condition",
                    name,
                });
            }
        };
        Ok(ConditionBuilder { definition })
    }

    pub fn effect(
        &mut self,
        name: impl Into<String>,
    ) -> Result<EffectBuilder<'_>, SchemaRegistrationError> {
        let name = name.into();
        let definition = match self.schema.effects.entry(name.clone()) {
            Entry::Vacant(entry) => entry.insert(EffectDefinition {
                modes: BTreeSet::new(),
                params: Vec::new(),
            }),
            Entry::Occupied(_) => {
                return Err(SchemaRegistrationError {
                    kind: "effect",
                    name,
                });
            }
        };
        Ok(EffectBuilder { definition })
    }

    pub fn speaker(
        &mut self,
        name: impl Into<String>,
        definition: SpeakerDefinition,
    ) -> Result<(), SchemaRegistrationError> {
        let name = name.into();
        match self.schema.speakers.entry(name.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(definition);
                Ok(())
            }
            Entry::Occupied(_) => Err(SchemaRegistrationError {
                kind: "speaker",
                name,
            }),
        }
    }

    /// Full canonical model for registries, metadata domains, projection
    /// declarations, and provenance supported by Recite core.
    pub fn schema_mut(&mut self) -> &mut ProjectSchema {
        &mut self.schema
    }

    #[must_use]
    pub fn schema(&self) -> &ProjectSchema {
        &self.schema
    }

    pub fn export_json(&self) -> Result<String, Vec<Diagnostic>> {
        export_schema_manifest_json(&self.schema)
    }

    pub fn export_json_with_producer(
        &self,
        producer: ProducerIdentity,
    ) -> Result<String, Vec<Diagnostic>> {
        export_schema_manifest_json_with_producer(&self.schema, producer)
    }
}

pub struct ConditionBuilder<'a> {
    definition: &'a mut ConditionDefinition,
}

impl ConditionBuilder<'_> {
    #[must_use]
    pub fn param<T: ReciteType>(self, name: impl Into<String>) -> Self {
        self.definition.params.push(ParameterDefinition {
            name: name.into(),
            type_ref: T::schema_type(),
        });
        self
    }

    pub fn returns_bool(self) {
        self.definition.returns = ConditionReturnType::Bool;
    }

    pub fn returns_enum<T: ReciteEnum>(self) {
        self.definition.returns = ConditionReturnType::Enum(T::NAME.to_owned());
    }
}

pub struct EffectBuilder<'a> {
    definition: &'a mut EffectDefinition,
}

impl EffectBuilder<'_> {
    #[must_use]
    pub fn param<T: ReciteType>(self, name: impl Into<String>) -> Self {
        self.definition.params.push(ParameterDefinition {
            name: name.into(),
            type_ref: T::schema_type(),
        });
        self
    }

    #[must_use]
    pub fn mode(self, mode: EffectMode) -> Self {
        self.definition.modes.insert(mode);
        self
    }

    #[must_use]
    pub fn immediate(self) -> Self {
        self.mode(EffectMode::Immediate)
    }
    #[must_use]
    pub fn blocking(self) -> Self {
        self.mode(EffectMode::Blocking)
    }
    #[must_use]
    pub fn deferred(self) -> Self {
        self.mode(EffectMode::Deferred)
    }
}

/// Typed declarations expanded in source order and lowered through the same
/// builder as manual registration. This adds no shipping compiler dependency.
#[macro_export]
macro_rules! recite_schema {
    ($schema:expr;) => { Ok::<(), $crate::SchemaRegistrationError>(()) };
    ($schema:expr; condition $name:ident ($($param:ident : $ty:ty),* $(,)?) -> bool; $($rest:tt)*) => { (|| {
        let declaration = $schema.condition(stringify!($name))?;
        $(let declaration = declaration.param::<$ty>(stringify!($param));)*
        declaration.returns_bool();
        $crate::recite_schema!($schema; $($rest)*)
    })() };
    ($schema:expr; condition $name:ident ($($param:ident : $ty:ty),* $(,)?) -> $return:ty; $($rest:tt)*) => { (|| {
        let declaration = $schema.condition(stringify!($name))?;
        $(let declaration = declaration.param::<$ty>(stringify!($param));)*
        declaration.returns_enum::<$return>();
        $crate::recite_schema!($schema; $($rest)*)
    })() };
    ($schema:expr; effect $name:ident ($($param:ident : $ty:ty),* $(,)?) [$mode:ident]; $($rest:tt)*) => { (|| {
        let declaration = $schema.effect(stringify!($name))?.$mode();
        $(let declaration = declaration.param::<$ty>(stringify!($param));)*
        let _ = declaration;
        $crate::recite_schema!($schema; $($rest)*)
    })() };
}
