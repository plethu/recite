//! A `:match` must not have an arm traversal can never select.

use std::collections::BTreeMap;

use recite_core::SourceSpan;
use recite_core::ast::{MatchBranch, MatchPattern};

use super::super::participation::ValidationCompleteness;
use super::super::state::Validator;
use crate::diagnostics;

impl Validator<'_> {
    /// Traversal selects the first arm for a variant, so a repeated variant
    /// never runs. The wildcard must be the last arm, as the spec requires:
    /// a second wildcard never runs, and a variant arm after it reads as if
    /// the wildcard would catch it first.
    pub(super) fn validate_match_arm_patterns(&mut self, branch: &MatchBranch) {
        if self.participation.ast_structure() != ValidationCompleteness::Complete {
            return;
        }
        let mut variants: BTreeMap<&str, SourceSpan> = BTreeMap::new();
        let mut wildcard: Option<SourceSpan> = None;
        for arm in &branch.arms {
            let pattern = match &arm.pattern {
                MatchPattern::Variant(variant) => variant.as_str(),
                MatchPattern::Wildcard => "_",
            };
            if let Some(wildcard_span) = &wildcard {
                self.diagnostics
                    .push(diagnostics::match_case_after_wildcard(
                        pattern,
                        arm.span.clone(),
                        wildcard_span.clone(),
                    ));
                continue;
            }
            match &arm.pattern {
                MatchPattern::Wildcard => wildcard = Some(arm.span.clone()),
                MatchPattern::Variant(variant) => {
                    if let Some(first) = variants.get(variant.as_str()) {
                        self.diagnostics.push(diagnostics::duplicate_match_case(
                            variant,
                            arm.span.clone(),
                            first.clone(),
                        ));
                    } else {
                        variants.insert(variant.as_str(), arm.span.clone());
                    }
                }
            }
        }
    }
}
