//! One pending editor choice; opening another replaces the previous request.
pub use crate::{Writer, routes, rules};
use recite_writer_model::RouteOwner;

/// The label a choice hangs from: a line's speaker, or a reply's or beat's destination. While its
/// slip is open, the label's control reports itself expanded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opener {
    Speaker(String),
    Destination(RouteOwner),
}

pub enum Request {
    Rules(rules::selection::Request),
    Destination(routes::Request),
    Speaker(routes::SpeakerRequest),
    Rename(crate::workspaces::BeatChoice),
}

impl Request {
    pub fn is_current(&self, writer: &Writer) -> bool {
        match self {
            Self::Rules(request) => request.is_current(writer),
            Self::Destination(request) => request.is_current(writer),
            Self::Speaker(request) => request.is_current(writer),
            Self::Rename(request) => request.is_current(writer),
        }
    }
    /// The label this choice hangs from, for a speaker or destination.
    pub fn opener(&self) -> Option<Opener> {
        match self {
            Self::Destination(request) => Some(Opener::Destination(request.owner.clone())),
            Self::Speaker(request) => Some(Opener::Speaker(request.passage.clone())),
            Self::Rules(_) | Self::Rename(_) => None,
        }
    }
    /// The value the choice holds now, which its slip highlights when it opens.
    pub fn current(&self) -> Option<&str> {
        match self {
            Self::Destination(request) => request.current.as_deref(),
            Self::Speaker(request) => request.current.as_deref(),
            Self::Rules(_) | Self::Rename(_) => None,
        }
    }
}
