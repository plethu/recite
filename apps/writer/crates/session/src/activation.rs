//! Native writer activation. Linux keeps one owner for the file-backed writer.
#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::IncomingRoute;
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
        Self::claim_launch(&crate::startup::LaunchOptions {
            intent: crate::startup::Startup::Writer {
                project: project.map(std::path::Path::to_path_buf),
                route: route.map(str::to_owned),
            },
            document: None,
            editor: None,
        })
    }
    pub fn claim_launch(options: &crate::startup::LaunchOptions) -> Result<Self, String> {
        if !matches!(options.intent, crate::startup::Startup::Writer { .. }) {
            return Err("Only a project or welcome invocation can claim the writer window".into());
        }
        #[cfg(target_os = "linux")]
        {
            linux::claim(options)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = options;
            Ok(Self::Owner(ActivationHost))
        }
    }
}
