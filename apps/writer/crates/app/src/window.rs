//! How the window's width is shared: compact chrome below the minimum comfortable window, and
//! the scene list's width, narrowed and then hidden as the window shrinks.
use super::Writer;

/// Below this width, before the UI scale, chrome compacts and the scene list hides.
pub const COMPACT_WINDOW: f64 = 900.;
/// Below this width the scene list narrows to [`NARROW_SIDEBAR`].
const WIDE_WINDOW: f64 = 1100.;
/// The scene list's width in narrow windows.
pub const NARROW_SIDEBAR: f64 = 192.;
/// The workspace's padding either side, and the least content width it keeps.
const WORKSPACE_PAD: f64 = 16.;
const MINIMUM_CONTENT: f64 = 280.;

impl Writer {
    pub fn compact_chrome(&self) -> bool {
        self.viewport.width < COMPACT_WINDOW * f64::from(self.ui_scale())
    }
    pub fn content_width(&self) -> f64 {
        (self.viewport.width - self.navigation_width() - 2. * WORKSPACE_PAD).max(MINIMUM_CONTENT)
    }
    /// The scene list's width: hidden in focus mode, by choice or below the minimum window, and
    /// narrowed to 192px with ellipsis below 1100px so the sheet keeps its measure.
    pub fn navigation_width(&self) -> f64 {
        let scale = f64::from(self.ui_scale());
        if self.attention.is_focused()
            || self.attention.sidebar_hidden()
            || self.viewport.width < COMPACT_WINDOW * scale
        {
            return 0.;
        }
        if self.viewport.width < WIDE_WINDOW * scale {
            self.drawer_width().min(NARROW_SIDEBAR)
        } else {
            self.drawer_width()
        }
    }
    pub fn drawer_width(&self) -> f64 {
        f64::from(
            self.config
                .writer
                .presentation
                .value(recite_config::WriterPresentationField::DrawerWidth),
        )
    }
}
