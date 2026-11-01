use super::*;
use std::f64::consts::PI;
use xilem::masonry::peniko::color::palette::css::BLACK;

fn grid(step: f64, x: f64) -> Grid {
    Grid {
        step,
        origin: Point::new(x, 0.),
        ink: BLACK,
    }
}
fn ink(tile: &Tile) -> f64 {
    let Brush::Image(brush) = &tile.brush else {
        panic!("the grid tile is an image");
    };
    brush
        .image
        .data
        .data()
        .chunks(4)
        .map(|texel| f64::from(texel[3]) / 255.)
        .sum()
}

#[test]
fn a_tile_holds_one_dot_of_its_geometric_area_at_each_device_scale() {
    for scale in [1., 2.] {
        let tile = Tile::new(grid(24., 0.), scale);
        assert_eq!(tile.pixels, (24. * scale) as u32);
        let area = PI * (RADIUS * scale).powi(2);
        assert!(
            (ink(&tile) - area).abs() < area * 0.05,
            "{scale}x: {} vs {area}",
            ink(&tile)
        );
    }
}

#[test]
fn panning_reuses_the_tile_and_zoom_or_scale_rebuilds_it() {
    let tile = Tile::new(grid(24., 0.), 1.);
    assert!(
        tile.serves(grid(24., 513.25), 1.),
        "camera position never changes the tile"
    );
    assert!(!tile.serves(grid(16.5, 0.), 1.));
    assert!(!tile.serves(grid(24., 0.), 2.));
}
