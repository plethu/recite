//! Read-only text with byte-range styles and an optional visual line cap.
//!
//! Recite owns this widget so comparison emphasis and Map excerpts need no toolkit fork. A visual
//! abbreviation never changes what assistive technology reads: accessibility always describes the
//! complete text and its original layout.
use std::{any::TypeId, num::NonZeroUsize, ops::Range};
use xilem::masonry::{
    TextAlign, TextAlignOptions,
    accesskit::{Node, Role},
    core::{
        AccessCtx, ArcStr, BrushIndex, ChildrenIds, LayoutCtx, MeasureCtx, NoAction, PaintCtx,
        PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, StyleSet, Update, UpdateCtx,
        UsesProperty, Widget, WidgetMut, render_text, set_accesskit_brush_properties,
    },
    imaging::Painter,
    kurbo::{Affine, Axis, Point, Size},
    layout::{AsUnit, LenReq, Length},
    parley::{FontContext, Layout, LayoutAccessibility, LayoutContext},
    properties::{ContentColor, LineBreaking},
    theme::{TEXT_SIZE_NORMAL, default_text_styles},
};

mod abbreviation;
#[cfg(test)]
mod tests;

/// Distinct widths kept shaped; measurement and layout usually ask for two or three.
const CACHE_CAPACITY: usize = 4;

struct Shaped {
    max_advance: Option<f32>,
    last_used: u64,
    layout: Layout<BrushIndex>,
    abbreviation: Option<abbreviation::Abbreviation>,
}
impl Shaped {
    fn display(&self) -> &Layout<BrushIndex> {
        self.abbreviation
            .as_ref()
            .map_or(&self.layout, |abbreviation| &abbreviation.layout)
    }
}

pub(crate) struct ShapedText {
    text: ArcStr,
    styles: StyleSet,
    ranged: Vec<(Range<usize>, StyleProperty)>,
    max_lines: Option<NonZeroUsize>,
    shaped: Vec<Shaped>,
    active: usize,
    clock: u64,
    accessibility: LayoutAccessibility,
    /// Abbreviation candidates shaped since construction, for cost tests.
    abbreviation_shapes: usize,
}

impl ShapedText {
    pub(crate) fn new(text: impl Into<ArcStr>) -> Self {
        let mut styles = StyleSet::new(TEXT_SIZE_NORMAL);
        default_text_styles(&mut styles);
        Self {
            text: text.into(),
            styles,
            ranged: Vec::new(),
            max_lines: None,
            shaped: Vec::new(),
            active: 0,
            clock: 0,
            accessibility: LayoutAccessibility::default(),
            abbreviation_shapes: 0,
        }
    }
    pub(crate) fn with_style(mut self, property: impl Into<StyleProperty>) -> Self {
        self.styles.insert(property.into());
        self
    }
    /// Styles applied to byte ranges after the global styles.
    ///
    /// # Panics
    ///
    /// Panics if a range is outside the text, splits a UTF-8 character, or names a brush other
    /// than the single text colour.
    pub(crate) fn with_ranged_styles(mut self, styles: Vec<(Range<usize>, StyleProperty)>) -> Self {
        self.replace_ranged(styles);
        self
    }
    /// Cap the visual text at a line count, ending in a shaped ellipsis on overflow.
    pub(crate) fn with_max_lines(mut self, limit: Option<NonZeroUsize>) -> Self {
        self.max_lines = limit;
        self
    }
    /// The text currently drawn, which ends in an ellipsis when abbreviated.
    pub(crate) fn displayed_text(&self) -> &str {
        self.shaped
            .get(self.active)
            .and_then(|shaped| shaped.abbreviation.as_ref())
            .map_or(self.text.as_ref(), |abbreviation| &abbreviation.text)
    }

    /// Replace the text, clearing its ranged styles.
    pub(crate) fn set_text(this: &mut WidgetMut<'_, Self>, text: impl Into<ArcStr>) {
        this.widget.text = text.into();
        this.widget.ranged.clear();
        Self::reshape(this);
    }
    pub(crate) fn insert_style(this: &mut WidgetMut<'_, Self>, property: impl Into<StyleProperty>) {
        this.widget.styles.insert(property.into());
        Self::reshape(this);
    }
    pub(crate) fn set_ranged_styles(
        this: &mut WidgetMut<'_, Self>,
        styles: Vec<(Range<usize>, StyleProperty)>,
    ) {
        this.widget.replace_ranged(styles);
        Self::reshape(this);
    }
    pub(crate) fn set_max_lines(this: &mut WidgetMut<'_, Self>, limit: Option<NonZeroUsize>) {
        if this.widget.max_lines != limit {
            this.widget.max_lines = limit;
            Self::reshape(this);
        }
    }

    fn reshape(this: &mut WidgetMut<'_, Self>) {
        this.widget.shaped.clear();
        this.widget.active = 0;
        this.ctx.request_layout();
    }
    fn replace_ranged(&mut self, styles: Vec<(Range<usize>, StyleProperty)>) {
        for (range, property) in &styles {
            assert!(
                self.text.get(range.clone()).is_some(),
                "a styled range must lie on character boundaries within the text"
            );
            assert!(
                !matches!(
                    property,
                    StyleProperty::Brush(BrushIndex(1..))
                        | StyleProperty::UnderlineBrush(Some(BrushIndex(1..)))
                        | StyleProperty::StrikethroughBrush(Some(BrushIndex(1..)))
                ),
                "shaped text draws with a single text colour"
            );
        }
        self.ranged = styles;
    }

    /// The cache entry for `max_advance`, shaping it if no entry matches exactly.
    fn shape(
        &mut self,
        fonts: &mut FontContext,
        layouts: &mut LayoutContext<BrushIndex>,
        max_advance: Option<f32>,
    ) -> usize {
        self.clock += 1;
        if let Some(index) = self
            .shaped
            .iter()
            .position(|shaped| shaped.max_advance == max_advance)
        {
            self.shaped[index].last_used = self.clock;
            return index;
        }
        let mut layout = Layout::new();
        let mut builder = layouts.ranged_builder(fonts, &self.text, 1., true);
        for property in self.styles.inner().values() {
            builder.push_default(property.to_owned());
        }
        for (range, property) in &self.ranged {
            builder.push(property.clone(), range.clone());
        }
        builder.build_into(&mut layout, &self.text);
        layout.break_all_lines(max_advance);
        let abbreviation = self.max_lines.and_then(|lines| {
            let mut abbreviator = abbreviation::Abbreviator {
                text: &self.text,
                styles: &self.styles,
                ranged: &self.ranged,
                max_advance,
                fonts,
                layouts,
                shapes: 0,
            };
            let result = abbreviator.abbreviate(&layout, lines);
            self.abbreviation_shapes += abbreviator.shapes;
            result
        });
        let shaped = Shaped {
            max_advance,
            last_used: self.clock,
            layout,
            abbreviation,
        };
        if self.shaped.len() < CACHE_CAPACITY {
            self.shaped.push(shaped);
            return self.shaped.len() - 1;
        }
        let active = self.active;
        let index = (0..self.shaped.len())
            .filter(|index| *index != active)
            .min_by_key(|index| self.shaped[*index].last_used)
            .unwrap_or(0);
        self.shaped[index] = shaped;
        index
    }
}

/// The inline extent to wrap at for a measurement, or `None` when unbounded.
fn max_advance(
    breaking: LineBreaking,
    inline_axis: bool,
    request: LenReq,
    cross: Option<Length>,
) -> Option<f32> {
    if breaking != LineBreaking::WordWrap {
        return None;
    }
    let length = match (inline_axis, request) {
        (true, LenReq::MinContent) => Some(Length::ZERO),
        (true, LenReq::MaxContent) => None,
        (true, LenReq::FitContent(space)) => Some(space),
        // Block extent follows the inline extent; without one, use the inline default.
        (false, LenReq::MinContent) => cross.or(Some(Length::ZERO)),
        (false, LenReq::MaxContent | LenReq::FitContent(_)) => cross,
    };
    length.map(|length| length.get() as f32)
}

impl UsesProperty<ContentColor> for ShapedText {}
impl UsesProperty<LineBreaking> for ShapedText {}

impl Widget for ShapedText {
    type Action = NoAction;

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }
    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    fn property_changed(&mut self, ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        LineBreaking::prop_changed(ctx, property_type);
        ContentColor::prop_changed(ctx, property_type);
    }
    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                self.shaped.clear();
                self.active = 0;
                ctx.request_layout();
            }
            Update::DisabledChanged(_) => ctx.request_render(),
            _ => {}
        }
    }
    fn measure(
        &mut self,
        ctx: &mut MeasureCtx<'_>,
        props: &PropertiesRef<'_>,
        axis: Axis,
        request: LenReq,
        cross: Option<Length>,
    ) -> Length {
        // Only horizontal top-to-bottom writing is supported, so the inline axis is horizontal.
        let inline_axis = axis == Axis::Horizontal;
        let breaking = *props.get::<LineBreaking>(ctx.property_cache());
        let max_advance = max_advance(breaking, inline_axis, request, cross);
        let (fonts, layouts) = ctx.text_contexts();
        let index = self.shape(fonts, layouts, max_advance);
        let display = self.shaped[index].display();
        if inline_axis {
            display.width().px()
        } else {
            display.height().px()
        }
    }
    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        let breaking = *props.get::<LineBreaking>(ctx.property_cache());
        let inline_space = size.width as f32;
        let max_advance = (breaking == LineBreaking::WordWrap).then_some(inline_space);
        let (fonts, layouts) = ctx.text_contexts();
        self.active = self.shape(fonts, layouts, max_advance);
        let shaped = &mut self.shaped[self.active];
        let options = TextAlignOptions::default();
        shaped
            .layout
            .align(Some(inline_space), TextAlign::Start, options);
        if let Some(abbreviation) = &mut shaped.abbreviation {
            abbreviation
                .layout
                .align(Some(inline_space), TextAlign::Start, options);
        }
        let display = shaped.display();
        match (
            display.get(0),
            display
                .len()
                .checked_sub(1)
                .and_then(|last| display.get(last)),
        ) {
            (Some(first), Some(last)) => ctx.set_baselines(
                f64::from(first.metrics().baseline),
                f64::from(last.metrics().baseline),
            ),
            _ => ctx.clear_baselines(),
        }
        if breaking == LineBreaking::Clip || shaped.abbreviation.is_some() {
            let border_box = size.to_rect() + ctx.border_box_insets();
            ctx.set_clip_path(border_box);
        } else {
            ctx.clear_clip_path();
        }
    }
    fn paint(
        &mut self,
        ctx: &mut PaintCtx<'_>,
        props: &PropertiesRef<'_>,
        painter: &mut Painter<'_>,
    ) {
        let colour = props.get::<ContentColor>(ctx.property_cache());
        if let Some(shaped) = self.shaped.get(self.active) {
            render_text(
                painter,
                Affine::IDENTITY,
                shaped.display(),
                &[colour.color.into()],
                true,
            );
        }
    }
    fn accessibility_role(&self) -> Role {
        Role::Label
    }
    fn accessibility(
        &mut self,
        ctx: &mut AccessCtx<'_>,
        props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let Some(shaped) = self.shaped.get(self.active) else {
            return;
        };
        let origin = Point::ORIGIN + ctx.border_box_translation();
        let colour = props.get::<ContentColor>(ctx.property_cache()).color;
        // The complete text and its original layout, never the visual abbreviation.
        self.accessibility.build_nodes(
            self.text.as_ref(),
            &shaped.layout,
            ctx.tree_update(),
            node,
            AccessCtx::next_node_id,
            origin.x,
            origin.y,
            |node, style| set_accesskit_brush_properties(node, style, &[colour.into()]),
        );
    }
    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
    fn get_debug_text(&self) -> Option<String> {
        Some(self.displayed_text().to_owned())
    }
}
