use recite_build::{ProjectBuildEngine, ProjectBuildRequest};
use recite_compiler::{
    authoring::{
        BuildControl, BuildEngine, BuildFailure, BuildFailureReason, BuildGeneration,
        BuildInputKind, SnapshotGeneration,
    },
    compile::{
        CompileInput, CompileOptions, compile_inputs_to_messagepack,
        compile_inputs_to_messagepack_with_schema,
    },
};
use recite_core::compiled::{
    CompiledAssetId, CompilerVersion, SourceMapId, decode_compiled_dialogue_messagepack,
};

mod support;
use support::project;

#[test]
fn shared_compilation_matches_independent_outputs_and_captured_inputs()
-> Result<(), Box<dyn std::error::Error>> {
    for with_schema in [false, true] {
        for count in [1, 3] {
            let (directory, request) = project(with_schema, count)?;
            let mut engine = ProjectBuildEngine::new(&request);
            let control = BuildControl::new();
            let candidates = engine.build(request.build_request(), &control)?;
            assert_eq!(candidates.len(), count);
            let inputs: Vec<_> = request
                .build_request()
                .inputs()
                .iter()
                .filter(|input| input.kind() == &BuildInputKind::Source)
                .map(|input| {
                    CompileInput::new(input.key().as_str(), input.content().expect("source"))
                })
                .collect();
            let mut fingerprints = Vec::new();
            for (candidate, target) in candidates.iter().zip(request.targets()) {
                assert_eq!(candidate.target(), target.target());
                let options = CompileOptions::new(
                    CompilerVersion::new(env!("CARGO_PKG_VERSION"))?,
                    CompiledAssetId::new(target.asset_id())?,
                    SourceMapId::new(format!("{}.map", target.asset_id()))?,
                    request.build_request().fingerprints().schema().clone(),
                );
                let independent = match request.schema() {
                    Some(schema) => {
                        compile_inputs_to_messagepack_with_schema(inputs.clone(), options, schema)?
                    }
                    None => compile_inputs_to_messagepack(inputs.clone(), options)?,
                };
                assert!(independent.is_ok(), "{:?}", independent.diagnostics);
                assert_eq!(Some(candidate.bytes()), independent.messagepack.as_deref());
                let asset = decode_compiled_dialogue_messagepack(candidate.bytes())?;
                assert_eq!(asset.header.asset_id.as_str(), target.asset_id());
                fingerprints.push(
                    asset
                        .content_fingerprint()
                        .map_err(|error| error.to_string())?
                        .clone(),
                );
            }
            assert!(fingerprints.windows(2).all(|pair| pair[0] != pair[1]));
            // A later file edit cannot change this immutable captured request.
            std::fs::write(directory.path().join("dialogue/other.recite"), "malformed")?;
            assert_eq!(engine.build(request.build_request(), &control)?, candidates);
            let mut selected = request.clone();
            assert!(selected.select_asset("compiled/0.recitec"));
            let selected_candidates =
                ProjectBuildEngine::new(&selected).build(selected.build_request(), &control)?;
            assert_eq!(selected_candidates, candidates[..1]);
        }
    }
    Ok(())
}

#[test]
fn empty_and_cancelled_builds_produce_no_candidates_and_foreign_requests_fail()
-> Result<(), Box<dyn std::error::Error>> {
    for count in [0, 3] {
        let (directory, request) = project(false, count)?;
        let mut engine = ProjectBuildEngine::new(&request);
        let control = BuildControl::new();
        if count == 0 {
            assert!(engine.build(request.build_request(), &control)?.is_empty());
        }
        control.cancel();
        assert!(engine.build(request.build_request(), &control)?.is_empty());
        let foreign = ProjectBuildRequest::prepare_with_generations(
            directory.path(),
            BuildGeneration::new(1),
            SnapshotGeneration::initial(),
        )?
        .into_request()
        .ok_or("missing request")?;
        assert!(matches!(
            engine.build(foreign.build_request(), &control),
            Err(BuildFailure::Engine {
                reason: BuildFailureReason::Host
            })
        ));
    }
    Ok(())
}
