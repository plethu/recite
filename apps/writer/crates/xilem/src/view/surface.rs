//! Borrowed presentation derived once; Workbench remains the selection authority.
use super::*;
use crate::{reading::Row, rules::ReplySession};
use recite_writer_model::{PassageKind, RouteOwner};

pub(super) enum Surface<'a> {
    Manuscript {
        passage: &'a str,
        route: Option<RouteOwner>,
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
            View::Passage(id) => {
                let route = model.selected().ok().flatten().and_then(|passage| {
                    matches!(passage.kind, PassageKind::Choice { .. })
                        .then(|| RouteOwner::Reply(id.clone()))
                });
                Self::Manuscript { passage: id, route }
            }
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

    pub fn title(&self) -> &'static str {
        match self {
            Self::Manuscript { .. } => "Manuscript",
            Self::Reply(_) => "Reply",
            Self::SourceRepair { .. } => "Source attention",
            Self::Beat { .. } | Self::Empty => "Beat reading",
        }
    }
}
