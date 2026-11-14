use super::*;
use recite_writer_session::palette::GRAIN_PEAK;

#[test]
fn the_identity_tile_repeats_in_the_given_ink_without_exceeding_its_peak() {
    let (width, height, alpha) = coverage().expect("bundled grain tile");
    assert_eq!((*width, *height), (256, 256));
    assert_eq!(alpha.len(), 256 * 256);
    let peak = (GRAIN_PEAK * 255.).round() as u8;
    assert!(alpha.iter().all(|&alpha| alpha <= peak));
    assert!(alpha.iter().any(|&alpha| alpha > 0));
    let ink = Color::from_rgb8(64, 46, 51);
    let brush = grain(ink).expect("grain brush");
    assert_eq!(brush.sampler.x_extend, Extend::Repeat);
    assert_eq!(brush.sampler.y_extend, Extend::Repeat);
    assert_eq!((brush.image.width, brush.image.height), (256, 256));
    let pixels = brush.image.data.data();
    assert_eq!(&pixels[..3], &[64, 46, 51]);
    assert!(
        pixels
            .chunks_exact(4)
            .zip(alpha)
            .all(|(pixel, &alpha)| pixel[3] == alpha)
    );
}
