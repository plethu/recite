use godot::builtin::{Callable, GString, PackedByteArray, VarDictionary, Variant};
use godot::classes::{INode, Node};
use godot::prelude::*;
use recite_runtime::{ConditionExpectedType, ConditionValue};

use crate::adapter::{
    AdapterError, AdapterErrorKind, AdapterResult, ConditionCall, ReciteDialogueDriver,
    ReciteOutput as AdapterOutput,
};
use crate::binding_types::{
    ReciteAdapterError, ReciteConditionFailure, ReciteOperationResult, ReciteOutputObject,
};
use crate::catalog_resource::ReciteDialogueCatalogResource;
use crate::convert::{error_dictionary, interpolation_values, output_dictionary};
use crate::dialogue_resource::ReciteDialogueResource;

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct ReciteDialogueNode {
    base: Base<Node>,
    driver: ReciteDialogueDriver,
    pending_outputs: VecDeque<AdapterOutput>,
    emitting_outputs: bool,
}

#[godot_api]
impl INode for ReciteDialogueNode {}

#[godot_api]
impl ReciteDialogueNode {
    #[signal]
    fn output(output: Gd<ReciteOutputObject>);

    #[signal]
    fn adapter_error(error: Gd<ReciteAdapterError>);

    #[func]
    fn start(
        &mut self,
        asset: Gd<ReciteDialogueResource>,
        block_id: GString,
        locale: GString,
    ) -> Gd<ReciteOperationResult> {
        self.start_with_variant(asset, block_id, locale, GString::new())
    }

    #[func]
    fn start_with_variant(
        &mut self,
        asset: Gd<ReciteDialogueResource>,
        block_id: GString,
        locale: GString,
        variant: GString,
    ) -> Gd<ReciteOperationResult> {
        let asset = match asset.bind().cloned_asset() {
            Ok(asset) => asset,
            Err(error) => return self.emit_error_result(error),
        };
        let block_id = optional_string(block_id);
        let locale = optional_string(locale);
        let variant = optional_string(variant);
        let result = self.driver.start_with_variant(
            &asset,
            block_id.as_deref(),
            locale.as_deref(),
            variant.as_deref(),
        );
        self.apply_driver_result(result)
    }

    #[func]
    fn set_locale_variant(&mut self, variant: GString) -> Gd<ReciteOperationResult> {
        match self
            .driver
            .set_locale_variant(optional_string(variant).as_deref())
        {
            Ok(()) => ReciteOperationResult::success(Vec::new()),
            Err(error) => self.emit_error_result(error),
        }
    }

    #[func]
    fn clear_locale_variant(&mut self) -> Gd<ReciteOperationResult> {
        match self.driver.set_locale_variant(None) {
            Ok(()) => ReciteOperationResult::success(Vec::new()),
            Err(error) => self.emit_error_result(error),
        }
    }

    #[func]
    fn set_locale_catalog(
        &mut self,
        catalog: Gd<ReciteDialogueCatalogResource>,
    ) -> Gd<ReciteOperationResult> {
        match catalog.bind().cloned_catalog() {
            Ok(catalog) => {
                self.driver.set_locale_catalog(catalog);
                ReciteOperationResult::success(Vec::new())
            }
            Err(error) => self.emit_error_result(error),
        }
    }

    #[func]
    fn clear_locale_catalog(&mut self) -> Gd<ReciteOperationResult> {
        self.driver.clear_locale_catalog();
        ReciteOperationResult::success(Vec::new())
    }

    #[func]
    fn select_choice(&mut self, choice_id: GString) -> Gd<ReciteOperationResult> {
        let choice_id = choice_id.to_string();
        let result = self.driver.select_choice(&choice_id);
        self.apply_driver_result(result)
    }

    #[func]
    fn acknowledge_effect(
        &mut self,
        effect_request_id: GString,
        succeeded: bool,
        failure_reason: GString,
    ) -> Gd<ReciteOperationResult> {
        let effect_request_id = effect_request_id.to_string();
        let failure_reason = optional_string(failure_reason);
        let result = self.driver.acknowledge_effect(
            &effect_request_id,
            succeeded,
            failure_reason.as_deref(),
        );
        self.apply_driver_result(result)
    }

    #[func]
    fn snapshot(&mut self) -> Gd<ReciteOperationResult> {
        match self.driver.snapshot() {
            Ok(bytes) => ReciteOperationResult::success_with_snapshot(bytes),
            Err(error) => self.emit_error_result(error),
        }
    }

    #[func]
    fn restore(
        &mut self,
        asset: Gd<ReciteDialogueResource>,
        snapshot_bytes: PackedByteArray,
    ) -> Gd<ReciteOperationResult> {
        self.restore_with_variant(asset, snapshot_bytes, GString::new())
    }

    #[func]
    fn restore_with_variant(
        &mut self,
        asset: Gd<ReciteDialogueResource>,
        snapshot_bytes: PackedByteArray,
        variant: GString,
    ) -> Gd<ReciteOperationResult> {
        let asset = match asset.bind().cloned_asset() {
            Ok(asset) => asset,
            Err(error) => return self.emit_error_result(error),
        };
        let variant = optional_string(variant);
        let result =
            self.driver
                .restore_with_variant(&asset, snapshot_bytes.as_slice(), variant.as_deref());
        self.apply_driver_result(result)
    }

    #[func]
    fn end_session(&mut self) -> Gd<ReciteOperationResult> {
        match self.driver.end_session() {
            Ok(()) => ReciteOperationResult::success(Vec::new()),
            Err(error) => self.emit_error_result(error),
        }
    }

    #[func]
    fn active_asset_id(&self) -> GString {
        self.driver
            .active_asset_id()
            .map_or_else(GString::new, GString::from)
    }

    #[func]
    fn active_content_identity(&self) -> GString {
        self.driver
            .active_content_identity()
            .ok()
            .flatten()
            .map_or_else(GString::new, |identity| GString::from(&identity))
    }

    #[func]
    fn changed_asset_policy(&self) -> GString {
        GString::from("reload_for_next_session_only")
    }

    #[func]
    fn asset_state(&self, asset: Gd<ReciteDialogueResource>) -> VarDictionary {
        let available = asset.bind().revision_info();
        let mut state = VarDictionary::new();
        state.set("available", &available.to_variant());
        let mut active = VarDictionary::new();
        if let Some(asset_id) = self.driver.active_asset_id() {
            active.set("asset_id", asset_id);
            match self.driver.active_content_identity() {
                Ok(Some(identity)) => active.set("content_identity", identity),
                Ok(None) => {}
                Err(error) => active.set("identity_error", &error_dictionary(&error).to_variant()),
            }
            state.set("active", &active.to_variant());
        } else {
            state.set("active", &Variant::nil());
        }
        let active_identity = active.get("content_identity");
        let available_identity = available.get("content_identity");
        state.set(
            "active_differs_from_available",
            active_identity.is_some()
                && available_identity.is_some()
                && active_identity != available_identity,
        );
        state.set("changed_asset_policy", "reload_for_next_session_only");
        state
    }

    #[func]
    fn register_condition(&mut self, name: GString, callable: Callable) {
        let name = name.to_string();
        self.driver.register_condition(name, move |call| {
            evaluate_callable_condition(&callable, call)
        });
    }

    #[func]
    fn unregister_condition(&mut self, name: GString) {
        self.driver.unregister_condition(&name.to_string());
    }

    #[func]
    fn set_interpolation_values(&mut self, values: VarDictionary) -> Gd<ReciteOperationResult> {
        match interpolation_values(&values) {
            Ok(values) => {
                self.driver.set_interpolation_values(values);
                ReciteOperationResult::success(Vec::new())
            }
            Err(error) => self.emit_error_result(error),
        }
    }

    fn apply_driver_result(
        &mut self,
        result: AdapterResult<Vec<AdapterOutput>>,
    ) -> Gd<ReciteOperationResult> {
        match result {
            Ok(outputs) => {
                self.pending_outputs.extend(outputs.iter().cloned());
                self.drain_outputs();
                ReciteOperationResult::success(outputs)
            }
            Err(error) => self.emit_error_result(error),
        }
    }

    fn drain_outputs(&mut self) {
        if self.emitting_outputs {
            return;
        }
        self.emitting_outputs = true;
        while let Some(output) = self.pending_outputs.pop_front() {
            let output = ReciteOutputObject::new(output_dictionary(&output));
            self.base_mut()
                .emit_signal("output", &[output.to_variant()]);
        }
        self.emitting_outputs = false;
    }

    fn emit_error_result(&mut self, error: AdapterError) -> Gd<ReciteOperationResult> {
        let error_object = ReciteAdapterError::new(error.clone());
        self.base_mut()
            .emit_signal("adapter_error", &[error_object.to_variant()]);
        ReciteOperationResult::failure(error)
    }
}

pub(crate) fn optional_string(value: GString) -> Option<String> {
    let value = value.to_string();
    if value.is_empty() { None } else { Some(value) }
}

pub(crate) fn catalog_result(result: AdapterResult<()>) -> Gd<ReciteOperationResult> {
    match result {
        Ok(()) => ReciteOperationResult::success(Vec::new()),
        Err(error) => ReciteOperationResult::failure(error),
    }
}

fn evaluate_callable_condition(
    callable: &Callable,
    call: ConditionCall<'_>,
) -> AdapterResult<ConditionValue> {
    if !callable.is_valid() {
        return Err(AdapterError::new(AdapterErrorKind::MissingConditionHandler));
    }

    let query = condition_query_dictionary(call);
    let result = callable.call(&[query.to_variant()]);
    if let Ok(failure) = result.try_to::<Gd<ReciteConditionFailure>>() {
        return Err(AdapterError::with_detail(
            AdapterErrorKind::ConditionEvaluationFailed,
            failure.bind().reason(),
        ));
    }
    match call.expected_type() {
        ConditionExpectedType::Bool => result.try_to::<bool>().map(ConditionValue::Bool),
        ConditionExpectedType::Enum => result
            .try_to::<GString>()
            .map(|value| ConditionValue::EnumVariant(value.to_string())),
    }
    .map_err(|error| {
        AdapterError::with_detail(
            AdapterErrorKind::InvalidConditionResult,
            format!("condition callable returned an incompatible value: {error}"),
        )
    })
}

fn condition_query_dictionary(call: ConditionCall<'_>) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("function", call.function());
    dictionary.set(
        "expected_type",
        match call.expected_type() {
            ConditionExpectedType::Bool => "bool",
            ConditionExpectedType::Enum => "enum",
        },
    );
    let mut args = VarArray::new();
    for arg in call.arguments() {
        let value = crate::convert::adapter_value_dictionary(&arg);
        args.push(&value.to_variant());
    }
    dictionary.set("args", &args.to_variant());
    dictionary
}
use std::collections::VecDeque;
