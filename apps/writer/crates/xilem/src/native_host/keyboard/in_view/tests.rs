use super::*;

#[test]
fn only_a_viewport_that_shrank_past_the_line_reveals_it() {
    let viewport = Rect::new(0., 100., 800., 400.);
    let inside = Rect::new(40., 300., 700., 330.);
    let below = Rect::new(40., 390., 700., 420.);
    assert!(shrank_away(Some(500.), viewport, below));
    assert!(!shrank_away(Some(500.), viewport, inside));
    // Growing, holding or first seeing the viewport moves nothing.
    assert!(!shrank_away(Some(300.), viewport, below));
    assert!(!shrank_away(Some(200.), viewport, below));
    assert!(!shrank_away(None, viewport, below));
}
