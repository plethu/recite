use super::condition::Operation;
use super::model::{
    ConditionAnswer, PreviewConditionRequest, PreviewEvent, PreviewInputs, PreviewStatus,
};

pub(super) struct Trial<'a> {
    pub(super) operation: Operation,
    pub(super) answers: Vec<ConditionAnswer>,
    pub(super) requests: Vec<PreviewConditionRequest>,
    pub(super) inputs: PreviewInputs<'a>,
    pub(super) prior_status: PreviewStatus,
    pub(super) prefix: Option<Vec<PreviewEvent>>,
    pub(super) runtime_trace: crate::DialogueTrace,
}

/// A pending query is also a rollback: its answers replay from the same boundary.
pub(super) enum TrialFailure {
    Pending {
        query: super::model::PreviewConditionQuery,
        block: recite_core::BlockId,
    },
    Error(super::model::PreviewError),
}

pub(super) fn traverse(
    session: &mut crate::DialogueSession,
    projection: &mut std::sync::Arc<Vec<crate::DialogueEffectRequest>>,
    asset: &recite_core::compiled::CompiledDialogue,
    operation: &Operation,
    context: &super::condition::ReplayContext<'_>,
    resolution: crate::LocaleResolution<'_>,
) -> Result<(crate::DialogueEvent, recite_core::BlockId), TrialFailure> {
    let trial = ProjectionTrial::new(session, projection);
    trial.session.try_traverse(|traversal| {
        let result = match operation {
            Operation::Advance { .. } => traversal.next_with(asset, context, resolution),
            Operation::Choose { choice_id, .. } => {
                traversal.choose_with(asset, choice_id.clone(), context, resolution)
            }
        };
        if let Some(mismatch) = context.mismatch() {
            return Err(TrialFailure::Error(
                super::model::PreviewError::ConditionReplayMismatch {
                    mismatch: mismatch.to_string(),
                },
            ));
        }
        let block = || {
            asset
                .blocks
                .get(traversal.session().active_block_index().as_u32() as usize)
                .map(|block| block.id.clone())
        };
        if let Some(query) = context.pending_query() {
            return Err(match block() {
                Some(block) => TrialFailure::Pending { query, block },
                None => malformed("condition query has no active block"),
            });
        }
        let event = result
            .map_err(|error| TrialFailure::Error(super::model::PreviewError::Runtime(error)))?;
        let block = block().ok_or_else(|| malformed("runtime event has no active block"))?;
        Ok((event, block))
    })
}

fn malformed(reason: &str) -> TrialFailure {
    TrialFailure::Error(super::model::PreviewError::Runtime(
        crate::DialogueError::MalformedCompiledAsset {
            reason: reason.to_owned(),
        },
    ))
}

// Release only the preview's redundant internal owner during traversal. Caller-
// retained outputs/snapshots still force copy-on-write. Restore on unwind too.
struct ProjectionTrial<'a> {
    session: &'a mut crate::DialogueSession,
    projection: &'a mut std::sync::Arc<Vec<crate::DialogueEffectRequest>>,
}

impl<'a> ProjectionTrial<'a> {
    fn new(
        session: &'a mut crate::DialogueSession,
        projection: &'a mut std::sync::Arc<Vec<crate::DialogueEffectRequest>>,
    ) -> Self {
        *projection = std::sync::Arc::default();
        Self {
            session,
            projection,
        }
    }
}

impl Drop for ProjectionTrial<'_> {
    fn drop(&mut self) {
        *self.projection = std::sync::Arc::clone(&self.session.deferred_effects);
    }
}
