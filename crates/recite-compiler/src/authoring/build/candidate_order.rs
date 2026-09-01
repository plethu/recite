use super::publish::BuildCandidate;
use std::cmp::Ordering;

pub(crate) fn compare_candidates(left: &BuildCandidate, right: &BuildCandidate) -> Ordering {
    left.target()
        .cmp(right.target())
        .then_with(|| left.bytes().cmp(right.bytes()))
}

pub(crate) fn candidates_are_ordered(candidates: &[BuildCandidate]) -> bool {
    candidates
        .windows(2)
        .all(|pair| compare_candidates(&pair[0], &pair[1]).is_le())
}
