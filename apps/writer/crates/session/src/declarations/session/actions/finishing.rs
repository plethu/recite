//! Finish the accepted action and retire displaced owners away from the UI.
use super::*;
/// Adoption stays busy through recovery and retirement, including actions with no source change.
pub struct DeclarationFinish {
    pub(super) token: ActivityToken,
    pub(super) payload: FinishPayload,
}
pub(super) enum FinishPayload {
    Unchanged,
    Bound(Option<Source>),
    Replaced {
        ticket: Durability<DraftSnapshot>,
        state: Replacement,
    },
    Output(Arc<SchemaOutput>),
    Registration(Result<Option<recite_config::ProducerRegistration>, String>),
}
impl FinishPayload {
    fn finish(self) -> Result<(), FileError> {
        match self {
            Self::Unchanged => Ok(()),
            Self::Bound(source) => {
                drop(source);
                Ok(())
            }
            Self::Replaced { ticket, state } => {
                let result = ticket.wait();
                drop(state);
                result
            }
            Self::Output(output) => {
                drop(output);
                Ok(())
            }
            Self::Registration(registration) => {
                drop(registration);
                Ok(())
            }
        }
    }
}
pub struct DeclarationFinished {
    pub(super) token: ActivityToken,
    pub(super) result: Option<Result<(), FileError>>,
}

impl DeclarationFinish {
    pub fn execute(self) -> DeclarationFinished {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.payload.finish()))
                .unwrap_or_else(|_| {
                    Err(std::io::Error::other(
                        "Declaration recovery or cleanup stopped unexpectedly.",
                    )
                    .into())
                });
        DeclarationFinished {
            token: self.token,
            result: Some(result),
        }
    }
}
