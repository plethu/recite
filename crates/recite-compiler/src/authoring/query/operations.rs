use recite_core::{DocumentKey, SourcePosition};

use super::super::snapshot::AuthoringSnapshot;
use super::context;
use super::schema;
use super::types::{CompletionCandidate, CompletionSite, QueryResult};

impl AuthoringSnapshot {
    /// Returns candidates for the syntax site under the cursor.
    #[must_use]
    pub fn complete(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
    ) -> QueryResult<Vec<CompletionCandidate>> {
        let Some(document) = self.document(key) else {
            return QueryResult::NoMatch;
        };
        let Some(site) = context::at(key, document.source_text(), position) else {
            return QueryResult::NoMatch;
        };
        schema::complete_site(self, key, document, site)
    }

    /// Returns the compiler-owned syntax site used by source completion.
    /// Hosts use this typed classification when they need to project a
    /// completion result into a broader protocol response.
    #[must_use]
    pub fn completion_site(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
    ) -> Option<CompletionSite> {
        let document = self.document(key)?;
        context::at(key, document.source_text(), position).map(|site| site.completion_site())
    }

    /// Enumerates one schema projector's typed semantic names for later adapters.
    #[must_use]
    pub fn projection_candidates(&self, projector: &str) -> QueryResult<Vec<CompletionCandidate>> {
        let Some(schema) = &self.schema else {
            return QueryResult::unavailable(super::types::QueryUnavailableReason::Incomplete(
                super::types::QueryClass::Schema,
            ));
        };
        let Ok(position) = SourcePosition::new(1, 1) else {
            return QueryResult::unavailable(super::types::QueryUnavailableReason::Unsupported);
        };
        let span = recite_core::SourceSpan::point("<schema>", position);
        if !schema.presentation_projectors.contains_key(projector) {
            return QueryResult::NoMatch;
        }
        QueryResult::Ready(schema::explicit_projection_candidates(
            schema, projector, &span,
        ))
    }
}
