use bevy_app::{App, Plugin, Update};
use bevy_asset::AssetApp;
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};

use crate::asset::{AssetCache, failed_assets, refresh_assets};
use crate::context::{ReciteCatalog, ReciteConditions, ReciteInterpolation};
use crate::session::{RequestCursor, process_requests};
use crate::{
    ReciteAssetStatus, ReciteDialogueAsset, ReciteDialogueLoader, ReciteOutput, ReciteOwner,
    ReciteRequest,
};

/// Public schedule boundaries. Prepare game state before `ProcessRequests`,
/// then read ordered output after it in `Output`.
#[derive(SystemSet, Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReciteSet {
    PrepareContext,
    ProcessRequests,
    Output,
}

/// Bevy asset, message, and single-session resource integration.
pub struct RecitePlugin;

impl Plugin for RecitePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ReciteDialogueAsset>()
            .init_asset_loader::<ReciteDialogueLoader>()
            .init_resource::<AssetCache>()
            .init_resource::<ReciteOwner>()
            .init_resource::<ReciteConditions>()
            .init_resource::<ReciteCatalog>()
            .init_resource::<ReciteInterpolation>()
            .init_resource::<RequestCursor>()
            .add_message::<ReciteRequest>()
            .add_message::<ReciteOutput>()
            .add_message::<ReciteAssetStatus>()
            .configure_sets(
                Update,
                (
                    ReciteSet::PrepareContext,
                    ReciteSet::ProcessRequests,
                    ReciteSet::Output,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (refresh_assets, failed_assets)
                    .chain()
                    .in_set(ReciteSet::PrepareContext),
            )
            .add_systems(Update, process_requests.in_set(ReciteSet::ProcessRequests));
    }
}
