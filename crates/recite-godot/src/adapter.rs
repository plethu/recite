use std::collections::BTreeMap;

use recite_adapter::{SessionDriver, StartRequest};
use recite_core::{ChoiceId, EffectId};
use recite_runtime::{
    ConditionEvaluationError, ConditionEvaluationErrorKind, ConditionQuery, ConditionValue,
    DialogueContext, EffectAck, LocaleResolution, localisation::InterpolationValues,
};

pub(crate) use crate::adapter_error::{AdapterError, AdapterErrorKind, AdapterResult};
use crate::adapter_policy::session_options;
pub(crate) use crate::adapter_surface::{
    AdapterValue, ConditionCall, ReciteDialogueAsset, ReciteOutput,
};
use crate::catalog::ReciteDialogueCatalog;

pub type ConditionHandlerResult = Result<ConditionValue, AdapterError>;
type ConditionHandler = dyn Fn(ConditionCall<'_>) -> ConditionHandlerResult;

#[derive(Default)]
pub struct ReciteDialogueDriver {
    driver: SessionDriver,
    conditions: BTreeMap<String, Box<ConditionHandler>>,
    interpolation_values: InterpolationValues,
    locale_catalog: Option<ReciteDialogueCatalog>,
    locale_variant: Option<String>,
}

impl ReciteDialogueDriver {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_condition<F>(&mut self, name: impl Into<String>, handler: F)
    where
        F: Fn(ConditionCall<'_>) -> ConditionHandlerResult + 'static,
    {
        self.conditions.insert(name.into(), Box::new(handler));
    }

    pub fn unregister_condition(&mut self, name: &str) {
        self.conditions.remove(name);
    }

    pub fn set_interpolation_values(&mut self, values: InterpolationValues) {
        self.interpolation_values = values;
    }

    pub fn set_locale_catalog(&mut self, catalog: ReciteDialogueCatalog) {
        self.locale_catalog = Some(catalog);
    }

    pub fn clear_locale_catalog(&mut self) {
        self.locale_catalog = None;
    }

    pub fn set_locale_variant(&mut self, variant: Option<&str>) -> AdapterResult<()> {
        self.locale_variant = validate_variant(variant)?;
        Ok(())
    }

    pub fn start(
        &mut self,
        asset: &ReciteDialogueAsset,
        block_id: Option<&str>,
        locale: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        self.start_with_variant(asset, block_id, locale, None)
    }

    pub fn start_with_variant(
        &mut self,
        asset: &ReciteDialogueAsset,
        block_id: Option<&str>,
        locale: Option<&str>,
        variant: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        let variant = validate_variant(variant)?;
        let options = session_options(locale)?;
        let context = GodotContext {
            conditions: &self.conditions,
        };
        let resolution = locale_resolution(
            &self.interpolation_values,
            self.locale_catalog.as_ref(),
            variant.as_deref(),
        );
        let output = self.driver.start(
            StartRequest {
                asset: asset.loaded(),
                block_id,
                options,
            },
            &context,
            resolution,
        )?;
        self.locale_variant = variant;
        Ok(output)
    }

    pub fn select_choice(&mut self, choice_id: &str) -> AdapterResult<Vec<ReciteOutput>> {
        let choice = ChoiceId::new(choice_id).map_err(|error| {
            AdapterError::with_detail(AdapterErrorKind::InvalidChoice, error.to_string())
        })?;
        let context = GodotContext {
            conditions: &self.conditions,
        };
        let resolution = locale_resolution(
            &self.interpolation_values,
            self.locale_catalog.as_ref(),
            self.locale_variant.as_deref(),
        );
        self.driver.select_choice(choice, &context, resolution)
    }

    pub fn acknowledge_effect(
        &mut self,
        effect_request_id: &str,
        succeeded: bool,
        failure_reason: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        let effect = EffectId::new(effect_request_id).map_err(|error| {
            AdapterError::with_detail(AdapterErrorKind::EffectAcknowledgement, error.to_string())
        })?;
        let ack = if succeeded {
            EffectAck::Completed
        } else {
            EffectAck::Failed {
                reason: failure_reason.unwrap_or("").to_owned(),
            }
        };
        let context = GodotContext {
            conditions: &self.conditions,
        };
        let resolution = locale_resolution(
            &self.interpolation_values,
            self.locale_catalog.as_ref(),
            self.locale_variant.as_deref(),
        );
        self.driver
            .acknowledge_effect(effect, ack, &context, resolution)
    }

    pub fn snapshot(&self) -> AdapterResult<Vec<u8>> {
        self.driver.snapshot()
    }

    pub fn restore(
        &mut self,
        asset: &ReciteDialogueAsset,
        bytes: &[u8],
    ) -> AdapterResult<Vec<ReciteOutput>> {
        self.restore_with_variant(asset, bytes, None)
    }

    pub fn restore_with_variant(
        &mut self,
        asset: &ReciteDialogueAsset,
        bytes: &[u8],
        variant: Option<&str>,
    ) -> AdapterResult<Vec<ReciteOutput>> {
        let variant = validate_variant(variant)?;
        let context = GodotContext {
            conditions: &self.conditions,
        };
        let resolution = locale_resolution(
            &self.interpolation_values,
            self.locale_catalog.as_ref(),
            variant.as_deref(),
        );
        let output = self
            .driver
            .restore(asset.loaded(), bytes, &context, resolution)?;
        self.locale_variant = variant;
        Ok(output)
    }

    #[must_use]
    pub fn active_asset_id(&self) -> Option<&str> {
        self.driver
            .active_asset()
            .map(recite_adapter::LoadedDialogue::asset_id)
    }

    pub fn active_content_identity(&self) -> AdapterResult<Option<String>> {
        self.driver
            .active_asset()
            .map(recite_adapter::LoadedDialogue::content_identity)
            .transpose()
    }

    pub fn end_session(&mut self) -> AdapterResult<()> {
        self.driver.end_session()
    }

    #[must_use]
    pub fn has_active_session(&self) -> bool {
        self.driver.has_active_session()
    }
}

fn validate_variant(variant: Option<&str>) -> AdapterResult<Option<String>> {
    let Some(variant) = variant.filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    if variant.contains('\0') {
        return Err(AdapterError::with_detail(
            AdapterErrorKind::Localisation,
            "locale variant must not contain NUL",
        ));
    }
    Ok(Some(variant.to_owned()))
}

fn locale_resolution<'a>(
    values: &'a InterpolationValues,
    catalog: Option<&'a ReciteDialogueCatalog>,
    variant: Option<&'a str>,
) -> LocaleResolution<'a> {
    let resolution = LocaleResolution::new().with_values(values);
    let resolution = variant.map_or(resolution, |variant| resolution.with_variant(variant));
    catalog.map_or(resolution, |catalog| resolution.with_provider(catalog))
}

struct GodotContext<'a> {
    conditions: &'a BTreeMap<String, Box<ConditionHandler>>,
}

impl DialogueContext for GodotContext<'_> {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let Some(handler) = self.conditions.get(query.function()) else {
            return Err(ConditionEvaluationError::with_kind(
                ConditionEvaluationErrorKind::MissingHandler,
                "missing_condition_handler_error",
            ));
        };
        handler(ConditionCall { query }).map_err(|error| {
            let kind = match error.kind() {
                AdapterErrorKind::MissingConditionHandler => {
                    ConditionEvaluationErrorKind::MissingHandler
                }
                AdapterErrorKind::InvalidConditionResult => {
                    ConditionEvaluationErrorKind::InvalidResult
                }
                _ => ConditionEvaluationErrorKind::EvaluationFailed,
            };
            ConditionEvaluationError::with_kind(kind, error.message())
        })
    }
}
