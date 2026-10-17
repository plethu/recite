use super::*;
use recite_writer_model::{Document, ScriptEntry};

fn blocks() -> Result<Arc<[ScriptBlock]>, Box<dyn std::error::Error>> {
    Ok(Document::new(
        ":: start default\n> one@00000000000000000001\n  Hello.\n? choose@00000000000000000002\n  Go.\n  -> finish\n:: finish\n> end@00000000000000000003\n  Done.\n-> END\n",
    )?
    .script_snapshot()?)
}

fn compare_fresh(
    previous: &Topology,
    blocks: Vec<ScriptBlock>,
    saved: &placement::Positions,
    reused: bool,
) -> Topology {
    let blocks: Arc<[ScriptBlock]> = blocks.into();
    let refreshed = Topology::refresh(blocks.clone(), saved, Some(previous));
    assert!(refreshed == Topology::new(blocks, saved));
    assert_eq!(Arc::ptr_eq(&refreshed.geometry, &previous.geometry), reused);
    refreshed
}

#[test]
fn prose_and_choice_labels_refresh_without_rebuilding_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let blocks = blocks()?;
    let saved = placement::Positions::new();
    let old = Topology::new(blocks.clone(), &saved);
    let mut changed = blocks.to_vec();
    if let ScriptEntry::Passage(p) = &mut changed[0].entries[0] {
        p.text = "Different café prose.".into();
    }
    if let ScriptEntry::Passage(p) = &mut changed[0].entries[1] {
        p.text = "A new choice label.".into();
    }
    let refreshed = compare_fresh(&old, changed, &saved, true);
    assert_eq!(refreshed.links[0].label, "A new choice label.");
    assert_eq!(old.links[0].label, "Go.");
    let mut conditional = refreshed.blocks.to_vec();
    let choice = conditional[0].entries.remove(1);
    conditional[0].entries.push(ScriptEntry::Group {
        heading: "if".into(),
        entries: vec![choice],
    });
    let refreshed = compare_fresh(&refreshed, conditional, &saved, true);
    assert!(refreshed.links[0].conditional);
    Ok(())
}

#[test]
fn ordered_blocks_and_all_connection_slots_invalidate_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let blocks = blocks()?;
    let saved = placement::Positions::new();
    let old = Topology::new(blocks.clone(), &saved);
    let mut variants = Vec::new();
    let mut changed = blocks.to_vec();
    changed[0].is_default = false;
    variants.push(changed);
    let mut changed = blocks.to_vec();
    changed.reverse();
    variants.push(changed);
    let mut changed = blocks.to_vec();
    changed[1].id = "renamed".into();
    variants.push(changed);
    for target in ["END", "missing", "start"] {
        let mut changed = blocks.to_vec();
        changed[0].entries.push(ScriptEntry::Jump(target.into()));
        variants.push(changed);
    }
    let mut changed = blocks.to_vec();
    let duplicate_choice = changed[0].entries[1].clone();
    changed[0].entries.push(duplicate_choice);
    variants.push(changed);
    let mut changed = blocks.to_vec();
    changed[0].entries.remove(1);
    variants.push(changed);
    for changed in variants {
        compare_fresh(&old, changed, &saved, false);
    }
    let mut changed = blocks.to_vec();
    if let ScriptEntry::Passage(p) = &mut changed[0].entries[1] {
        p.kind = recite_writer_model::PassageKind::Choice {
            destination: Some("missing".into()),
        };
    }
    compare_fresh(&old, changed, &saved, false);
    let mut ordered = blocks.to_vec();
    ordered[0].entries.push(ScriptEntry::Jump("END".into()));
    let old_order = Topology::new(ordered.clone().into(), &saved);
    ordered[0].entries.swap(1, 2);
    compare_fresh(&old_order, ordered, &saved, false);
    let mut duplicate = blocks.to_vec();
    duplicate[1].id = duplicate[0].id.clone();
    let old = Topology::new(duplicate.clone().into(), &saved);
    compare_fresh(&old, duplicate, &saved, true);
    Ok(())
}

#[test]
fn saved_placement_and_its_passage_key_invalidate_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let blocks = blocks()?;
    let key = placement::card_key(&blocks[0]);
    let saved = placement::Positions::from([(key, (80., 90.))]);
    let old = Topology::new(blocks.clone(), &saved);
    let mut moved = saved.clone();
    moved.insert(placement::card_key(&blocks[0]), (180., 190.));
    compare_fresh(&old, blocks.to_vec(), &moved, false);
    compare_fresh(&old, blocks.to_vec(), &Default::default(), false);
    let mut changed = blocks.to_vec();
    if let ScriptEntry::Passage(p) = &mut changed[0].entries[0] {
        p.id = "ffffffffffffffffffff".into();
    }
    compare_fresh(&old, changed, &saved, false);
    compare_fresh(&old, blocks.to_vec(), &saved, true);
    Ok(())
}

#[test]
fn empty_graph_and_signed_zero_placement_match_fresh_geometry()
-> Result<(), Box<dyn std::error::Error>> {
    let empty = Topology::new(Arc::from([]), &Default::default());
    compare_fresh(&empty, Vec::new(), &Default::default(), true);
    let blocks = blocks()?;
    let key = placement::card_key(&blocks[0]);
    let saved = placement::Positions::from([(key.clone(), (0., 0.))]);
    let old = Topology::new(blocks.clone(), &saved);
    let signed = placement::Positions::from([(key, (-0., -0.))]);
    let refreshed = compare_fresh(&old, blocks.to_vec(), &signed, false);
    assert_eq!(refreshed.nodes[0].x.to_bits(), (-0.0_f32).to_bits());
    // The entry marker intentionally keeps the pre-placement layout coordinates.
    assert_eq!(refreshed.entry, old.entry);
    Ok(())
}
