//! Versioned personal map placement. Dialogue files never contain viewport geometry.
use recite_config::UserStateFile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) type Positions = BTreeMap<String, (f32, f32)>;

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layouts {
    version: u32,
    scenes: BTreeMap<String, Positions>,
}
#[derive(Debug, thiserror::Error)]
pub(super) enum LayoutError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(
        "unsupported map layout version or invalid coordinates; saved layout was left untouched"
    )]
    Invalid,
}
impl Layouts {
    fn parse(source: Option<&str>) -> Result<Self, LayoutError> {
        let value: Self = source
            .map(serde_json::from_str)
            .transpose()?
            .unwrap_or(Self {
                version: 1,
                ..Self::default()
            });
        if value.version != 1
            || value
                .scenes
                .values()
                .flat_map(|p| p.values())
                .any(|(x, y)| {
                    !x.is_finite()
                        || !y.is_finite()
                        || *x < 0.
                        || *y < 0.
                        || *x > 100_000.
                        || *y > 100_000.
                })
        {
            return Err(LayoutError::Invalid);
        }
        Ok(value)
    }
}
pub(super) fn load(file: &UserStateFile, scene: &str) -> Result<Positions, String> {
    let source = file.load().map_err(|e| e.to_string())?;
    Ok(Layouts::parse(source.as_deref())
        .map_err(|e| e.to_string())?
        .scenes
        .remove(scene)
        .unwrap_or_default())
}
pub(super) fn save(file: &UserStateFile, scene: &str, positions: &Positions) -> Result<(), String> {
    file.update(|source| {
        let mut layouts = Layouts::parse(source)?;
        if positions.is_empty() {
            layouts.scenes.remove(scene);
        } else {
            layouts.scenes.insert(scene.to_owned(), positions.clone());
        }
        let next = serde_json::to_string_pretty(&layouts)?;
        Layouts::parse(Some(&next))?;
        Ok::<_, LayoutError>(next)
    })
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;

/// In-memory placement is retained even if persistence fails; the error stays visible.
pub(super) struct Arrangement {
    scenes: BTreeMap<String, Positions>,
    store: Option<Result<UserStateFile, String>>,
    pub error: Option<String>,
}
impl Arrangement {
    pub fn new(store: Option<Result<UserStateFile, String>>) -> Self {
        Self {
            scenes: BTreeMap::new(),
            error: store.as_ref().and_then(|s| s.as_ref().err()).cloned(),
            store,
        }
    }
    pub fn ensure_scene(&mut self, scene: &str) {
        if !self.scenes.contains_key(scene) {
            let positions = match &self.store {
                Some(Ok(file)) => match load(file, scene) {
                    Ok(positions) => positions,
                    Err(error) => {
                        self.error = Some(error);
                        Positions::new()
                    }
                },
                _ => Positions::new(),
            };
            self.scenes.insert(scene.into(), positions);
        }
    }
    pub fn positions(&self, scene: &str) -> &Positions {
        &self.scenes[scene]
    }
    pub fn move_card(&mut self, scene: &str, card: &str, x: f32, y: f32) {
        self.scenes
            .entry(scene.into())
            .or_default()
            .insert(card.into(), (x.clamp(8., 100_000.), y.clamp(12., 100_000.)));
    }
    pub fn reset(&mut self, scene: &str) {
        self.scenes.insert(scene.into(), Positions::new());
        self.persist(scene);
    }
    pub fn persist(&mut self, scene: &str) {
        if let Some(Ok(file)) = &self.store {
            self.error = save(file, scene, &self.scenes[scene]).err();
        }
    }
}

/// Passage IDs survive beat renaming; effects-only beats fall back to the block ID.
pub(super) fn card_key(block: &recite_writer_model::ScriptBlock) -> String {
    fn first(entries: &[recite_writer_model::ScriptEntry]) -> Option<String> {
        entries.iter().find_map(|entry| match entry {
            recite_writer_model::ScriptEntry::Passage(p) => Some(format!("passage:{}", p.id)),
            recite_writer_model::ScriptEntry::Group { entries, .. } => first(entries),
            _ => None,
        })
    }
    first(&block.entries).unwrap_or_else(|| format!("block:{}", block.id))
}
