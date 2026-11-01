//! Adopt analysis already produced by a shared structural operation; never replay compilation.
use super::*;

impl SourceEditRequest {
    pub(crate) fn prepare_candidate(
        self,
        candidate: Document,
    ) -> Result<PreparedSourceEdit, EditError> {
        if self.base.key != candidate.key || !Arc::ptr_eq(&self.base.context, &candidate.context) {
            return Err(EditError::Stale);
        }
        let changed = candidate.source != self.base.source;
        let (source, kernel, projections, history, version) = if changed {
            let version = self
                .base
                .version
                .as_i64()
                .checked_add(1)
                .map(DocumentVersion::new)
                .ok_or(EditError::RevisionExhausted)?;
            if candidate.version != version {
                return Err(EditError::Stale);
            }
            let history =
                HistoryEdit::Record(Change::between(&self.base.source, &candidate.source));
            (
                candidate.source,
                Some(candidate.kernel),
                Some(candidate.projections),
                history,
                version,
            )
        } else {
            // A no-op keeps the original analysis/revision and both history directions.
            (
                self.base.source.clone(),
                None,
                None,
                HistoryEdit::None,
                self.base.version,
            )
        };
        Ok(PreparedSourceEdit {
            request: self,
            source,
            kernel,
            projections,
            history,
            version,
            retired_projections: None,
            retired_history: Vec::new(),
            accepted: false,
        })
    }
}

#[cfg(test)]
mod tests;
