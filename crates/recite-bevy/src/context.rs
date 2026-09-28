use std::collections::BTreeMap;

use bevy_ecs::resource::Resource;
use bevy_ecs::world::World;
use recite_adapter::ReciteDialogueCatalog;
use recite_runtime::{
    ConditionEvaluationError, ConditionEvaluationErrorKind, ConditionQuery, ConditionValue,
    DialogueContext, localisation::InterpolationValues,
};

type ConditionHandler = dyn for<'a> Fn(&World, ConditionQuery<'a>) -> Result<ConditionValue, ConditionEvaluationError>
    + Send
    + Sync;

/// Registered pure condition queries. Handlers may inspect `&World`, but must
/// not mutate game state, advance time, or rely on unordered iteration.
#[derive(Resource, Default)]
pub struct ReciteConditions {
    handlers: BTreeMap<String, Box<ConditionHandler>>,
}

impl ReciteConditions {
    pub fn register(
        &mut self,
        name: impl Into<String>,
        handler: impl for<'a> Fn(
            &World,
            ConditionQuery<'a>,
        ) -> Result<ConditionValue, ConditionEvaluationError>
        + Send
        + Sync
        + 'static,
    ) {
        self.handlers.insert(name.into(), Box::new(handler));
    }

    pub fn unregister(&mut self, name: &str) {
        self.handlers.remove(name);
    }
}

pub(crate) struct WorldContext<'a> {
    pub(crate) world: &'a World,
}

impl DialogueContext for WorldContext<'_> {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let name = query.function();
        let registry = self.world.resource::<ReciteConditions>();
        let Some(handler) = registry.handlers.get(name) else {
            return Err(ConditionEvaluationError::with_kind(
                ConditionEvaluationErrorKind::MissingHandler,
                format!("no Bevy condition handler registered for `{name}`"),
            ));
        };
        handler(self.world, query)
    }
}

/// Host-owned gettext catalogue. The owner borrows this resource for every
/// operation, so replacing it affects the next operation of an active session.
/// Session locale remains explicit in `ReciteRequest::Start`.
#[derive(Resource, Default)]
pub struct ReciteCatalog(pub ReciteDialogueCatalog);

/// Caller-owned interpolation values, excluded from Recite session snapshots.
#[derive(Resource, Default)]
pub struct ReciteInterpolation(pub InterpolationValues);
