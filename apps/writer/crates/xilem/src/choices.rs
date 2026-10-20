//! One pending editor choice; opening another replaces the previous request.
use crate::{Writer, routes, rules};

pub(crate) enum Request {
    Rules(rules::selection::Request),
    Destination(routes::Request),
}

impl Request {
    pub fn is_current(&self, writer: &Writer) -> bool {
        match self {
            Self::Rules(request) => request.is_current(writer),
            Self::Destination(request) => request.is_current(writer),
        }
    }
}
