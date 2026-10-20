//! Shared comparison values and bounded source alignment.
mod source;
#[derive(Clone, Debug, PartialEq)]
pub struct ComparisonRow {
    pub caption: String,
    pub before: Option<String>,
    pub after: Option<String>,
}
