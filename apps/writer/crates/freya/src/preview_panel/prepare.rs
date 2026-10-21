//! Widget capture stays in the native frontend; trial policy is shared.
use crate::editing::Writer;
pub(super) use recite_writer_session::preview::Inputs;
pub(super) fn capture(writer: Writer, configured: bool) -> Inputs {
    let locale = if configured {
        writer.trial.locale.peek().clone()
    } else {
        String::new()
    };
    let include_drafts = configured && *writer.trial.include_drafts.peek();
    Inputs {
        locale,
        variant: if configured {
            writer.trial.variant.peek().clone()
        } else {
            String::new()
        },
        include_drafts,
        values: if configured {
            Some(writer.trial.values.peek().clone())
        } else {
            None
        },
    }
}
