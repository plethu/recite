//! Captured preview inputs and their freshness, independent of native widgets.
mod provenance;
pub use provenance::{capture_provenance, catalogue_stale};

mod inputs;
pub use inputs::{Inputs, parse_value};

mod request;
pub use request::Request;
