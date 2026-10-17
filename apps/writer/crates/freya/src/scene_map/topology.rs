//! Prose refreshes content; ordered connections and saved placement own geometry.
use super::{
    layout::{self, HEIGHT, Node, WIDTH},
    placement,
    routing::{self, Route},
};
use recite_writer_model::{SceneLink, ScriptBlock};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Deref,
    sync::Arc,
};

#[derive(Clone, PartialEq)]
pub(super) struct Topology {
    pub blocks: Arc<[ScriptBlock]>,
    pub links: Vec<SceneLink>,
    geometry: Arc<Geometry>,
}

#[derive(PartialEq)]
pub(super) struct Geometry {
    positions: placement::Positions,
    pub ids: BTreeMap<String, usize>,
    pub adjacency: Vec<Vec<usize>>,
    pub incident: Vec<Vec<usize>>,
    pub nodes: Arc<[Node]>,
    pub routes: Vec<Option<Route>>,
    pub bounds: (f32, f32, f32, f32),
    pub entry: Option<(f32, f32, f32)>,
    pub endings: BTreeSet<String>,
}
impl Deref for Topology {
    type Target = Geometry;
    fn deref(&self) -> &Self::Target {
        &self.geometry
    }
}

impl Topology {
    pub fn new(blocks: Arc<[ScriptBlock]>, saved: &placement::Positions) -> Self {
        let links = recite_writer_model::scene_links(&blocks);
        let geometry = Arc::new(Geometry::new(&blocks, &links, saved));
        Self {
            blocks,
            links,
            geometry,
        }
    }

    pub fn refresh(
        blocks: Arc<[ScriptBlock]>,
        saved: &placement::Positions,
        previous: Option<&Self>,
    ) -> Self {
        if previous.is_none() {
            return Self::new(blocks, saved);
        }
        let links = recite_writer_model::scene_links(&blocks);
        let geometry = previous
            .filter(|old| old.same_shape(&blocks, &links, saved))
            .map_or_else(
                || Arc::new(Geometry::new(&blocks, &links, saved)),
                |old| old.geometry.clone(),
            );
        Self {
            blocks,
            links,
            geometry,
        }
    }

    fn same_shape(
        &self,
        blocks: &[ScriptBlock],
        links: &[SceneLink],
        saved: &placement::Positions,
    ) -> bool {
        self.positions.len() == saved.len()
            && self
                .positions
                .iter()
                .zip(saved)
                .all(|((old_key, old), (new_key, new))| {
                    old_key == new_key
                        && old.0.to_bits() == new.0.to_bits()
                        && old.1.to_bits() == new.1.to_bits()
                })
            && self.blocks.len() == blocks.len()
            && self.blocks.iter().zip(blocks).all(|(old, new)| {
                old.id == new.id
                    && old.is_default == new.is_default
                    && (saved.is_empty() || placement::card_key(old) == placement::card_key(new))
            })
            && self.links.len() == links.len()
            && self.links.iter().zip(links).all(|(old, new)| {
                old.origin == new.origin
                    && old.destination == new.destination
                    && old.returning == new.returning
            })
    }
}

impl Geometry {
    fn new(blocks: &[ScriptBlock], links: &[SceneLink], saved: &placement::Positions) -> Self {
        let ids: BTreeMap<_, _> = blocks
            .iter()
            .enumerate()
            .map(|(i, b)| (b.id.clone(), i))
            .collect();
        let mut adjacency = vec![Vec::new(); blocks.len()];
        let mut incident = vec![Vec::new(); blocks.len()];
        for (index, link) in links.iter().enumerate() {
            if let (Some(&from), Some(&to)) = (ids.get(&link.origin), ids.get(&link.destination)) {
                adjacency[from].push(to);
                adjacency[to].push(from);
                incident[from].push(index);
                incident[to].push(index);
            }
        }
        let mut nodes = layout::layout(blocks, links, WIDTH);
        let entry = nodes
            .get(blocks.iter().position(|b| b.is_default).unwrap_or(0))
            .map(|n| (n.x, n.y, n.width));
        for (block, node) in blocks.iter().zip(&mut nodes) {
            if let Some(&(x, y)) = saved.get(&placement::card_key(block)) {
                node.x = x;
                node.y = y;
            }
        }
        let routes = routing::routes(blocks, &nodes, links);
        let left = nodes
            .iter()
            .map(|n| n.x)
            .chain(routes.iter().flatten().map(|r| r.bounds.0))
            .fold(WIDTH, f32::min);
        let top = routes
            .iter()
            .flatten()
            .map(|r| r.bounds.1)
            .fold(0., f32::min);
        let right = nodes
            .iter()
            .map(|n| n.x + n.width)
            .chain(routes.iter().flatten().map(|r| r.bounds.0 + r.bounds.2))
            .fold(1., f32::max);
        let bottom = nodes
            .iter()
            .map(|n| n.y + HEIGHT)
            .chain(routes.iter().flatten().map(|r| r.bounds.1 + r.bounds.3))
            .fold(1., f32::max);
        let endings = links
            .iter()
            .filter(|l| l.destination == "END")
            .map(|l| l.origin.clone())
            .collect();
        Self {
            positions: saved.clone(),
            ids,
            adjacency,
            incident,
            nodes: nodes.into(),
            routes,
            bounds: (left, top, right - left, bottom - top),
            entry,
            endings,
        }
    }
}

pub(super) fn visible(
    bounds: (f32, f32, f32, f32),
    camera: super::camera::Camera,
    size: (f32, f32),
) -> bool {
    let (x, y, w, h) = bounds;
    let margin = 160.;
    x * camera.zoom + camera.x <= size.0 + margin
        && (x + w) * camera.zoom + camera.x >= -margin
        && y * camera.zoom + camera.y <= size.1 + margin
        && (y + h) * camera.zoom + camera.y >= -margin
}

#[cfg(test)]
mod tests;
