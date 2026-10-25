//! Product diagnostics use the requested filter without an implicit unbounded trace file.
use tracing_subscriber::EnvFilter;

pub(super) fn init() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"));
    // Respect an embedding host's subscriber if one is already installed.
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}
