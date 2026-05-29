fn main() {
    if let Err(error) = recite_lsp::run_stdio() {
        eprintln!("recite-lsp: {error}");
        std::process::exit(1);
    }
}
