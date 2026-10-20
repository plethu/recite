use super::*;
impl Session {
    pub fn accept_declaration(
        &mut self,
        receipt: &mut DeclarationReceipt,
    ) -> Result<DeclarationFinish, FileError> {
        let Activity::PreparingDeclaration(token) = &self.activity else {
            return Err(FileError::DeclarationChanged);
        };
        if !token.matches(&receipt.token) {
            return Err(FileError::DeclarationChanged);
        }
        let prepared = receipt.result.take().ok_or(FileError::DeclarationChanged)?;
        let cancelled = token.cancelled();
        let current = Arc::ptr_eq(&self.schema_output, &receipt.schema_output)
            && receipt.inputs.matches(self.source.as_ref());
        if cancelled || !current {
            // Rejected prepared leases retire with the receipt, away from the event loop.
            receipt.result = Some(prepared);
            self.activity = Activity::Idle;
            return Err(if cancelled {
                FileError::DeclarationActionCancelled
            } else {
                FileError::DeclarationChanged
            });
        }
        let mut work = DeclarationFinish {
            token: token.clone(),
            payload: FinishPayload::Unchanged,
        };
        match prepared {
            Ok(PreparedDeclaration::Bound(source)) => {
                work.payload = FinishPayload::Bound(self.source.replace(*source))
            }
            Ok(PreparedDeclaration::Replaced(mut state)) => {
                let source = self.source.as_mut().ok_or(FileError::SchemaOwnership)?;
                // Queue the proposed recovery before publishing any buffer mutation.
                let ticket = match source.begin_replacement_recovery(&state.baseline, &state.draft)
                {
                    Ok(ticket) => ticket,
                    Err(error) => {
                        receipt.result = Some(Ok(PreparedDeclaration::Replaced(state)));
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
                work.payload = FinishPayload::Replaced { ticket, state };
            }
            Ok(PreparedDeclaration::Unchanged) => (),
            Ok(PreparedDeclaration::Output(output)) => {
                work.payload =
                    FinishPayload::Output(std::mem::replace(&mut self.schema_output, output))
            }
            Ok(PreparedDeclaration::Registration(registration)) => {
                work.payload = FinishPayload::Registration(std::mem::replace(
                    &mut self.registration,
                    registration,
                ))
            }
            Err(error) => {
                self.activity = Activity::Idle;
                return Err(error);
            }
        }
        self.activity = Activity::FinishingDeclaration(work.token.clone());
        Ok(work)
    }
    pub fn accept_declaration_finish(
        &mut self,
        receipt: &mut DeclarationFinished,
    ) -> Result<(), FileError> {
        let Activity::FinishingDeclaration(token) = &self.activity else {
            return Err(FileError::DeclarationChanged);
        };
        if !token.matches(&receipt.token) {
            return Err(FileError::DeclarationChanged);
        }
        let result = receipt.result.take().ok_or(FileError::DeclarationChanged)?;
        self.activity = Activity::Idle;
        result
    }
}
