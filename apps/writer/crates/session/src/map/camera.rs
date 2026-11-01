//! World-to-viewport camera independent of native event delivery.
#[derive(Clone, Copy, PartialEq)]
pub enum Framing {
    Entry,
    Fit,
    Free,
}

#[derive(Clone, Copy, PartialEq)]
pub struct Camera {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
    pub framing: Framing,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.,
            x: 0.,
            y: 0.,
            framing: Framing::Entry,
        }
    }
}
impl Camera {
    pub fn frame_entry(&mut self, viewport: (f32, f32), entry: (f32, f32, f32)) {
        self.zoom = 1.;
        self.x = (viewport.0 - entry.2) / 2. - entry.0;
        self.y = 32. - entry.1;
        self.framing = Framing::Entry;
    }

    pub fn fit(&mut self, viewport: (f32, f32), bounds: (f32, f32, f32, f32)) {
        let (left, top, width, height) = bounds;
        self.zoom = ((viewport.0 - 32.) / width)
            .min((viewport.1 - 32.) / height)
            .clamp(0.001, 1.);
        self.x = (viewport.0 - width * self.zoom) / 2. - left * self.zoom;
        self.y = 16. - top * self.zoom;
        self.framing = Framing::Fit;
    }
    pub fn zoom_to(&mut self, zoom: f32, viewport: (f32, f32)) {
        self.zoom_at(zoom, (viewport.0 / 2., viewport.1 / 2.));
    }
    pub fn zoom_at(&mut self, zoom: f32, anchor: (f32, f32)) {
        let zoom = zoom.clamp(0.001, 2.);
        let ratio = zoom / self.zoom;
        self.x = anchor.0 - (anchor.0 - self.x) * ratio;
        self.y = anchor.1 - (anchor.1 - self.y) * ratio;
        self.zoom = zoom;
        self.framing = Framing::Free;
    }
}

#[cfg(test)]
mod tests;
