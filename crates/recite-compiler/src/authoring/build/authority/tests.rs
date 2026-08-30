use super::super::super::SnapshotGeneration;
use super::super::identity::{BuildGeneration, BuildInput};
use super::super::publish::{
    BuildCandidate, BuildPreparedHandle, BuildTarget, PreparedPublishIdentity, PublishOutcome,
};
use super::super::request::BuildRequest;
use super::{BuildAuthority, BuildAuthorityCommitError, BuildAuthorityFence};

struct Handle(PreparedPublishIdentity);
impl BuildPreparedHandle for Handle {
    fn identity(&self) -> PreparedPublishIdentity {
        self.0.clone()
    }
}

#[test]
fn authority_refusal_after_permit_relock_returns_prepared_handle() {
    let request_a = BuildRequest::new(
        BuildGeneration::new(1),
        SnapshotGeneration::new(1),
        [BuildInput::saved_source(key("a.recite"), "a")],
    )
    .unwrap_or_else(|error| panic!("request A: {error}"));
    let request_b = BuildRequest::new(
        BuildGeneration::new(2),
        SnapshotGeneration::new(2),
        [BuildInput::saved_source(key("a.recite"), "b")],
    )
    .unwrap_or_else(|error| panic!("request B: {error}"));
    let fence = BuildAuthorityFence::new(BuildAuthority::from_request(&request_a));
    let permit = fence
        .acquire(&request_a)
        .unwrap_or_else(|error| panic!("permit: {error}"));
    fence
        .install_if_newer(BuildAuthority::from_request(&request_b))
        .unwrap_or_else(|error| panic!("install B: {error}"));
    let candidate = BuildCandidate::new(
        BuildTarget::new("a.recitec").unwrap_or_else(|error| panic!("target: {error}")),
        b"A".to_vec(),
    );
    let handle = Handle(PreparedPublishIdentity::for_request(
        &request_a,
        vec![candidate],
    ));
    let error = permit
        .commit(handle, |_| PublishOutcome::Published {
            targets: Vec::new(),
        })
        .expect_err("B must win before A commit");
    match error {
        BuildAuthorityCommitError::Refused { prepared, .. } => {
            assert_eq!(
                prepared.identity().request_identity(),
                &super::super::request_identity::BuildRequestIdentity::from_request(&request_a)
            );
        }
        BuildAuthorityCommitError::Poisoned { .. } => panic!("authority was not poisoned"),
    }
}

fn key(value: &str) -> recite_core::DocumentKey {
    recite_core::DocumentKey::new(value).unwrap_or_else(|error| panic!("test key: {error}"))
}
