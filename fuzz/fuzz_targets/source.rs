//! Untrusted source text: parsing, lowering and validated compilation never
//! panic, and a compiled asset plays without panicking. A NUL byte splits the
//! input into `dialogue/main.recite` and `dialogue/side.recite`, so
//! cross-file diverts and choices compile.

#![no_main]

use libfuzzer_sys::fuzz_target;
use recite_runtime::start_scene;

const PATHS: [&str; 2] = ["dialogue/main.recite", "dialogue/side.recite"];

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let sources: Vec<(String, String)> = PATHS
        .iter()
        .zip(text.splitn(PATHS.len(), '\0'))
        .map(|(path, source)| ((*path).to_owned(), source.to_owned()))
        .collect();
    for (path, source) in &sources {
        let _ = recite_parser::lower_source_file(path, source);
        let _ = recite_parser::parse(path.as_str(), source.as_str()).lower_source_file();
    }
    if let Some(asset) = recite_fuzz::compile(&sources)
        && let Ok(mut session) = start_scene(&asset, None)
    {
        recite_fuzz::play(&asset, &mut session, 32);
    }
});
