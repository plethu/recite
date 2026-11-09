//! Which reading surface the current view shows, derived once; Workbench remains the selection
//! authority.
pub use crate::{Writer, reading::Row, rules::ReplySession};
use recite_writer_model::View;

pub enum Surface<'a> {
    Manuscript {
        passage: &'a str,
    },
    Reply(&'a ReplySession),
    Beat {
        id: &'a str,
        rows: &'a [Row],
    },
    Empty,
    SourceRepair {
        error: Option<&'a str>,
        import: bool,
    },
}
impl<'a> Surface<'a> {
    pub fn resolve(state: &'a Writer) -> Self {
        let model = state.session.model();
        let error = state
            .reading
            .script()
            .err()
            .or(state.reading.error.as_deref());
        match model.view() {
            View::Passage(id) => Self::Manuscript { passage: id },
            View::Block(id) => match error {
                Some(error) => Self::SourceRepair {
                    error: Some(error),
                    import: false,
                },
                None => Self::Beat {
                    id,
                    rows: &state.reading.rows,
                },
            },
            View::Source => {
                if let Some(reply) = &state.rules
                    && reply.is_open_for(model)
                {
                    Self::Reply(reply)
                } else if error.is_none()
                    && !model.has_draft()
                    && state
                        .reading
                        .script()
                        .is_ok_and(<[recite_writer_model::ScriptBlock]>::is_empty)
                {
                    Self::Empty
                } else {
                    Self::SourceRepair {
                        error,
                        import: true,
                    }
                }
            }
        }
    }
}
