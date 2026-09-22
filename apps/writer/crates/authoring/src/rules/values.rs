use crate::EditError;
use recite_core::{
    Argument, ParameterDefinition, ProjectSchema, ScalarValue, SchemaTypeDefinition, SchemaTypeRef,
};

#[derive(Clone, Debug, PartialEq)]
pub struct RuleArgument {
    pub label: String,
    pub value: String,
    pub type_name: String,
    pub choices: Vec<String>,
    original: String,
    kind: SchemaTypeRef,
    quoted: bool,
}
impl RuleArgument {
    pub(super) fn new(
        argument: &Argument,
        parameter: Option<&ParameterDefinition>,
        schema: Option<&ProjectSchema>,
        index: usize,
    ) -> Self {
        let (value, quoted, inferred) = match argument {
            Argument::Identifier(value) => (value.clone(), false, SchemaTypeRef::Symbol),
            Argument::Value(ScalarValue::String(value)) => {
                (value.clone(), true, SchemaTypeRef::String)
            }
            Argument::Value(ScalarValue::Integer(value)) => {
                (value.to_string(), false, SchemaTypeRef::Int)
            }
            Argument::Value(ScalarValue::Float(value)) => {
                (value.to_string(), false, SchemaTypeRef::Float)
            }
            Argument::Value(ScalarValue::Boolean(value)) => {
                (value.to_string(), false, SchemaTypeRef::Bool)
            }
        };
        let kind = parameter.map_or(inferred, |p| p.type_ref.clone());
        let choices = match (&kind, schema) {
            (SchemaTypeRef::Bool, _) => vec!["false".into(), "true".into()],
            (SchemaTypeRef::Registry(name), Some(schema)) => schema
                .registries
                .get(name)
                .map(|r| r.values.iter().cloned().collect())
                .unwrap_or_default(),
            (SchemaTypeRef::Enum(name), Some(schema)) => schema
                .types
                .get(name)
                .map(|SchemaTypeDefinition::Enum(e)| e.values.iter().cloned().collect())
                .unwrap_or_default(),
            (SchemaTypeRef::Speaker, Some(schema)) => schema.speakers.keys().cloned().collect(),
            _ => Vec::new(),
        };
        let type_name = match &kind {
            SchemaTypeRef::String => "Text".into(),
            SchemaTypeRef::Symbol => "Symbol".into(),
            SchemaTypeRef::Int => "Whole number".into(),
            SchemaTypeRef::Float => "Number".into(),
            SchemaTypeRef::Bool => "True / false".into(),
            SchemaTypeRef::Speaker => "Speaker".into(),
            SchemaTypeRef::Enum(name) | SchemaTypeRef::Registry(name) => name.clone(),
            SchemaTypeRef::Array(_) => "Array (edit in Source)".into(),
        };
        Self {
            label: parameter.map_or_else(|| format!("Argument {}", index + 1), |p| p.name.clone()),
            original: value.clone(),
            value,
            type_name,
            choices,
            kind,
            quoted,
        }
    }
    pub(super) fn changed(&self) -> bool {
        self.value != self.original
    }
    pub(super) fn validate(&self) -> Result<(), EditError> {
        let valid = match self.kind {
            SchemaTypeRef::Int => self.value.parse::<i64>().is_ok(),
            SchemaTypeRef::Float => self.value.parse::<f64>().is_ok_and(f64::is_finite),
            SchemaTypeRef::Bool => matches!(self.value.as_str(), "true" | "false"),
            SchemaTypeRef::Array(_) => false,
            _ => self.quoted || recite_parser::is_metadata_symbol(&self.value),
        };
        if self
            .value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
            || !valid
            || (!self.choices.is_empty() && !self.choices.contains(&self.value))
        {
            return Err(EditError::InvalidRuleValue {
                argument: self.label.clone(),
                expected: self.type_name.to_lowercase(),
            });
        }
        Ok(())
    }
    pub(super) fn source(&self) -> Result<String, EditError> {
        if self.quoted {
            serde_json::to_string(&self.value).map_err(|_| EditError::Position)
        } else if matches!(self.kind, SchemaTypeRef::Float)
            && self.value.parse::<f64>().is_ok_and(f64::is_finite)
            && !self.value.contains(['.', 'e', 'E'])
        {
            Ok(format!("{}.0", self.value))
        } else {
            Ok(self.value.clone())
        }
    }
}
pub(super) fn arguments_source(arguments: &[RuleArgument]) -> Result<String, EditError> {
    Ok(arguments
        .iter()
        .map(RuleArgument::source)
        .collect::<Result<Vec<_>, _>>()?
        .join(", "))
}

pub(super) fn default_argument(
    parameter: &ParameterDefinition,
    schema: &ProjectSchema,
) -> RuleArgument {
    let initial = match parameter.type_ref {
        SchemaTypeRef::String => Argument::Value(ScalarValue::String(String::new())),
        SchemaTypeRef::Int => Argument::Value(ScalarValue::Integer(0)),
        SchemaTypeRef::Float => Argument::Value(ScalarValue::Float(0.)),
        SchemaTypeRef::Bool => Argument::Value(ScalarValue::Boolean(false)),
        _ => Argument::Identifier(String::new()),
    };
    let mut argument = RuleArgument::new(&initial, Some(parameter), Some(schema), 0);
    if let Some(first) = argument.choices.first() {
        argument.value = first.clone();
    }
    argument
}
