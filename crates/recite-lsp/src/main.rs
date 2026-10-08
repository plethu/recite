fn main() {
    if let Err(error) = configure_trace() {
        eprintln!("recite-lsp: cannot configure native trace: {error}");
        std::process::exit(1);
    }
    if let Err(error) = recite_lsp::run_stdio() {
        eprintln!("recite-lsp: {error}");
        std::process::exit(1);
    }
}

fn configure_trace() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(directory) = std::env::var_os("RECITE_LSP_TRACE_DIR") else {
        return Ok(());
    };
    let path = std::path::PathBuf::from(directory).join(format!("{}.jsonl", std::process::id()));
    let file = std::fs::File::create_new(path)?;
    tracing_subscriber::fmt()
        .json()
        .with_timer(tracing_subscriber::fmt::time::Uptime::default())
        .with_thread_ids(true)
        .with_writer(std::sync::Mutex::new(file))
        .with_max_level(tracing::Level::TRACE)
        .try_init()?;
    Ok(())
}
