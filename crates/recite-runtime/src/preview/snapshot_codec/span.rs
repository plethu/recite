use recite_core::{SourcePosition, SourceSpan};
use serde::{Deserialize, Serialize};

use super::super::model::PreviewError;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PositionWire {
    line: u32,
    column: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SpanWire {
    file: String,
    start: PositionWire,
    end: Option<PositionWire>,
}

impl From<&SourcePosition> for PositionWire {
    fn from(position: &SourcePosition) -> Self {
        Self {
            line: position.line(),
            column: position.column(),
        }
    }
}

impl PositionWire {
    pub(super) fn into_position(self) -> Result<SourcePosition, PreviewError> {
        SourcePosition::new(self.line, self.column).map_err(invalid)
    }
}

impl From<&SourceSpan> for SpanWire {
    fn from(span: &SourceSpan) -> Self {
        Self {
            file: span.file.clone(),
            start: (&span.start).into(),
            end: span.end.as_ref().map(Into::into),
        }
    }
}

impl SpanWire {
    pub(super) fn into_span(self) -> Result<SourceSpan, PreviewError> {
        Ok(SourceSpan::new(
            self.file,
            self.start.into_position()?,
            self.end.map(PositionWire::into_position).transpose()?,
        ))
    }
}

fn invalid(error: impl std::fmt::Display) -> PreviewError {
    PreviewError::SnapshotDecodeFailed {
        reason: error.to_string(),
    }
}
