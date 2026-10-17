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
    for index in return_edges(blocks, &links) {
        links[index].returning = true;
    }
    links
}

/// Beats with an authored END destination, including nested conditional entries.
/// Duplicate beat names share the same badge, as they do in `scene_links`.
pub fn scene_endings(blocks: &[ScriptBlock]) -> std::collections::BTreeSet<&str> {
    blocks
        .iter()
        .filter(|block| has_ending(&block.entries))
        .map(|block| block.id.as_str())
        .collect()
}

fn has_ending(entries: &[ScriptEntry]) -> bool {
    entries.iter().any(|entry| match entry {
        ScriptEntry::Jump(destination) => destination == "END",
        ScriptEntry::Passage(passage) => matches!(
            &passage.kind,
            PassageKind::Choice { destination: Some(destination) } if destination == "END"
        ),
        ScriptEntry::Group { entries, .. } => has_ending(entries),
        _ => false,
    })
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
#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    Unseen,
    Active,
    Finished,
}

fn return_edges(blocks: &[ScriptBlock], links: &[SceneLink]) -> Vec<usize> {
    let ids: std::collections::BTreeMap<_, _> = blocks
        .iter()
        .enumerate()
        .map(|(index, block)| (block.id.as_str(), index))
        .collect();
    let mut adjacency = vec![Vec::new(); blocks.len()];
    for (index, link) in links.iter().enumerate() {
        if let Some(&origin) = ids.get(link.origin.as_str()) {
            adjacency[origin].push(index);
        }
    }
    let targets: Vec<_> = links
        .iter()
        .map(|link| ids.get(link.destination.as_str()).copied())
        .collect();
    let mut state = vec![Visit::Unseen; blocks.len()];
    let mut returning = Vec::new();
    for root in blocks
        .iter()
        .filter(|b| b.is_default)
        .chain(blocks.iter().filter(|b| !b.is_default))
    {
        let root = ids[root.id.as_str()];
        if state[root] != Visit::Unseen {
            continue;
        }
        state[root] = Visit::Active;
        let mut stack = vec![(root, 0)];
        while let Some((origin, next)) = stack.last_mut() {
            let Some(&edge) = adjacency[*origin].get(*next) else {
                state[*origin] = Visit::Finished;
                stack.pop();
                continue;
            };
            *next += 1;
            let Some(target) = targets[edge] else {
                continue;
            };
            match state[target] {
                Visit::Active => returning.push(edge),
                Visit::Unseen => {
                    state[target] = Visit::Active;
                    stack.push((target, 0));
                }
                Visit::Finished => {}
            }
        }
    }
    returning
}
