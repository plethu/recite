use std::fmt::Display;
use std::fs;
use std::path::Path;

use recite_cli::watch::{ProjectBuildEngine, ProjectBuildPreparation, ProjectBuildRequest};
use recite_compiler::{
    BuildControl, BuildEngine, BuildFailure, BuildFailureReason, BuildGeneration,
    BuildPreparedHandle, BuildPublisher, BuildResultFailure, PreparedPublishIdentity,
    PublishAbortReason, PublishFailure, PublishOutcome, SnapshotGeneration,
};
use recite_core::decode_compiled_dialogue_messagepack;
use tempfile::TempDir;

fn require<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        require(fs::create_dir_all(parent), "parent directory");
    }
    require(fs::write(path, contents), "write file");
}

fn manifest(assets: &str) -> String {
    format!("format_version = 1\n\n[discovery]\nsource_roots = [\"dialogue\"]\n\n{assets}")
}

fn source() -> &'static str {
    ":: start default speaker=hazel\n> intro@11111111111111111111\n  Hello.\n-> END\n"
}

fn ready(root: &Path) -> ProjectBuildRequest {
    match require(ProjectBuildRequest::prepare(root), "preparation") {
        ProjectBuildPreparation::Ready(request) => *request,
        ProjectBuildPreparation::Rejected { diagnostics } => {
            panic!("unexpected diagnostics: {diagnostics:?}")
        }
        _ => panic!("unknown preparation outcome"),
    }
}

struct Prepared {
    identity: PreparedPublishIdentity,
}

impl BuildPreparedHandle for Prepared {
    fn identity(&self) -> PreparedPublishIdentity {
        self.identity.clone()
    }
}

#[derive(Default)]
struct CountingPublisher {
    prepare_calls: usize,
    commit_calls: usize,
}

impl BuildPublisher for CountingPublisher {
    type Prepared = Prepared;

    fn prepare(
        &mut self,
        request: &recite_compiler::BuildRequest,
        candidates: &[recite_compiler::BuildCandidate],
        _control: &BuildControl,
    ) -> Result<Self::Prepared, PublishFailure> {
        self.prepare_calls += 1;
        Ok(Prepared {
            identity: PreparedPublishIdentity::for_request(request, candidates.to_vec()),
        })
    }

    fn abort(&mut self, _prepared: Option<Self::Prepared>, _reason: PublishAbortReason) {}

    fn commit(&mut self, prepared: Self::Prepared) -> PublishOutcome {
        self.commit_calls += 1;
        PublishOutcome::Published {
            targets: prepared
                .identity
                .candidates()
                .iter()
                .map(|candidate| candidate.target().clone())
                .collect(),
        }
    }
}

#[test]
fn engine_refuses_a_foreign_request_without_candidates() {
    let temp = require(TempDir::new(), "tempdir");
    write_file(
        temp.path(),
        "recite.project.toml",
        &manifest(
            "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/a.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
        ),
    );
    write_file(temp.path(), "dialogue/main.recite", source());
    let request_a = ready(temp.path());
    let request_b = require(
        ProjectBuildRequest::prepare_with_generations(
            temp.path(),
            BuildGeneration::new(1),
            SnapshotGeneration::initial(),
        ),
        "second preparation",
    )
    .into_request()
    .expect("second request");
    let mut engine = ProjectBuildEngine::new(&request_a);

    let check = engine.check(request_b.build_request(), &BuildControl::new());
    assert_eq!(
        check.freshness().expected(),
        request_a.build_request().fingerprints()
    );
    let direct = engine.build(request_b.build_request(), &BuildControl::new());
    assert!(matches!(
        direct,
        Err(BuildFailure::Engine {
            reason: BuildFailureReason::Host
        })
    ));
}

#[test]
fn coordinator_rejects_foreign_request_before_publication() {
    let temp = require(TempDir::new(), "tempdir");
    write_file(
        temp.path(),
        "recite.project.toml",
        &manifest(
            "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/a.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
        ),
    );
    write_file(temp.path(), "dialogue/main.recite", source());
    let request_a = ready(temp.path());
    let request_b = require(
        ProjectBuildRequest::prepare_with_generations(
            temp.path(),
            BuildGeneration::new(1),
            SnapshotGeneration::initial(),
        ),
        "second preparation",
    )
    .into_request()
    .expect("second request");
    let mut engine = ProjectBuildEngine::new(&request_a);
    let mut publisher = CountingPublisher::default();
    let result = require(
        recite_compiler::BuildCoordinator::new().run(
            request_b.build_request().clone(),
            &BuildControl::new(),
            &mut engine,
            &mut publisher,
        ),
        "coordinator run",
    );

    assert!(matches!(
        result.failure(),
        Some(BuildResultFailure::Check(
            recite_compiler::BuildCheckError::RequestMismatch
        ))
    ));
    assert!(result.candidates().is_empty());
    assert_eq!(publisher.prepare_calls, 0);
    assert_eq!(publisher.commit_calls, 0);
}

#[test]
fn multiple_targets_are_ordered_and_repeatable_in_memory() {
    let temp = require(TempDir::new(), "tempdir");
    write_file(
        temp.path(),
        "recite.project.toml",
        &manifest(
            "[[scenes]]\nid = \"scene.z\"\nasset = \"compiled/z.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n\n[[scenes]]\nid = \"scene.a\"\nasset = \"compiled/a.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
        ),
    );
    write_file(temp.path(), "dialogue/main.recite", source());
    let request = ready(temp.path());
    let mut first = ProjectBuildEngine::new(&request);
    let mut second = ProjectBuildEngine::new(&request);
    let control = BuildControl::new();
    let first = require(
        first.build(request.build_request(), &control),
        "first build",
    );
    let second = require(
        second.build(request.build_request(), &control),
        "second build",
    );

    assert_eq!(first, second);
    assert_eq!(
        first
            .iter()
            .map(|candidate| candidate.target().as_str())
            .collect::<Vec<_>>(),
        ["compiled/a.recitec", "compiled/z.recitec"]
    );
    for candidate in &first {
        let asset = require(
            decode_compiled_dialogue_messagepack(candidate.bytes()),
            "decode candidate",
        );
        assert_eq!(asset.header.asset_id.as_str(), candidate.target().as_str());
    }
}

#[test]
fn discovery_io_failures_remain_typed_preparation_errors() {
    let missing = require(TempDir::new(), "tempdir");
    assert!(matches!(
        ProjectBuildRequest::prepare(missing.path()),
        Err(recite_cli::watch::ProjectBuildPreparationError::Discovery(
            recite_config::ProjectDiscoveryError::NotFound { .. }
        ))
    ));

    let read = require(TempDir::new(), "tempdir");
    require(
        fs::create_dir(read.path().join("recite.project.toml")),
        "manifest directory",
    );
    assert!(matches!(
        ProjectBuildRequest::prepare(read.path()),
        Err(recite_cli::watch::ProjectBuildPreparationError::Discovery(
            recite_config::ProjectDiscoveryError::Read { .. }
        ))
    ));

    let non_utf8 = require(TempDir::new(), "tempdir");
    require(
        fs::write(non_utf8.path().join("recite.project.toml"), [0xff, 0xfe]),
        "non-UTF8 manifest",
    );
    assert!(matches!(
        ProjectBuildRequest::prepare(non_utf8.path()),
        Err(recite_cli::watch::ProjectBuildPreparationError::Discovery(
            recite_config::ProjectDiscoveryError::NonUtf8 { .. }
        ))
    ));
}
