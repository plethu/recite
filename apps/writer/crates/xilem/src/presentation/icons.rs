//! Named vector faces keep compact native controls independent of text glyph metrics.
#[cfg(test)]
mod tests;
mod widget;
use widget::NativeIcon;
use xilem::{
    Pod, ViewCtx,
    core::{MessageCtx, MessageResult, Mut, View, ViewMarker},
    masonry::kurbo::{Affine, BezPath, Circle, Shape},
    peniko::Color,
};

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Icon {
    Back,
    Forward,
    Link,
    Undo,
    Redo,
    Plus,
    Minus,
    /// A disclosure chevron pointing right: collapsed.
    Collapsed,
    /// A disclosure chevron pointing down: expanded.
    Expanded,
    /// The identity's colon device: two squares turned on their corners, filled.
    Colon,
    /// A move grip: two columns of three dots, filled.
    Grip,
    /// A magnifier: the leading mark of a search field.
    Search,
    /// A pin: keep this beat as a reference beside the work.
    Pin,
    /// A pen nib: rename this object.
    Rename,
}
impl Icon {
    fn filled(self) -> bool {
        matches!(self, Self::Colon | Self::Grip)
    }
    fn path(self) -> BezPath {
        let mut path = BezPath::new();
        match self {
            Self::Back | Self::Forward => {
                let (outside, inside) = if matches!(self, Self::Back) {
                    (15., 8.)
                } else {
                    (9., 16.)
                };
                path.move_to((outside, 5.));
                path.line_to((inside, 12.));
                path.line_to((outside, 19.));
            }
            Self::Grip => {
                for x in [9., 15.] {
                    for y in [6., 12., 18.] {
                        path.extend(Circle::new((x, y), 1.8).path_elements(0.1));
                    }
                }
            }
            Self::Colon => {
                for centre in [8., 16.] {
                    path.move_to((12., centre - 3.2));
                    path.line_to((15.2, centre));
                    path.line_to((12., centre + 3.2));
                    path.line_to((8.8, centre));
                    path.close_path();
                }
            }
            Self::Collapsed => {
                path.move_to((9., 6.));
                path.line_to((15., 12.));
                path.line_to((9., 18.));
            }
            Self::Expanded => {
                path.move_to((6., 9.));
                path.line_to((12., 15.));
                path.line_to((18., 9.));
            }
            Self::Plus | Self::Minus => {
                path.move_to((5., 12.));
                path.line_to((19., 12.));
                if matches!(self, Self::Plus) {
                    path.move_to((12., 5.));
                    path.line_to((12., 19.));
                }
            }
            Self::Link => {
                path.move_to((10., 13.));
                path.line_to((14., 11.));
                path.move_to((9., 16.));
                path.line_to((7., 16.));
                path.curve_to((1.67, 16.), (1.67, 8.), (7., 8.));
                path.line_to((10., 8.));
                path.move_to((15., 8.));
                path.line_to((17., 8.));
                path.curve_to((22.33, 8.), (22.33, 16.), (17., 16.));
                path.line_to((14., 16.));
            }
            Self::Pin => {
                // A drawing pin seen from the side: head, collar and point.
                path.move_to((9., 4.));
                path.line_to((15., 4.));
                path.move_to((10., 4.));
                path.line_to((10., 11.));
                path.line_to((7., 14.));
                path.line_to((17., 14.));
                path.line_to((14., 11.));
                path.line_to((14., 4.));
                path.move_to((12., 14.));
                path.line_to((12., 20.));
            }
            Self::Rename => {
                // A pencil laid on a line of text.
                path.move_to((5., 17.));
                path.line_to((15., 7.));
                path.line_to((17., 9.));
                path.line_to((7., 19.));
                path.line_to((5., 19.));
                path.close_path();
                path.move_to((13., 9.));
                path.line_to((15., 11.));
                path.move_to((11., 19.));
                path.line_to((19., 19.));
            }
            Self::Search => {
                path.extend(Circle::new((10.5, 10.5), 5.5).path_elements(0.1));
                path.move_to((14.5, 14.5));
                path.line_to((19., 19.));
            }
            Self::Undo | Self::Redo => {
                path.move_to((8., 5.));
                path.line_to((3., 10.));
                path.line_to((8., 15.));
                path.move_to((3., 10.));
                path.line_to((14., 10.));
                path.curve_to((22., 10.), (22., 22.), (14., 22.));
                if matches!(self, Self::Redo) {
                    path.apply_affine(Affine::new([-1., 0., 0., 1., 24., 0.]));
                }
            }
        }
        path
    }
}
#[derive(Clone, PartialEq)]
pub(crate) struct Face {
    scale: f32,
    name: &'static str,
    icon: Icon,
    color: Color,
}
/// A decorative face named `name`, for marks that replace a glyph inside other content.
pub(crate) fn mark(scale: f32, name: &'static str, icon: Icon, color: Color) -> Face {
    face(scale, name, icon, color)
}

pub(super) fn face(scale: f32, name: &'static str, icon: Icon, color: Color) -> Face {
    Face {
        scale,
        name,
        icon,
        color,
    }
}
impl ViewMarker for Face {}
impl<S: 'static> View<S, (), ViewCtx> for Face {
    type Element = Pod<NativeIcon>;
    type ViewState = ();
    fn build(&self, ctx: &mut ViewCtx, _: &mut S) -> (Self::Element, ()) {
        (ctx.create_pod(NativeIcon(self.clone())), ())
    }
    fn rebuild(
        &self,
        previous: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut S,
    ) {
        if self != previous {
            NativeIcon::set(&mut element, self.clone());
        }
    }
    fn teardown(&self, _: &mut (), _: &mut ViewCtx, _: Mut<'_, Self::Element>) {}
    fn message(
        &self,
        _: &mut (),
        _: &mut MessageCtx,
        _: Mut<'_, Self::Element>,
        _: &mut S,
    ) -> MessageResult<()> {
        MessageResult::Stale
    }
}
