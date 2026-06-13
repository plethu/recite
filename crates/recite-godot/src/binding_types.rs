use godot::builtin::{GString, PackedByteArray, VarArray, VarDictionary};
use godot::classes::{IRefCounted, RefCounted};
use godot::prelude::*;

use crate::adapter::{AdapterError, ReciteOutput as AdapterOutput};
use crate::convert::{bytes_to_packed, error_dictionary, outputs_array, result_dictionary};

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct ReciteOperationResult {
    base: Base<RefCounted>,
    ok: bool,
    outputs: VarArray,
    snapshot_bytes: PackedByteArray,
    error: VarDictionary,
}

#[godot_api]
impl IRefCounted for ReciteOperationResult {}

#[godot_api]
impl ReciteOperationResult {
    pub(crate) fn success(outputs: Vec<AdapterOutput>) -> Gd<Self> {
        Self::new(true, outputs, Vec::new(), None)
    }

    pub(crate) fn success_with_snapshot(snapshot_bytes: Vec<u8>) -> Gd<Self> {
        Self::new(true, Vec::new(), snapshot_bytes, None)
    }

    pub(crate) fn failure(error: AdapterError) -> Gd<Self> {
        Self::new(false, Vec::new(), Vec::new(), Some(error))
    }

    fn new(
        ok: bool,
        outputs: Vec<AdapterOutput>,
        snapshot_bytes: Vec<u8>,
        error: Option<AdapterError>,
    ) -> Gd<Self> {
        let output_array = outputs_array(&outputs);
        let snapshot_bytes = bytes_to_packed(&snapshot_bytes);
        let error_dictionary = error
            .as_ref()
            .map_or_else(VarDictionary::new, error_dictionary);
        Gd::from_init_fn(|base| Self {
            base,
            ok,
            outputs: output_array,
            snapshot_bytes,
            error: error_dictionary,
        })
    }

    #[func]
    fn is_ok(&self) -> bool {
        self.ok
    }

    #[func]
    fn outputs(&self) -> VarArray {
        self.outputs.clone()
    }

    #[func]
    fn snapshot_bytes(&self) -> PackedByteArray {
        self.snapshot_bytes.clone()
    }

    #[func]
    fn error(&self) -> VarDictionary {
        self.error.clone()
    }

    #[func]
    fn as_dictionary(&self) -> VarDictionary {
        result_dictionary(self.ok, &self.outputs, &self.snapshot_bytes, &self.error)
    }
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct ReciteOutput {
    base: Base<RefCounted>,
    data: VarDictionary,
}

#[godot_api]
impl IRefCounted for ReciteOutput {}

#[godot_api]
impl ReciteOutput {
    pub(crate) fn new(data: VarDictionary) -> Gd<Self> {
        Gd::from_init_fn(|base| Self { base, data })
    }

    #[func]
    fn data(&self) -> VarDictionary {
        self.data.clone()
    }

    #[func]
    fn kind(&self) -> GString {
        self.data
            .get("kind")
            .and_then(|value| value.try_to::<GString>().ok())
            .unwrap_or_default()
    }
}

#[derive(GodotClass)]
#[class(init, base=RefCounted)]
pub struct ReciteAdapterError {
    base: Base<RefCounted>,
    data: VarDictionary,
}

#[godot_api]
impl IRefCounted for ReciteAdapterError {}

#[godot_api]
impl ReciteAdapterError {
    pub(crate) fn new(error: AdapterError) -> Gd<Self> {
        let data = error_dictionary(&error);
        Gd::from_init_fn(|base| Self { base, data })
    }

    #[func]
    fn data(&self) -> VarDictionary {
        self.data.clone()
    }

    #[func]
    fn code(&self) -> GString {
        self.data
            .get("code")
            .and_then(|value| value.try_to::<GString>().ok())
            .unwrap_or_default()
    }

    #[func]
    fn message(&self) -> GString {
        self.data
            .get("message")
            .and_then(|value| value.try_to::<GString>().ok())
            .unwrap_or_default()
    }
}
