fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    // Options consume one value; help must not intercept literal editor arguments.
    if args
        .iter()
        .step_by(2)
        .any(|arg| arg == "--help" || arg == "-h")
    {
        println!("{}", recite_writer_xilem::HELP);
        return;
    }
    let result = recite_writer_xilem::Startup::parse(args)
        .and_then(|startup| {
            recite_writer_xilem::Writer::open(startup).map_err(|error| error.to_string())
        })
        .and_then(|writer| recite_writer_xilem::run(writer).map_err(|error| error.to_string()));
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(2);
    }
}
