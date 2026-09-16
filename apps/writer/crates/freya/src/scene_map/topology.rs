//! Geometry is invalidated by source or saved placement, never by camera or hover.
use super::{
    layout::{self, HEIGHT, Node, WIDTH},
    placement,
    routing::{self, Route},
};
use recite_writer_model::{SceneLink, ScriptBlock};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone, PartialEq)]
pub(super) struct Topology {
    pub links: Vec<SceneLink>,
    pub ids: BTreeMap<String, usize>,
    pub adjacency: Vec<Vec<usize>>,
    pub incident: Vec<Vec<usize>>,
    pub nodes: Arc<[Node]>,
    pub routes: Vec<Option<Route>>,
    pub bounds: (f32, f32, f32, f32),
    pub entry: Option<(f32, f32, f32)>,
    pub endings: BTreeSet<String>,
}
impl Topology {
    pub fn new(blocks: &[ScriptBlock], saved: &placement::Positions) -> Self {
        let links = recite_writer_model::scene_links(blocks);
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
        let mut nodes = layout::layout(blocks, &links, WIDTH);
        let entry = nodes
            .get(blocks.iter().position(|b| b.is_default).unwrap_or(0))
            .map(|n| (n.x, n.y, n.width));
        for (block, node) in blocks.iter().zip(&mut nodes) {
            if let Some(&(x, y)) = saved.get(&placement::card_key(block)) {
                node.x = x;
                node.y = y;
            }
        }
        let routes = routing::routes(blocks, &nodes, &links);
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
            links,
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
