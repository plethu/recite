//! Writes the checked-in seed corpora: sources, compiled assets and saves
//! taken along runs of deterministic generated programs and the fixed asset.
//!
//! Run from the repository root:
//! `cargo run --manifest-path fuzz/Cargo.toml --bin seed-corpus`.

use std::fs;
use std::path::Path;

use proptest::strategy::{Strategy, ValueTree};
use proptest::test_runner::TestRunner;
use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs_with_schema};
use recite_core::compiled::{CompiledAssetId, CompilerVersion, SourceMapId};
use recite_runtime::{snapshot::encode_session_messagepack, start_scene};

const PROGRAMS: usize = 12;

fn main() {
    let root = Path::new("fuzz/corpus");
    for target in ["compiled_asset", "session_snapshot", "source"] {
        let directory = root.join(target);
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap_or_else(|error| panic!("{error}"));
    }
    let write = |target: &str, name: String, bytes: &[u8]| {
        fs::write(root.join(target).join(name), bytes).unwrap_or_else(|error| panic!("{error}"));
    };

    let mut programs = vec![vec![(
        "dialogue/main.recite".to_owned(),
        recite_fuzz::FIXED_SOURCE.to_owned(),
    )]];
    let mut runner = TestRunner::deterministic();
    for _ in 0..PROGRAMS {
        let tree = recite_testkit::program()
            .new_tree(&mut runner)
            .unwrap_or_else(|error| panic!("{error}"));
        programs.push(tree.current().sources());
    }

    for (index, sources) in programs.iter().enumerate() {
        for (file, (_, source)) in sources.iter().enumerate() {
            write(
                "source",
                format!("program-{index:02}-{file}.recite"),
                source.as_bytes(),
            );
        }
        let Some(output) = compile_output(sources) else {
            panic!("seed program {index} compiles");
        };
        write(
            "compiled_asset",
            format!("program-{index:02}.recitec"),
            &output.messagepack,
        );
        // Saves at every boundary of one run, against this program's asset.
        let asset = output.dialogue;
        let Ok(mut session) = start_scene(&asset, None) else {
            continue;
        };
        let mut step = 0;
        recite_fuzz::play_with(&asset, &mut session, 24, &mut |session| {
            if let Ok(bytes) = encode_session_messagepack(&asset, session) {
                write(
                    "session_snapshot",
                    format!("program-{index:02}-save-{step:02}.bin"),
                    &bytes,
                );
            }
            step += 1;
        });
    }
}

fn compile_output(
    sources: &[(String, String)],
) -> Option<recite_compiler::compile::CompiledAssetOutput> {
    let schema = recite_fuzz::schema();
    compile_inputs_with_schema(
        sources
            .iter()
            .map(|(path, source)| CompileInput::new(path.as_str(), source.as_str())),
        CompileOptions::new(
            CompilerVersion::new("0.0.1").ok()?,
            CompiledAssetId::new(recite_testkit::ASSET_ID).ok()?,
            SourceMapId::new("generated.recitec.map").ok()?,
            schema.canonical_fingerprint(),
        ),
        schema,
    )
    .ok()?
    .asset
}
