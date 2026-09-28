use bevy_asset::{AssetId, Handle};
use bevy_ecs::message::{Message, MessageCursor, Messages};
use bevy_ecs::resource::Resource;
use bevy_ecs::world::World;
use recite_adapter::{AdapterError, AdapterErrorKind, SessionDriver, StartRequest};
use recite_core::{ChoiceId, EffectId, LocaleId};
use recite_runtime::{DialogueSessionOptions, EffectAck, LocaleResolution};

use crate::ReciteDialogueAsset;
use crate::asset::{AssetCache, CachedRevision, ReciteRevision};
use crate::context::{ReciteCatalog, ReciteInterpolation, WorldContext};
use crate::output::{ReciteOutput, ReciteOutputValue};

/// Ordered commands for the single resource-owned session.
#[derive(Message, Clone, Debug)]
pub enum ReciteRequest {
    Start {
        asset: Handle<ReciteDialogueAsset>,
        block_id: Option<String>,
        locale: Option<LocaleId>,
        variant: Option<String>,
    },
    SelectChoice(ChoiceId),
    AcknowledgeEffect {
        effect: EffectId,
        ack: EffectAck,
    },
    Snapshot,
    Restore {
        asset: Handle<ReciteDialogueAsset>,
        snapshot: Vec<u8>,
        variant: Option<String>,
    },
    End,
}

impl ReciteRequest {
    fn handle_id(&self) -> Option<AssetId<ReciteDialogueAsset>> {
        match self {
            Self::Start { asset, .. } | Self::Restore { asset, .. } => Some(asset.id()),
            _ => None,
        }
    }
}

/// One Bevy resource owns one Recite session and its immutable compiled
/// revision. Bevy entity IDs and handles never enter runtime snapshots.
#[derive(Resource, Debug, Default)]
pub struct ReciteOwner {
    driver: SessionDriver,
    variant: Option<String>,
    active_revision: Option<ReciteRevision>,
    next_sequence: u64,
}

impl ReciteOwner {
    #[must_use]
    pub fn has_active_session(&self) -> bool {
        self.driver.has_active_session()
    }

    #[must_use]
    pub fn active_revision(&self) -> Option<&ReciteRevision> {
        self.active_revision.as_ref()
    }
}

#[derive(Resource, Default)]
pub(crate) struct RequestCursor(MessageCursor<ReciteRequest>);

pub(crate) fn process_requests(world: &mut World) {
    let requests =
        world.resource_scope(|world, mut cursor: bevy_ecs::world::Mut<RequestCursor>| {
            cursor
                .0
                .read(world.resource::<Messages<ReciteRequest>>())
                .cloned()
                .collect::<Vec<_>>()
        });
    if requests.is_empty() {
        return;
    }
    world.resource_scope(|world, mut owner: bevy_ecs::world::Mut<ReciteOwner>| {
        for request in requests {
            let handle_id = request.handle_id();
            let value = match apply_request(world, &mut owner, request) {
                Ok(value) => value,
                Err(error) => ReciteOutputValue::Error { error, handle_id },
            };
            let sequence = owner.next_sequence;
            owner.next_sequence = owner.next_sequence.wrapping_add(1);
            world.write_message(ReciteOutput { sequence, value });
        }
    });
}

fn apply_request(
    world: &World,
    owner: &mut ReciteOwner,
    request: ReciteRequest,
) -> Result<ReciteOutputValue, AdapterError> {
    match request {
        ReciteRequest::Start {
            asset,
            block_id,
            locale,
            variant,
        } => {
            if owner.driver.has_active_session() {
                return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive));
            }
            let current = current_asset(world, asset.id())?;
            let mut options = DialogueSessionOptions::new();
            if let Some(locale) = locale {
                options = options.with_locale(locale);
            }
            let context = WorldContext { world };
            let resolution = resolution(world, variant.as_deref());
            let events = owner.driver.start(
                StartRequest {
                    asset: &current.dialogue,
                    block_id: block_id.as_deref(),
                    options,
                },
                &context,
                resolution,
            )?;
            owner.active_revision = Some(current.revision);
            owner.variant = variant;
            Ok(ReciteOutputValue::Dialogue(events))
        }
        ReciteRequest::SelectChoice(choice) => {
            let context = WorldContext { world };
            let resolution = resolution(world, owner.variant.as_deref());
            let events = owner.driver.select_choice(choice, &context, resolution)?;
            Ok(ReciteOutputValue::Dialogue(events))
        }
        ReciteRequest::AcknowledgeEffect { effect, ack } => {
            let context = WorldContext { world };
            let resolution = resolution(world, owner.variant.as_deref());
            let events = owner
                .driver
                .acknowledge_effect(effect, ack, &context, resolution)?;
            Ok(ReciteOutputValue::Dialogue(events))
        }
        ReciteRequest::Snapshot => Ok(ReciteOutputValue::Snapshot(owner.driver.snapshot()?)),
        ReciteRequest::Restore {
            asset,
            snapshot,
            variant,
        } => {
            if owner.driver.has_active_session() {
                return Err(AdapterError::new(AdapterErrorKind::SessionAlreadyActive));
            }
            let current = current_asset(world, asset.id())?;
            let context = WorldContext { world };
            let resolution = resolution(world, variant.as_deref());
            let events =
                owner
                    .driver
                    .restore(&current.dialogue, &snapshot, &context, resolution)?;
            owner.active_revision = Some(current.revision);
            owner.variant = variant;
            Ok(ReciteOutputValue::Dialogue(events))
        }
        ReciteRequest::End => {
            owner.driver.end_session()?;
            owner.active_revision = None;
            owner.variant = None;
            Ok(ReciteOutputValue::Ended)
        }
    }
}

fn current_asset(
    world: &World,
    id: AssetId<ReciteDialogueAsset>,
) -> Result<CachedRevision, AdapterError> {
    world
        .resource::<AssetCache>()
        .get(id)
        .cloned()
        .ok_or_else(|| {
            AdapterError::with_detail(
                AdapterErrorKind::AssetLoadOrDecode,
                format!("compiled asset handle {id:?} has no accepted revision"),
            )
        })
}

fn resolution<'a>(world: &'a World, variant: Option<&'a str>) -> LocaleResolution<'a> {
    let catalog = world.resource::<ReciteCatalog>();
    let values = world.resource::<ReciteInterpolation>();
    let mut resolution = LocaleResolution::new()
        .with_provider(&catalog.0)
        .with_values(&values.0);
    if let Some(variant) = variant {
        resolution = resolution.with_variant(variant);
    }
    resolution
}
