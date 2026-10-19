//! Rules projection owns a lightweight immutable view, never live editing history.
use crate::{CompletionSnapshot, Document, EditError, ReplyRules};

pub struct ReplyRulesRequest {
    base: CompletionSnapshot,
    passage: String,
    passages: Option<crate::projection_cache::CachedPassages>,
}
impl Document {
    pub fn reply_rules_request(&self, passage: &str) -> ReplyRulesRequest {
        ReplyRulesRequest {
            base: self.completion_snapshot(),
            passage: passage.into(),
            passages: self.projections.capture_passages(),
        }
    }
}
impl ReplyRulesRequest {
    pub fn is_current(&self, document: &Document) -> bool {
        self.base.is_current(document)
    }
    pub fn prepare(&self) -> Result<ReplyRules, EditError> {
        let mut document = self.base.projection_document();
        if let Some(passages) = &self.passages {
            document.projections = passages.restore();
        }
        document.reply_rules(&self.passage)
    }
}
