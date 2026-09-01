//! Versioned capability values shared by local Recite tools.

mod name;
mod report;

pub use name::{CapabilityId, CapabilityName, CapabilityNameError};
pub use report::{
    CAPABILITY_REPORT_VERSION, Capability, CapabilityReport, CapabilityReportError,
    CapabilityStatus,
};
