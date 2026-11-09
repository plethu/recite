//! One pending editor choice; opening another replaces the previous request.
pub use crate::{Writer, routes, rules};

pub enum Request {
    Rules(rules::selection::Request),
    Destination(routes::Request),
    Rename(crate::workspaces::BeatChoice),
}

impl Request {
    pub fn is_current(&self, writer: &Writer) -> bool {
        match self {
            Self::Rules(request) => request.is_current(writer),
            Self::Destination(request) => request.is_current(writer),
            Self::Rename(request) => request.is_current(writer),
        }
    }
}
