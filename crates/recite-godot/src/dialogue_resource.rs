use godot::builtin::{GString, PackedByteArray, VarDictionary};
use godot::classes::{FileAccess, IResource, Resource};
use godot::prelude::*;

use crate::adapter::{AdapterError, AdapterErrorKind, AdapterResult, ReciteDialogueAsset};
use crate::binding_types::ReciteOperationResult;
use crate::convert::error_dictionary;

#[derive(GodotClass)]
#[class(init, base=Resource)]
pub struct ReciteDialogueResource {
    base: Base<Resource>,
    asset: Option<ReciteDialogueAsset>,
    #[var(set, usage_flags = [STORAGE])]
    compiled_bytes: PackedByteArray,
    #[var(usage_flags = [STORAGE])]
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
        self.cloned_asset()
            .map_or_else(|_| GString::new(), |asset| GString::from(asset.asset_id()))
    }

    #[func]
    fn content_identity(&self) -> GString {
        self.cloned_asset()
            .and_then(|asset| asset.content_identity())
            .map_or_else(|_| GString::new(), |identity| GString::from(&identity))
    }

    #[func]
    fn is_loaded(&self) -> bool {
        self.cloned_asset().is_ok()
    }

    #[func]
    pub(crate) fn last_error(&self) -> VarDictionary {
        self.last_error.clone()
    }

    #[func]
    pub(crate) fn revision_info(&self) -> VarDictionary {
        let mut info = VarDictionary::new();
        info.set("loaded", self.asset.is_some());
        if let Some(asset) = &self.asset {
            info.set("asset_id", asset.asset_id());
            match asset.content_identity() {
                Ok(identity) => info.set("content_identity", identity),
                Err(error) => info.set("identity_error", &error_dictionary(&error).to_variant()),
            }
        }
        info.set("import_error", &self.last_error().to_variant());
        info.set("source_freshness", "unavailable");
        info.set("schema_freshness", "unavailable");
        info
    }

    #[func]
    fn set_compiled_bytes(&mut self, bytes: PackedByteArray) {
        // Godot calls this setter when a saved Resource is deserialized. A
        // rejected refresh must leave both the bytes and decoded asset intact.
        let _result = self.load_from_rust_bytes(bytes.as_slice());
    }

    fn load_from_rust_bytes(&mut self, bytes: &[u8]) -> Gd<ReciteOperationResult> {
        match ReciteDialogueAsset::load_from_bytes(bytes) {
            Ok(asset) => {
                self.asset = Some(asset);
                self.compiled_bytes = PackedByteArray::from(bytes);
                self.last_error = VarDictionary::new();
                ReciteOperationResult::success(Vec::new())
            }
            Err(error) => {
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
