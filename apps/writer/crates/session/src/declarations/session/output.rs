//! The observed output bytes and their validation state form one freshness identity.
use super::{Arc, ProjectSchema};
use crate::declarations::producer::Generated;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchemaOutputStatus {
    Ready,
    Damaged,
}

pub(super) enum SchemaOutput {
    Ready(Generated),
    Damaged {
        baseline: String,
        last_valid: Arc<ProjectSchema>,
    },
}
impl SchemaOutput {
    pub(super) fn baseline(&self) -> &str {
        match self {
            Self::Ready(generated) => &generated.text,
            Self::Damaged { baseline, .. } => baseline,
        }
    }
    pub(super) fn last_valid_schema(&self) -> &Arc<ProjectSchema> {
        match self {
            Self::Ready(generated) => &generated.schema,
            Self::Damaged { last_valid, .. } => last_valid,
        }
    }
    pub(super) fn status(&self) -> SchemaOutputStatus {
        match self {
            Self::Ready(_) => SchemaOutputStatus::Ready,
            Self::Damaged { .. } => SchemaOutputStatus::Damaged,
        }
    }
}
