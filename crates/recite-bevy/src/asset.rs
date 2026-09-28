use std::collections::BTreeMap;
use std::io;

use bevy_asset::io::Reader;
use bevy_asset::{
    Asset, AssetEvent, AssetId, AssetLoadFailedEvent, AssetLoader, Assets, LoadContext,
};
use bevy_ecs::message::{Message, MessageReader, MessageWriter};
use bevy_ecs::resource::Resource;
use bevy_reflect::TypePath;
use recite_adapter::{AdapterError, AdapterErrorKind, LoadedDialogue};
use recite_core::compiled::ContentFingerprint;

/// Immutable, validated `.recitec` asset in Bevy's native asset cache.
#[derive(Asset, TypePath, Clone, Debug)]
pub struct ReciteDialogueAsset {
    dialogue: LoadedDialogue,
    revision: ReciteRevision,
}

impl ReciteDialogueAsset {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AdapterError> {
        let dialogue = LoadedDialogue::from_bytes(bytes)?;
        let fingerprint = dialogue.dialogue().content_fingerprint().map_err(|error| {
            AdapterError::with_detail(AdapterErrorKind::AssetLoadOrDecode, error.to_string())
        })?;
        let revision = ReciteRevision {
            asset_id: dialogue.asset_id().to_owned(),
            fingerprint: fingerprint.clone(),
        };
        Ok(Self { dialogue, revision })
    }

    #[must_use]
    pub fn dialogue(&self) -> &LoadedDialogue {
        &self.dialogue
    }

    #[must_use]
    pub fn revision(&self) -> &ReciteRevision {
        &self.revision
    }
}

/// CPU-only loader for compiled Recite assets.
#[derive(Default, TypePath)]
pub struct ReciteDialogueLoader;

#[derive(Debug, thiserror::Error)]
pub enum ReciteDialogueLoadError {
    #[error("compiled asset read failed: {0}")]
    Io(#[from] io::Error),
    #[error("compiled asset rejected: {0}")]
    Dialogue(#[from] AdapterError),
}

impl AssetLoader for ReciteDialogueLoader {
    type Asset = ReciteDialogueAsset;
    type Settings = ();
    type Error = ReciteDialogueLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(ReciteDialogueAsset::from_bytes(&bytes)?)
    }

    fn extensions(&self) -> &[&str] {
        &["recitec"]
    }
}

/// The authored identity of one exact compiled revision. Bevy handles and
/// entity IDs are deliberately absent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReciteRevision {
    pub asset_id: String,
    pub fingerprint: ContentFingerprint,
}

/// Source/schema freshness requires authoring inputs and is unavailable in a
/// shipping game that only sees compiled bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReciteAssetFreshness {
    Unavailable,
}

/// Bevy import result, distinct from source/schema freshness.
#[derive(Clone, Debug)]
pub enum ReciteAssetImport {
    Accepted {
        revision: ReciteRevision,
    },
    Rejected {
        error: AdapterError,
        retained: Option<ReciteRevision>,
    },
    Removed,
}

/// Result of Bevy import or refresh. The revision identifies authored content,
/// independently of Bevy's transient `AssetId`.
#[derive(Message, Clone, Debug)]
pub struct ReciteAssetStatus {
    pub handle_id: AssetId<ReciteDialogueAsset>,
    pub import: ReciteAssetImport,
    pub source_schema_freshness: ReciteAssetFreshness,
    pub path: Option<String>,
}

#[derive(Resource, Default)]
pub(crate) struct AssetCache(BTreeMap<AssetId<ReciteDialogueAsset>, CachedRevision>);

#[derive(Clone)]
pub(crate) struct CachedRevision {
    pub(crate) dialogue: LoadedDialogue,
    pub(crate) revision: ReciteRevision,
}

impl AssetCache {
    pub(crate) fn get(&self, id: AssetId<ReciteDialogueAsset>) -> Option<&CachedRevision> {
        self.0.get(&id)
    }
}

pub(crate) fn refresh_assets(
    mut events: MessageReader<AssetEvent<ReciteDialogueAsset>>,
    assets: bevy_ecs::system::Res<Assets<ReciteDialogueAsset>>,
    mut cache: bevy_ecs::system::ResMut<AssetCache>,
    mut statuses: MessageWriter<ReciteAssetStatus>,
) {
    for event in events.read() {
        let id = match *event {
            AssetEvent::Added { id }
            | AssetEvent::Modified { id }
            | AssetEvent::LoadedWithDependencies { id } => id,
            AssetEvent::Removed { id } | AssetEvent::Unused { id } => {
                cache.0.remove(&id);
                statuses.write(ReciteAssetStatus {
                    handle_id: id,
                    import: ReciteAssetImport::Removed,
                    source_schema_freshness: ReciteAssetFreshness::Unavailable,
                    path: None,
                });
                continue;
            }
        };
        if let Some(asset) = assets.get(id) {
            let revision = asset.revision.clone();
            cache.0.insert(
                id,
                CachedRevision {
                    dialogue: asset.dialogue.clone(),
                    revision: revision.clone(),
                },
            );
            statuses.write(ReciteAssetStatus {
                handle_id: id,
                import: ReciteAssetImport::Accepted { revision },
                source_schema_freshness: ReciteAssetFreshness::Unavailable,
                path: None,
            });
        }
    }
}

pub(crate) fn failed_assets(
    mut events: MessageReader<AssetLoadFailedEvent<ReciteDialogueAsset>>,
    cache: bevy_ecs::system::Res<AssetCache>,
    mut statuses: MessageWriter<ReciteAssetStatus>,
) {
    for event in events.read() {
        statuses.write(ReciteAssetStatus {
            handle_id: event.id,
            import: ReciteAssetImport::Rejected {
                error: AdapterError::with_detail(
                    AdapterErrorKind::AssetLoadOrDecode,
                    event.error.to_string(),
                ),
                retained: cache.get(event.id).map(|asset| asset.revision.clone()),
            },
            source_schema_freshness: ReciteAssetFreshness::Unavailable,
            path: Some(event.path.to_string()),
        });
    }
}
