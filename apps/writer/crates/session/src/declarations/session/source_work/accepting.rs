use super::*;
impl Session {
    pub fn accept_source(
        &mut self,
        receipt: &mut SourceReceipt,
    ) -> Result<SourceRecovery, FileError> {
        let Activity::ChangingSource(token) = &self.activity else {
            return Err(FileError::DeclarationSourceChanged);
        };
        if !token.matches(&receipt.token) {
            return Err(FileError::DeclarationSourceChanged);
        }
        let prepared = receipt
            .result
            .take()
            .ok_or(FileError::DeclarationSourceChanged)?;
        let cancelled = token.cancelled();
        let current = Arc::ptr_eq(&self.schema_output, &receipt.schema_output)
            && match (&receipt.source, &self.source) {
                (Some(stamp), Some(source)) => stamp.matches(source),
                (None, None) => true,
                _ => false,
            };
        if cancelled || !current {
            // Rejected prepared leases retire with the receipt, away from the event loop.
            receipt.result = Some(prepared);
            self.activity = Activity::Idle;
            return Err(if cancelled {
                FileError::DeclarationActionCancelled
            } else {
                FileError::DeclarationSourceChanged
            });
        }
        let mut work = SourceRecovery {
            token: token.clone(),
            ticket: None,
            retired_source: None,
            retired_state: None,
        };
        match prepared {
            Ok(PreparedSource::Bound(source)) => work.retired_source = self.source.replace(*source),
            Ok(PreparedSource::Replaced(mut state)) => {
                let source = self.source.as_mut().ok_or(FileError::SchemaOwnership)?;
                // Queue the proposed recovery before publishing any buffer mutation.
                let ticket = match source.begin_replacement_recovery(&state.baseline, &state.draft)
                {
                    Ok(ticket) => ticket,
                    Err(error) => {
                        receipt.result = Some(Ok(PreparedSource::Replaced(state)));
                        self.activity = Activity::Idle;
                        return Err(error);
                    }
                };
                std::mem::swap(&mut source.baseline, &mut state.baseline);
                std::mem::swap(&mut source.draft, &mut state.draft);
                source.revision.advance();
                if let Some(output) = &mut state.output {
                    std::mem::swap(&mut self.schema_output, output);
                }
                work.ticket = Some(ticket);
                work.retired_state = Some(state);
            }
            Ok(PreparedSource::Unchanged) => (),
            Err(error) => {
                self.activity = Activity::Idle;
                return Err(error);
            }
        }
        self.activity = Activity::RecoveringSource(work.token.clone());
        Ok(work)
    }
    pub fn accept_source_recovery(
        &mut self,
        receipt: &mut SourceRecovered,
    ) -> Result<(), FileError> {
        let Activity::RecoveringSource(token) = &self.activity else {
            return Err(FileError::DeclarationSourceChanged);
        };
        if !token.matches(&receipt.token) {
            return Err(FileError::DeclarationSourceChanged);
        }
        let result = receipt
            .result
            .take()
            .ok_or(FileError::DeclarationSourceChanged)?;
        self.activity = Activity::Idle;
        result
    }
}
