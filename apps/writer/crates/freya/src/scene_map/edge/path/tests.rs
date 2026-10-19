use super::*;
use skia_safe::{
    Color, PaintStyle,
    paint::{Cap, Join},
    surfaces,
};

#[test]
fn clipped_work_is_bounded_by_the_viewport_in_both_directions() {
    let bounds = Rect::from_xywh(10., 20., 300., 200.);
    for distance in [1000., 1_000_000.] {
        for (from, to) in [
            ((-distance, 50.), (distance, 50.)),
            ((distance, 50.), (-distance, 50.)),
            ((50., -distance), (50., distance)),
            ((50., distance), (50., -distance)),
        ] {
            let line = Line {
                from: from.into(),
                to: to.into(),
                distance: 17.,
            };
            let clipped = line.clipped(bounds).expect("visible segment");
            let length =
                (clipped.to.x - clipped.from.x).abs() + (clipped.to.y - clipped.from.y).abs();
            assert!(length <= bounds.width().max(bounds.height()));
            assert!(clipped.distance >= 17.);
            assert_eq!(
                clipped.distance,
                17. + (clipped.from.x - line.from.x).abs() + (clipped.from.y - line.from.y).abs()
            );
        }
    }
    assert!(
        Line {
            from: (-1000., -50.).into(),
            to: (1000., -50.).into(),
            distance: 0.
        }
        .clipped(bounds)
        .is_none()
    );
}

#[test]
fn actual_dashed_path_extent_and_size_do_not_grow_with_offscreen_route_length() {
    let bounds = Rect::from_xywh(0., 0., 300., 200.);
    for distance in [1_000., 1_000_000.] {
        let route = EdgePath::parse(&format!(
            "M-{distance} 43 L50 43 L50 {distance} L200 {distance} L200 150 L-{distance} 150"
        ))
        .expect("orthogonal route");
        let lines = route.lines.as_ref().expect("bounded orthogonal path");
        for period in [4., 7., 13.] {
            let visible = clipped_path(lines, bounds, period).expect("representable clipped path");
            assert!(visible.points().len() <= lines.len() * 2);
            for point in visible.points() {
                assert!(point.x >= bounds.left - period && point.x <= bounds.right + period);
                assert!(point.y >= bounds.top - period && point.y <= bounds.bottom + period);
            }
        }
    }
}

fn raster(path: &EdgePath, dash: &[f32], zoom: f32, pan: (f32, f32), optimized: bool) -> Vec<u8> {
    let mut surface = surfaces::raster_n32_premul((256, 256)).expect("raster surface");
    let canvas = surface.canvas();
    canvas.clear(Color::WHITE);
    canvas.translate(pan);
    canvas.scale((zoom, zoom));
    let mut paint = Paint::default();
    paint.set_color(Color::BLACK);
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(2. / zoom.max(0.25));
    paint.set_stroke_cap(Cap::Round);
    paint.set_stroke_join(Join::Round);
    if optimized {
        path.draw(canvas, &mut paint, dash);
    } else {
        paint.set_path_effect(PathEffect::dash(dash, 0.));
        canvas.draw_path(&path.path, &paint);
    }
    surface
        .peek_pixels()
        .expect("raster pixels")
        .bytes()
        .expect("pixel bytes")
        .to_vec()
}

#[test]
fn clipped_dash_raster_matches_continuous_route_at_corners_and_viewport_edges() {
    for source in [
        "M-1000 43 L50 43 L50 150 L200 150 L200 -1000",
        "M200 -1000 L200 150 L50 150 L50 43 L-1000 43",
        "M30 30 L100 30 L100 100 L180 100 L180 200",
        "M20 20 C20 100 200 100 200 200",
        "M20 20 L100 20 M100 20 L100 200",
        "M20 20 L100 20 L100 20 L100 200",
        "M20 30 L1000 30 L1000 180 L20 180 L20 30",
        "M-1000000 43 L50 43 L50 150 L200 150 L200 -1000000",
    ] {
        let path = EdgePath::parse(source).expect("test route");
        for dash in [&[4., 3.][..], &[6., 3., 1., 3.], &[1., 3.], &[]] {
            for (zoom, pan) in [
                (0.001, (0., 0.)),
                (0.01, (13.25, -8.75)),
                (0.375, (13.25, -8.75)),
                (0.5, (0., 0.)),
                (1., (0., 0.)),
                (2., (0., 0.)),
            ] {
                let original = raster(&path, dash, zoom, pan, false);
                let clipped = raster(&path, dash, zoom, pan, true);
                let different = original
                    .iter()
                    .zip(&clipped)
                    .filter(|(a, b)| a.abs_diff(**b) > 2)
                    .count();
                let error: usize = original
                    .iter()
                    .zip(&clipped)
                    .map(|(&a, &b)| usize::from(a.abs_diff(b)))
                    .sum();
                // Ignore two levels of 8-bit raster quantization in the pixel count.
                // Clipped float endpoints can alter antialiasing at a few caps;
                // a shifted dash pattern or changed join exceeds both bounds.
                assert!(
                    different <= 24 && error <= 512,
                    "different={different}, error={error}, source={source}, dash={dash:?}, zoom={zoom}"
                );
            }
        }
    }
}
