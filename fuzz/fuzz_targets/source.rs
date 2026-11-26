//! Untrusted source text: parsing, lowering and validated compilation never
//! panic, and a compiled asset plays without panicking.

#![no_main]

use libfuzzer_sys::fuzz_target;
use recite_runtime::start_scene;

fuzz_target!(|data: &[u8]| {
    let Ok(source) = std::str::from_utf8(data) else {
        return;
    };
    let lowered = recite_parser::lower_source_file("dialogue/main.recite", source);
    let _ = recite_parser::parse("dialogue/main.recite", source).lower_source_file();
    drop(lowered);
    let sources = [("dialogue/main.recite".to_owned(), source.to_owned())];
    if let Some(asset) = recite_fuzz::compile(&sources)
        && let Ok(mut session) = start_scene(&asset, None)
    {
        recite_fuzz::play(&asset, &mut session, 32);
    }
});
