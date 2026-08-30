use super::super::types::{ClauseKind, CompletionSite, CompletionSiteKind};
use super::Site;
use recite_core::SourceSpan;

impl Site {
    pub(crate) fn completion_site(&self) -> CompletionSite {
        match self {
            Self::Blocks { target, token } => CompletionSite::new(
                CompletionSiteKind::Block,
                token.clone(),
                Some(target.clone()),
            ),
            Self::Speakers(span) => {
                CompletionSite::new(CompletionSiteKind::Speaker, span.clone(), None)
            }
            Self::MetadataKey { span, .. } => {
                CompletionSite::new(CompletionSiteKind::MetadataKey, span.clone(), None)
            }
            Self::MetadataValue { token, .. } => {
                CompletionSite::new(CompletionSiteKind::MetadataValue, token.clone(), None)
            }
            Self::Conditions { span, .. } => {
                CompletionSite::new(CompletionSiteKind::Condition, span.clone(), None)
            }
            Self::Effects(span) => {
                CompletionSite::new(CompletionSiteKind::Effect, span.clone(), None)
            }
            Self::AvailabilityReasons { token, .. } => {
                CompletionSite::new(CompletionSiteKind::AvailabilityReason, token.clone(), None)
            }
        }
    }

    pub(crate) fn clause(&self) -> Option<(ClauseKind, SourceSpan)> {
        match self {
            Self::Conditions {
                clause: Some(clause),
                ..
            } => Some(clause.clone()),
            Self::Blocks { .. }
            | Self::Speakers(_)
            | Self::MetadataKey { .. }
            | Self::MetadataValue { .. }
            | Self::Conditions { clause: None, .. }
            | Self::Effects(_)
            | Self::AvailabilityReasons { .. } => None,
        }
    }
}
