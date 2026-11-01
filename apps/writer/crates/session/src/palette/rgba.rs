//! Unpremultiplied eight-bit sRGB channels; conversion belongs to the renderer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}
impl Rgba {
    pub const TRANSPARENT: Self = Self::from_argb(0, 0, 0, 0);
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }
    pub const fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a }
    }
    pub fn mix(start: Self, end: Self, amount: f32) -> Self {
        let amount = amount.clamp(0., 1.);
        let mix =
            |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * amount).round() as u8;
        Self::from_argb(
            mix(start.a, end.a),
            mix(start.r, end.r),
            mix(start.g, end.g),
            mix(start.b, end.b),
        )
    }
    pub fn monochrome(self) -> Self {
        let linear = |channel: u8| {
            let v = f64::from(channel) / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        let luminance = 0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b);
        let value = if luminance <= 0.0031308 {
            luminance * 12.92
        } else {
            1.055 * luminance.powf(1. / 2.4) - 0.055
        };
        let gray = (value * 255.).round() as u8;
        Self::from_argb(self.a, gray, gray, gray)
    }
}
