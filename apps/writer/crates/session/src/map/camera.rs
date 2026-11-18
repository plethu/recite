//! World-to-viewport camera independent of native event delivery.
#[derive(Clone, Copy, PartialEq)]
pub enum Framing {
    Entry,
    Fit,
    Free,
}

/// Where the map's world sits in the viewport. `zoom` is world units to screen pixels; the UI
/// scale is the zoom's 100%, so the map zooms with the interface as the sheet does.
#[derive(Clone, Copy, PartialEq)]
pub struct Camera {
    pub zoom: f32,
    pub x: f32,
    pub y: f32,
    pub framing: Framing,
    /// The UI scale: the zoom shown as 100%.
    pub scale: f32,
}
impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 1.,
            x: 0.,
            y: 0.,
            framing: Framing::Entry,
            scale: 1.,
        }
    }
}
impl Camera {
    /// The zoom relative to the UI scale: 1 at 100%, whatever the interface's size.
    pub fn relative(&self) -> f32 {
        self.zoom / self.scale
    }
    /// Zoom with the interface: the same relative zoom at `scale`, about the viewport's origin.
    pub fn rescale(&mut self, scale: f32) {
        let ratio = scale / self.scale;
        self.zoom *= ratio;
        self.x *= ratio;
        self.y *= ratio;
        self.scale = scale;
    }
    pub fn frame_entry(&mut self, viewport: (f32, f32), entry: (f32, f32, f32)) {
        self.zoom = self.scale;
        self.x = (viewport.0 - entry.2 * self.zoom) / 2. - entry.0 * self.zoom;
        self.y = 32. - entry.1 * self.zoom;
        self.framing = Framing::Entry;
    }

    pub fn fit(&mut self, viewport: (f32, f32), bounds: (f32, f32, f32, f32)) {
        let (left, top, width, height) = bounds;
        self.zoom = ((viewport.0 - 32.) / width)
            .min((viewport.1 - 32.) / height)
            .clamp(0.001, self.scale);
        self.x = (viewport.0 - width * self.zoom) / 2. - left * self.zoom;
        self.y = 16. - top * self.zoom;
        self.framing = Framing::Fit;
    }
    pub fn zoom_to(&mut self, zoom: f32, viewport: (f32, f32)) {
        self.zoom_at(zoom, (viewport.0 / 2., viewport.1 / 2.));
    }
    pub fn zoom_at(&mut self, zoom: f32, anchor: (f32, f32)) {
        let zoom = zoom.clamp(0.001, 2. * self.scale);
        let ratio = zoom / self.zoom;
        self.x = anchor.0 - (anchor.0 - self.x) * ratio;
        self.y = anchor.1 - (anchor.1 - self.y) * ratio;
        self.zoom = zoom;
        self.framing = Framing::Free;
    }
}

#[cfg(test)]
mod tests;
