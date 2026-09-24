use super::*;

#[test]
fn returns_get_distinct_lanes_and_moving_cards_does_not_change_link_meaning()
-> Result<(), Box<dyn std::error::Error>> {
    let session = recite_writer_model::WRITER_EXAMPLES[0].open()?;
    let blocks = session.document().script()?;
    let links = recite_writer_model::scene_links(&blocks);
    let mut nodes = super::super::layout::layout(&blocks, &links, 1200.);
    let initial = routes(&blocks, &nodes, &links);
    let rails: Vec<_> = initial
        .iter()
        .zip(&links)
        .filter(|(_, l)| l.returning)
        .filter_map(|(r, _)| r.as_ref().map(|r| r.bounds.0 + r.bounds.2))
        .collect();
    assert!(rails.len() > 2);
    for (i, rail) in rails.iter().enumerate() {
        assert!(!rails[..i].contains(rail));
    }
    let meaning = links.iter().map(|l| l.returning).collect::<Vec<_>>();
    nodes[0].y = 2000.;
    let moved = routes(&blocks, &nodes, &links);
    assert_ne!(
        initial[0].as_ref().map(|r| &r.path),
        moved[0].as_ref().map(|r| &r.path)
    );
    assert_eq!(
        meaning,
        links.iter().map(|l| l.returning).collect::<Vec<_>>()
    );
    Ok(())
}

#[test]
fn reply_anchor_stays_on_the_connection_after_rearrangement()
-> Result<(), Box<dyn std::error::Error>> {
    let session = recite_writer_model::WRITER_EXAMPLES[1].open()?;
    let blocks = session.document().script()?;
    let links = recite_writer_model::scene_links(&blocks);
    let mut nodes = super::super::layout::layout(&blocks, &links, 1200.);
    let link = &links[0];
    let origin = blocks
        .iter()
        .position(|b| b.id == link.origin)
        .ok_or("origin")?;
    let destination = blocks
        .iter()
        .position(|b| b.id == link.destination)
        .ok_or("destination")?;
    let outgoing = links.iter().filter(|l| l.origin == link.origin).count();
    let incoming = links
        .iter()
        .filter(|l| l.destination == link.destination)
        .count();
    for dx in [0., 150.] {
        nodes[destination].x += dx;
        let result = routes(&blocks, &nodes, &links);
        let route = result[0].as_ref().ok_or("route")?;
        let start = nodes[origin].x + nodes[origin].width / (outgoing + 1) as f32;
        let end = nodes[destination].x + nodes[destination].width / (incoming + 1) as f32;
        assert_eq!(
            route.label,
            (
                (start + end) / 2.,
                (nodes[origin].y + HEIGHT + nodes[destination].y) / 2.
            )
        );
    }
    Ok(())
}
