//! Saved build preparation, compilation and encoding; discovery is untimed.
use recite_build::{ProjectBuildEngine, ProjectBuildRequest};
use recite_compiler::authoring::{
    BuildCandidate, BuildControl, BuildEngine, BuildGeneration, SnapshotGeneration,
};
use recite_config::ProjectDiscoveryReport;

pub(super) struct Input {
    pub discovery: Option<ProjectDiscoveryReport>,
    files: usize,
    _directory: tempfile::TempDir,
}
pub(super) fn prepare(files: usize) -> Input {
    let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("project input: {error}"));
    std::fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n[[scenes]]\nid = \"probe\"\nasset = \"build/probe.recitec\"\nblock = \"shard_0\"\nparticipants = [\"narrator\"]\n",
    )
    .unwrap_or_else(|error| panic!("project input: {error}"));
    for file in 0..files {
        let mut source = format!(
            ":: shard_{file}{}\n",
            if file == 0 { " default" } else { "" }
        );
        for line in 0..25 {
            source.push_str(&format!(
                "> line_{file}_{line}@{:020x}\n  A courier greets the café 世界.\n",
                file * 25 + line + 1
            ));
        }
        source.push_str("-> END\n");
        std::fs::write(
            directory.path().join(format!("shard_{file}.recite")),
            source,
        )
        .unwrap_or_else(|error| panic!("project input: {error}"));
    }
    let discovery = recite_config::discover_project(directory.path())
        .unwrap_or_else(|error| panic!("project input: {error}"));
    assert_eq!(discovery.documents().len(), files);
    Input {
        discovery: Some(discovery),
        files,
        _directory: directory,
    }
}
pub(super) struct Built {
    input: Input,
    request: ProjectBuildRequest,
    _engine: ProjectBuildEngine,
    candidates: Vec<BuildCandidate>,
}
pub(super) fn run(mut input: Input) -> Built {
    let discovery = input
        .discovery
        .take()
        .unwrap_or_else(|| panic!("discovery consumed"));
    let result = recite_build::prepare_discovered(
        discovery,
        BuildGeneration::initial(),
        SnapshotGeneration::initial(),
    )
    .unwrap_or_else(|e| panic!("prepare: {e}"));
    assert!(
        result.is_ready(),
        "build diagnostics: {:?}",
        result.diagnostics()
    );
    let request = result
        .into_request()
        .unwrap_or_else(|| panic!("valid project rejected"));
    let mut engine = ProjectBuildEngine::new(&request);
    let candidates = engine
        .build(request.build_request(), &BuildControl::new())
        .unwrap_or_else(|e| panic!("compile: {e}"));
    Built {
        input,
        request,
        _engine: engine,
        candidates,
    }
}
pub(super) fn validate(built: Built) {
    assert!(built.request.diagnostics().is_empty());
    assert_eq!(
        built.request.build_request().inputs().len(),
        built.input.files + 1
    );
    let [candidate] = built.candidates.as_slice() else {
        panic!("one asset");
    };
    let dialogue = recite_core::compiled::decode_compiled_dialogue_messagepack(candidate.bytes())
        .unwrap_or_else(|e| panic!("asset: {e}"));
    assert_eq!(dialogue.lines.len(), built.input.files * 25);
    assert_eq!(dialogue.blocks.len(), built.input.files);
}
