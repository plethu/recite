//! A single row index; selection changes neither filtering nor scene expansion.
use super::{Caption, Row, SceneBranch};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct Cache {
    previous: Option<Inputs>,
    items: Arc<[Row]>,
}
struct Inputs {
    scenes: Vec<(String, bool)>,
    captions: Arc<[Caption]>,
    query: String,
    collapsed: Option<String>,
}
pub(super) struct Projection {
    pub items: Arc<[Row]>,
    // A hit leaves search matches untouched; Some(empty) clears old matches.
    pub matches: Option<Vec<String>>,
}

impl Cache {
    pub fn refresh(
        &mut self,
        scenes: &[SceneBranch],
        captions: &Arc<[Caption]>,
        query: &str,
        collapsed: Option<&str>,
    ) -> Projection {
        if self.previous.as_ref().is_some_and(|old| {
            Arc::ptr_eq(&old.captions, captions)
                && old.query == query
                && old.collapsed.as_deref() == collapsed
                && old.scenes.len() == scenes.len()
                && old
                    .scenes
                    .iter()
                    .zip(scenes)
                    .all(|((caption, active), scene)| {
                        caption == &scene.caption && *active == scene.active
                    })
        }) {
            return Projection {
                items: self.items.clone(),
                matches: None,
            };
        }
        let needle = query.to_lowercase();
        let items = build(scenes, captions, &needle, collapsed);
        let matches = if needle.is_empty() {
            Vec::new()
        } else {
            items
                .iter()
                .filter_map(|row| match row {
                    Row::Beat { caption_index } => Some(captions[*caption_index].id.clone()),
                    _ => None,
                })
                .collect()
        };
        self.previous = Some(Inputs {
            scenes: scenes
                .iter()
                .map(|scene| (scene.caption.clone(), scene.active))
                .collect(),
            captions: captions.clone(),
            query: query.to_owned(),
            collapsed: collapsed.map(str::to_owned),
        });
        self.items = items;
        Projection {
            items: self.items.clone(),
            matches: Some(matches),
        }
    }
}

fn build(
    scenes: &[SceneBranch],
    captions: &[Caption],
    needle: &str,
    collapsed: Option<&str>,
) -> Arc<[Row]> {
    let mut rows = Vec::new();
    for (scene_index, scene) in scenes.iter().enumerate() {
        let expanded = scene.active && (!needle.is_empty() || collapsed != Some(&scene.caption));
        let scene_matches = !needle.is_empty() && scene.caption.to_lowercase().contains(needle);
        let mut beats = Vec::new();
        if expanded {
            if needle.is_empty() {
                beats.reserve(captions.len());
            }
            for (caption_index, caption) in captions.iter().enumerate() {
                if needle.is_empty() || caption.folded.contains(needle) || scene_matches {
                    beats.push(Row::Beat { caption_index });
                }
            }
        }
        if needle.is_empty() || scene_matches || !beats.is_empty() {
            rows.push(Row::Scene {
                scene_index,
                expanded,
            });
            rows.extend(beats);
        }
    }
    rows.into()
}

#[cfg(test)]
mod tests;
