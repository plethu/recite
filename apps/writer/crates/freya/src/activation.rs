//! Native writer activation. Linux keeps one owner for the file-backed writer.
#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub(crate) use linux::IncomingRoute;
#[cfg(target_os = "linux")]
pub use linux::{ActivationHost, ActivationInbox};

#[cfg(not(target_os = "linux"))]
#[derive(Clone)]
pub struct ActivationInbox;

#[cfg(not(target_os = "linux"))]
pub struct ActivationHost;

#[cfg(not(target_os = "linux"))]
impl ActivationHost {
    pub fn inbox(&self) -> ActivationInbox {
        ActivationInbox
    }
}

/// Claim the writer window, or deliver this invocation to its owner.
pub enum Activation {
    Owner(ActivationHost),
    Forwarded,
}

impl Activation {
    pub fn claim(project: Option<&std::path::Path>, route: Option<&str>) -> Result<Self, String> {
        #[cfg(target_os = "linux")]
        {
            linux::claim(project, route)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (project, route);
            Ok(Self::Owner(ActivationHost))
        }
    }
}
