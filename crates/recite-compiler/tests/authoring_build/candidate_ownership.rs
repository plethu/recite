use super::support::*;
use recite_compiler::authoring::{BuildCandidate, BuildInput, BuildState, BuildStatusProjection};

#[test]
fn cloned_candidates_share_bytes_and_keep_value_equality() {
    let mut source = vec![1, 2, 3];
    let original = BuildCandidate::new(target("dialogue.recitec"), source.as_slice());
    source.fill(9);
    assert_eq!(original.bytes(), &[1, 2, 3]);
    let cloned = original.clone();
    assert!(std::ptr::eq(original.bytes(), cloned.bytes()));
    assert_eq!(original, cloned);
    assert_eq!(cloned, candidate("dialogue.recitec", &[1, 2, 3]));
    assert_ne!(cloned, candidate("other.recitec", &[1, 2, 3]));
    assert_ne!(cloned, candidate("dialogue.recitec", &[1, 2, 4]));
    drop(original);
    assert_eq!(cloned.bytes(), &[1, 2, 3]);
}

#[test]
fn status_projections_share_candidate_bytes_after_the_source_state_is_dropped() {
    let request = make_request(1, [BuildInput::saved_source(key("dialogue.recite"), "x")]);
    let state = BuildState::Ready {
        freshness: freshness(&request),
        request,
        candidates: vec![candidate("dialogue.recitec", &[1, 2, 3])],
        diagnostics: Vec::new(),
    };
    let projection = BuildStatusProjection::from_state(&state);
    let earlier = projection.clone();
    assert!(std::ptr::eq(
        state.candidates()[0].bytes(),
        projection.candidates()[0].bytes(),
    ));
    assert!(std::ptr::eq(
        projection.candidates()[0].bytes(),
        earlier.candidates()[0].bytes(),
    ));
    drop(state);
    drop(projection);
    assert_eq!(
        earlier.candidates(),
        &[candidate("dialogue.recitec", &[1, 2, 3])]
    );
}
