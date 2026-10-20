// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::layout::Length;
use crate::peniko::color::{AlphaColor, Srgb};

/// The color of a checkbox's "check" icon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckmarkColor {
    /// Colour of the selected indicator.
    pub color: AlphaColor<Srgb>,
}

impl Property for CheckmarkColor {
    fn static_default() -> &'static Self {
        static DEFAULT: CheckmarkColor = CheckmarkColor {
            color: AlphaColor::BLACK,
        };
        &DEFAULT
    }
}

/// The width of the stroke which draws a checkbox's "check" icon.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckmarkStrokeWidth {
    /// Stroke width in logical pixels.
    pub width: f64,
}

/// Indicator geometry shared by checkbox and radio controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CheckmarkSize {
    /// Side length of the indicator in logical units.
    pub side: Length,
    /// Space between the indicator and its label in logical units.
    pub gap: Length,
}
impl Property for CheckmarkSize {
    fn static_default() -> &'static Self {
        static DEFAULT: CheckmarkSize = CheckmarkSize {
            side: crate::theme::BASIC_WIDGET_HEIGHT,
            gap: crate::theme::WIDGET_CONTROL_COMPONENT_PADDING,
        };
        &DEFAULT
    }
}
impl Default for CheckmarkSize {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl Property for CheckmarkStrokeWidth {
    fn static_default() -> &'static Self {
        static DEFAULT: CheckmarkStrokeWidth = CheckmarkStrokeWidth { width: 1. };
        &DEFAULT
    }
}

// ---

impl Default for CheckmarkColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl CheckmarkColor {
    /// Creates new `CheckmarkColor` with given value.
    pub fn new(color: AlphaColor<Srgb>) -> Self {
        Self { color }
    }

    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}

// ---

impl Default for CheckmarkStrokeWidth {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl CheckmarkStrokeWidth {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type != TypeId::of::<Self>() {
            return;
        }
        ctx.request_paint_only();
    }
}
