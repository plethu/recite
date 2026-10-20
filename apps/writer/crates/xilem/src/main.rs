fn main() {
    let result = recite_writer_xilem::LaunchOptions::parse(std::env::args_os().skip(1)).and_then(
        |options| match options.intent {
            recite_writer_session::startup::Startup::Help => {
                println!("{}", recite_writer_xilem::HELP);
                Ok(())
            }
            recite_writer_session::startup::Startup::Version => {
                println!("recite-writer {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
            _ => recite_writer_xilem::run_launch(options),
        },
    );
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
