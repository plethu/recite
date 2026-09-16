//! Structural scene connections for navigation, independent of runtime traversal.
use crate::{PassageKind, ScriptBlock, ScriptEntry};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneLink {
    pub origin: String,
    pub destination: String,
    pub label: String,
    pub conditional: bool,
    /// Back edge in deterministic source-order depth-first traversal, independent of placement.
    pub returning: bool,
}

/// Source order is retained, including multiple choices reaching the same beat.
pub fn scene_links(blocks: &[ScriptBlock]) -> Vec<SceneLink> {
    let mut links = Vec::new();
    for block in blocks {
        collect(&block.id, &block.entries, false, &mut links);
    }
    classify_returns(blocks, &mut links);
    links
}

fn collect(origin: &str, entries: &[ScriptEntry], conditional: bool, links: &mut Vec<SceneLink>) {
    for entry in entries {
        let (destination, label) = match entry {
            ScriptEntry::Passage(passage) => match &passage.kind {
                PassageKind::Choice {
                    destination: Some(destination),
                } => (destination, passage.text.clone()),
                _ => continue,
            },
            ScriptEntry::Jump(destination) => (destination, "Continue".into()),
            ScriptEntry::Group { entries, .. } => {
                collect(origin, entries, true, links);
                continue;
            }
            _ => continue,
        };
        links.push(SceneLink {
            origin: origin.into(),
            destination: destination.clone(),
            label,
            conditional,
            returning: false,
        });
    }
}

// Iterative traversal avoids recursive stack growth for large scenes.
fn classify_returns(blocks: &[ScriptBlock], links: &mut [SceneLink]) {
    let ids: std::collections::BTreeSet<_> = blocks.iter().map(|b| b.id.as_str()).collect();
    let mut adjacency = std::collections::BTreeMap::<String, Vec<usize>>::new();
    for (index, link) in links.iter().enumerate() {
        adjacency
            .entry(link.origin.clone())
            .or_default()
            .push(index);
    }
    let mut visited = std::collections::BTreeSet::new();
    let mut active = std::collections::BTreeSet::new();
    for root in blocks
        .iter()
        .filter(|b| b.is_default)
        .chain(blocks.iter().filter(|b| !b.is_default))
    {
        if visited.contains(&root.id) {
            continue;
        }
        let mut stack = vec![(root.id.clone(), 0)];
        visited.insert(root.id.clone());
        active.insert(root.id.clone());
        while let Some((origin, next)) = stack.last_mut() {
            let edge = adjacency
                .get(origin)
                .and_then(|edges| edges.get(*next))
                .copied();
            let Some(index) = edge else {
                active.remove(origin);
                stack.pop();
                continue;
            };
            *next += 1;
            let target = links[index].destination.clone();
            if active.contains(&target) {
                links[index].returning = true;
            } else if ids.contains(target.as_str()) && visited.insert(target.clone()) {
                active.insert(target.clone());
                stack.push((target, 0));
            }
        }
    }
}
