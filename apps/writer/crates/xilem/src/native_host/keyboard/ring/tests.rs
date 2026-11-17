use super::*;

#[test]
fn a_control_at_the_edge_pans_until_its_ring_shows() {
    let viewport = Rect::new(0., 100., 400., 300.);
    // Inside with room for its ring: nothing moves.
    assert_eq!(ring_pan(viewport, Rect::new(10., 150., 200., 180.)), 0.);
    // Flush against the bottom: down by the ring's reach.
    assert_eq!(ring_pan(viewport, Rect::new(10., 270., 200., 300.)), RING);
    // Partly below: down until the ring's foot shows.
    assert_eq!(
        ring_pan(viewport, Rect::new(10., 280., 200., 310.)),
        10. + RING
    );
    // Above: up until the ring's head shows.
    assert_eq!(
        ring_pan(viewport, Rect::new(10., 95., 200., 125.)),
        -5. - RING
    );
    // Taller than the view: its top and ring show first.
    assert_eq!(
        ring_pan(viewport, Rect::new(10., 150., 200., 450.)),
        50. - RING
    );
}
