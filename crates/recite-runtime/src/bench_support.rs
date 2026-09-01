//! Maintainer-only probes for benchmark evidence.

use crate::PreviewPrompt;

/// Returns the validated plural arm cardinality attached to a preview prompt.
#[must_use]
pub fn plural_arm_count(prompt: &PreviewPrompt) -> Option<usize> {
    prompt.plural_arm_count()
}
