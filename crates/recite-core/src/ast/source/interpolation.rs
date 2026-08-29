/// A named source-level interpolation binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InterpolationBinding {
    pub name: String,
    pub value: String,
    pub value_type: InterpolationType,
}

impl InterpolationBinding {
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
        value_type: InterpolationType,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            value_type,
        }
    }
}

/// Scalar types accepted by interpolation bindings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InterpolationType {
    String,
    Integer,
    Float,
    Boolean,
}
