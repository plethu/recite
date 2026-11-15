use super::*;

#[test]
fn the_screen_is_one_small_repeating_tile_in_the_inks_own_alpha() {
    let ink = Color::from_rgba8(243, 233, 223, 26);
    for texture in [Texture::Dots, Texture::Hatch] {
        let brush = screen(texture, ink);
        assert_eq!((brush.image.width, brush.image.height), (PITCH, PITCH));
        assert_eq!(brush.sampler.x_extend, Extend::Repeat);
        let pixels = brush.image.data.data();
        assert!(
            pixels
                .chunks_exact(4)
                .all(|pixel| pixel[..3] == [243, 233, 223])
        );
        // Nothing in the tile is denser than the ink itself.
        assert!(pixels.chunks_exact(4).all(|pixel| pixel[3] <= 26));
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] == 26));
    }
    // Dots and hatch differ in shape, not in ink.
    assert_ne!(coverage(Texture::Dots), coverage(Texture::Hatch));
}
