use recite_writer_model::{SceneLink, ScriptBlock, ScriptEntry, scene_links};
use std::collections::BTreeSet;

// A deliberately small recursive reference walks outgoing links by source order.
// Exhaustive fixtures stay at three vertices; the production traversal is iterative.
struct Reference<'a> {
    links: &'a [SceneLink],
    ids: BTreeSet<&'a str>,
    visited: BTreeSet<&'a str>,
    active: BTreeSet<&'a str>,
    returning: Vec<bool>,
}
impl<'a> Reference<'a> {
    fn visit(&mut self, origin: &'a str) {
        if !self.visited.insert(origin) {
            return;
        }
        self.active.insert(origin);
        for index in 0..self.links.len() {
            if self.links[index].origin != origin {
                continue;
            }
            let target = self.links[index].destination.as_str();
            if self.active.contains(target) {
                self.returning[index] = true;
            } else if self.ids.contains(target) {
                self.visit(target);
            }
        }
        self.active.remove(origin);
    }
}

#[test]
fn return_classification_matches_reference_for_every_three_vertex_graph() {
    for entry in 0..3 {
        for edges in 0..512 {
            let blocks: Vec<_> = (0..3)
                .map(|from| ScriptBlock {
                    id: format!("beat_{from}"),
                    is_default: from == entry,
                    entries: (0..3)
                        .filter(|to| edges & (1 << (from * 3 + to)) != 0)
                        .map(|to| ScriptEntry::Jump(format!("beat_{to}")))
                        .collect(),
                })
                .collect();
            let links = scene_links(&blocks);
            let mut reference = Reference {
                links: &links,
                ids: blocks.iter().map(|b| b.id.as_str()).collect(),
                visited: BTreeSet::new(),
                active: BTreeSet::new(),
                returning: vec![false; links.len()],
            };
            for root in blocks
                .iter()
                .filter(|b| b.is_default)
                .chain(blocks.iter().filter(|b| !b.is_default))
            {
                reference.visit(&root.id);
            }
            assert_eq!(
                links.iter().map(|l| l.returning).collect::<Vec<_>>(),
                reference.returning,
                "entry={entry}, edges={edges}"
            );
        }
    }
}

#[test]
fn an_earlier_duplicate_keeps_its_default_root_priority() {
    let block = |id: &str, is_default, target: &str| ScriptBlock {
        id: id.into(),
        is_default,
        entries: vec![ScriptEntry::Jump(target.into())],
    };
    let links = scene_links(&[
        block("b", false, "a"),
        block("a", true, "b"),
        block("a", false, "END"),
    ]);
    assert_eq!(
        links.iter().map(|l| l.returning).collect::<Vec<_>>(),
        [true, false, false]
    );
}
