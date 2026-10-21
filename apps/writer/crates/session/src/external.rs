//! Filesystem notifications and preferred-application process lifetimes.
mod handoff;
mod watch;
pub use handoff::Handoff;
pub use watch::{Affected, Changes, Subscription, Watch};
