use super::*;
use xilem::masonry::imaging::record::Scene;

#[test]
fn dot_screen_reuses_geometry_through_repaints_selection_and_ink_changes() {
    let mut cache = ScreenCache::default();
    let bounds = Rect::new(12., 12., 308., 35.);
    let screen = DotScreen::new(Color::BLACK, Some(bounds));
    let mut scene = Scene::new();
    cache.paint(&screen, &mut Painter::new(&mut scene));
    assert_ne!(scene, Scene::new());
    let original = scene.clone();
    let geometry = cache.path.elements().as_ptr();
    for _ in 0..180 {
        scene.clear();
        cache.paint(&screen, &mut Painter::new(&mut scene));
        assert_eq!(scene, original);
        assert_eq!(cache.path.elements().as_ptr(), geometry);
    }
    cache.paint(
        &DotScreen::new(Color::WHITE, Some(bounds)),
        &mut Painter::new(&mut scene),
    );
    cache.paint(
        &DotScreen::new(Color::BLACK, None),
        &mut Painter::new(&mut scene),
    );
    cache.paint(&screen, &mut Painter::new(&mut scene));
    assert_eq!(cache.path.elements().as_ptr(), geometry);
    assert!(cache.path.elements().len() < 2_000);
    cache.paint(
        &DotScreen::new(Color::BLACK, Some(bounds.inset(-5.))),
        &mut Painter::new(&mut scene),
    );
    assert_ne!(cache.path.elements().as_ptr(), geometry);
}
