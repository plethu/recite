use godot::builtin::{Callable, GString, PackedByteArray, VarDictionary};
use godot::classes::{FileAccess, INode, IResource, Node, Resource};
use godot::prelude::*;
use recite_runtime::{ConditionExpectedType, ConditionValue};

use crate::adapter::{
    AdapterError, AdapterErrorKind, AdapterResult, ConditionCall, ReciteDialogueAsset,
    ReciteDialogueDriver, ReciteOutput as AdapterOutput,
};
use crate::binding_types::{ReciteAdapterError, ReciteOperationResult, ReciteOutputObject};
use crate::catalog_resource::ReciteDialogueCatalogResource;
use crate::convert::{error_dictionary, interpolation_values, output_dictionary};

#[derive(GodotClass)]
#[class(init, base=Resource)]
pub struct ReciteDialogueResource {
    base: Base<Resource>,
    asset: Option<ReciteDialogueAsset>,
    last_error: VarDictionary,
}

#[godot_api]
impl IResource for ReciteDialogueResource {}

#[godot_api]
impl ReciteDialogueResource {
    #[func]
    fn load_from_path(&mut self, path: GString) -> Gd<ReciteOperationResult> {
        let bytes = FileAccess::get_file_as_bytes(&path);
        if bytes.is_empty() {
            let error = AdapterError::with_detail(
                AdapterErrorKind::AssetLoadOrDecode,
                format!("failed to read `{path}` through Godot FileAccess"),
            );
            self.last_error = error_dictionary(&error);
            return ReciteOperationResult::failure(error);
        }

        self.load_from_rust_bytes(bytes.as_slice())
    }

    #[func]
    fn load_from_bytes(&mut self, bytes: PackedByteArray) -> Gd<ReciteOperationResult> {
        self.load_from_rust_bytes(bytes.as_slice())
    }

    #[func]
    fn asset_id(&self) -> GString {
        self.asset
            .as_ref()
            .map_or_else(GString::new, |asset| GString::from(asset.asset_id()))
    }

    #[func]
    fn is_loaded(&self) -> bool {
        self.asset.is_some()
    }

    #[func]
    fn last_error(&self) -> VarDictionary {
        self.last_error.clone()
    }

    fn load_from_rust_bytes(&mut self, bytes: &[u8]) -> Gd<ReciteOperationResult> {
        match ReciteDialogueAsset::load_from_bytes(bytes) {
            Ok(asset) => {
                self.asset = Some(asset);
                self.last_error = VarDictionary::new();
                ReciteOperationResult::success(Vec::new())
            }
            Err(error) => {
                self.asset = None;
                self.last_error = error_dictionary(&error);
                ReciteOperationResult::failure(error)
            }
        }
    }

    pub(crate) fn cloned_asset(&self) -> AdapterResult<ReciteDialogueAsset> {
        self.asset.as_ref().cloned().ok_or_else(|| {
            AdapterError::with_detail(
                AdapterErrorKind::AssetLoadOrDecode,
                "ReciteDialogueResource has no loaded asset",
            )
        })
    }
}

#[derive(GodotClass)]
#[class(init, base=Node)]
pub struct ReciteDialogueNode {
    base: Base<Node>,
    driver: ReciteDialogueDriver,
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
                for output in &outputs {
                    self.emit_output(output);
                }
                ReciteOperationResult::success(outputs)
            }
            Err(error) => self.emit_error_result(error),
        }
    }

    fn emit_output(&mut self, output: &AdapterOutput) {
        let output = ReciteOutputObject::new(output_dictionary(output));
        self.base_mut()
            .emit_signal("output", &[output.clone().to_variant()]);
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
