use super::{Tagged, WireConditionExpression, WireRange};
use serde::Serialize;
use serde::ser::SerializeTuple;

pub(crate) struct WireLine<'a> {
    pub(crate) id: &'a str,
    pub(crate) source_text: &'a str,
    pub(crate) speaker: Option<u32>,
    pub(crate) metadata: WireRange,
    pub(crate) source_map: u32,
}

impl Serialize for WireLine<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(7)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.speaker)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&Vec::<(&str, &str, &str)>::new())?;
        tuple.end()
    }
}

pub(crate) struct WireChoice<'a> {
    pub(crate) id: &'a str,
    pub(crate) source_text: &'a str,
    pub(crate) metadata: WireRange,
    pub(crate) availability_requirement: Option<WireConditionExpression<'a>>,
    pub(crate) availability_requirement_source_text: Option<&'a str>,
    pub(crate) availability_reason_override: Option<&'a str>,
    pub(crate) target: Tagged<u32>,
    pub(crate) echo: Tagged<&'a str>,
    pub(crate) source_map: u32,
}

impl Serialize for WireChoice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(11)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.availability_requirement)?;
        tuple.serialize_element(&self.availability_requirement_source_text)?;
        tuple.serialize_element(&self.availability_reason_override)?;
        tuple.serialize_element(&self.target)?;
        tuple.serialize_element(&self.echo)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&Vec::<(&str, &str, &str)>::new())?;
        tuple.end()
    }
}

pub(crate) struct WireCurrentLine<'a> {
    pub(crate) id: &'a str,
    pub(crate) source_text: &'a str,
    pub(crate) speaker: Option<u32>,
    pub(crate) metadata: WireRange,
    pub(crate) source_map: u32,
    pub(crate) authored_source_text: &'a str,
    pub(crate) interpolation_bindings: Vec<WireInterpolationBinding<'a>>,
}

impl Serialize for WireCurrentLine<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(7)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.speaker)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.serialize_element(&self.authored_source_text)?;
        tuple.serialize_element(&self.interpolation_bindings)?;
        tuple.end()
    }
}

pub(crate) struct WireLegacyLine<'a>(
    pub(crate) &'a str,
    pub(crate) &'a str,
    pub(crate) Option<u32>,
    pub(crate) WireRange,
    pub(crate) u32,
);

impl Serialize for WireLegacyLine<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(5)?;
        tuple.serialize_element(&self.0)?;
        tuple.serialize_element(&self.1)?;
        tuple.serialize_element(&self.2)?;
        tuple.serialize_element(&self.3)?;
        tuple.serialize_element(&self.4)?;
        tuple.end()
    }
}

pub(crate) struct WireCurrentChoice<'a> {
    pub(crate) id: &'a str,
    pub(crate) source_text: &'a str,
    pub(crate) metadata: WireRange,
    pub(crate) availability_requirement: Option<WireConditionExpression<'a>>,
    pub(crate) availability_requirement_source_text: Option<&'a str>,
    pub(crate) availability_reason_override: Option<&'a str>,
    pub(crate) target: Tagged<u32>,
    pub(crate) echo: Tagged<&'a str>,
    pub(crate) source_map: u32,
    pub(crate) authored_source_text: &'a str,
    pub(crate) interpolation_bindings: Vec<WireInterpolationBinding<'a>>,
}

impl Serialize for WireCurrentChoice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(11)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.availability_requirement)?;
        tuple.serialize_element(&self.availability_requirement_source_text)?;
        tuple.serialize_element(&self.availability_reason_override)?;
        tuple.serialize_element(&self.target)?;
        tuple.serialize_element(&self.echo)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.serialize_element(&self.authored_source_text)?;
        tuple.serialize_element(&self.interpolation_bindings)?;
        tuple.end()
    }
}

pub(crate) struct WireLegacyChoice<'a>(
    pub(crate) &'a str,
    pub(crate) &'a str,
    pub(crate) WireRange,
    pub(crate) Option<WireConditionExpression<'a>>,
    pub(crate) Option<&'a str>,
    pub(crate) Option<&'a str>,
    pub(crate) Tagged<u32>,
    pub(crate) Tagged<&'a str>,
    pub(crate) u32,
);

impl Serialize for WireLegacyChoice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(9)?;
        tuple.serialize_element(&self.0)?;
        tuple.serialize_element(&self.1)?;
        tuple.serialize_element(&self.2)?;
        tuple.serialize_element(&self.3)?;
        tuple.serialize_element(&self.4)?;
        tuple.serialize_element(&self.5)?;
        tuple.serialize_element(&self.6)?;
        tuple.serialize_element(&self.7)?;
        tuple.serialize_element(&self.8)?;
        tuple.end()
    }
}

pub(crate) struct WireInterpolationBinding<'a>(
    pub(crate) &'a str,
    pub(crate) &'a str,
    pub(crate) &'a str,
);

impl Serialize for WireInterpolationBinding<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(&self.0)?;
        tuple.serialize_element(&self.1)?;
        tuple.serialize_element(&self.2)?;
        tuple.end()
    }
}
