fn main() {
    let result =
        recite_writer::LaunchOptions::parse(std::env::args_os().skip(1)).and_then(|options| {
            match options.intent {
                recite_writer_session::startup::Startup::Help => {
                    println!("{}", recite_writer::HELP);
                    Ok(())
                }
                recite_writer_session::startup::Startup::Version => {
                    println!("recite-writer {}", env!("CARGO_PKG_VERSION"));
                    Ok(())
                }
                _ => recite_writer::run_launch(options),
            }
        });
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
