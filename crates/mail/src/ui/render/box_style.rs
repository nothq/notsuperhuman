//! The CSS box model applied to a GPUI element: sizing, spacing, borders,
//! backgrounds and positioning.

use super::MailRenderOptions;
use crate::ui::types::*;
use gpui::{prelude::*, px, relative, rgb, Div};

pub(super) fn apply_box_style(
    mut element: Div,
    style: &MailStyle,
    options: &MailRenderOptions,
) -> Div {
    element = apply_box_sizing(element, style);
    apply_box_decoration(element, style, options)
}

pub(super) fn apply_box_decoration(
    mut element: Div,
    style: &MailStyle,
    _options: &MailRenderOptions,
) -> Div {
    element = apply_box_spacing(element, style);
    element = apply_box_border(element, style);
    element = apply_box_positioning(element, style);
    if let Some(background) = style.background_color {
        // Under a border the box's own quad takes the border colour and the
        // fill is an inset layer (see `render_inset_fill`), so the only edge
        // that meets the page is border on border.
        element = if fill_under_border(style) {
            element
                .relative()
                .bg(box_border_color(style).map(rgb).unwrap_or(rgb(background)))
        } else {
            element.bg(rgb(background))
        };
    }
    if let Some(radius) = style.border_radius {
        element = element.rounded(px(radius));
    }
    if style.background_image_url.is_some() {
        element = element.relative().overflow_hidden();
    }
    element
}

/// Whether a box's fill has to be drawn inside its border: it has both, and
/// the border is one colour all round, so painting the box in that colour
/// changes nothing but the edge.
pub(super) fn fill_under_border(style: &MailStyle) -> bool {
    let edges = &style.border_edges;
    let colours = [
        edges.top.color,
        edges.right.color,
        edges.bottom.color,
        edges.left.color,
    ]
    .map(|colour| colour.or(style.border_color));
    style.background_color.is_some()
        && colours[0].is_some()
        && colours.iter().all(|colour| *colour == colours[0])
        && [
            box_border_top_width(style),
            box_border_right_width(style),
            box_border_bottom_width(style),
            box_border_left_width(style),
        ]
        .iter()
        .all(|width| *width > 0.0)
}

pub(super) fn apply_box_sizing(mut element: Div, style: &MailStyle) -> Div {
    if let Some(width) = style.width {
        element = element.w(px(width));
    }
    if let Some(width_percent) = style.width_percent {
        element = if width_percent >= 0.999 {
            element.w_full()
        } else {
            element.w(relative(width_percent))
        };
    }
    if let Some(min_width) = style.min_width {
        element = element.min_w(px(min_width));
    }
    if let Some(max_width) = style.max_width {
        element = element.max_w(px(max_width));
    }
    if let Some(max_width_percent) = style.max_width_percent {
        element = element.max_w(relative(max_width_percent));
    }
    if let Some(height) = style.height {
        element = element.min_h(px(height));
    }
    if let Some(max_height) = style.max_height {
        element = element.max_h(px(max_height));
    }
    if style.width.is_some() || style.width_percent.is_some() {
        element = if style.max_width_percent.is_some() {
            element.flex_shrink_1()
        } else {
            element.flex_shrink_0()
        };
    }
    element
}

pub(super) fn apply_box_spacing(mut element: Div, style: &MailStyle) -> Div {
    if style.padding.any() {
        element = element
            .pt(px(style.padding.top))
            .pr(px(style.padding.right))
            .pb(px(style.padding.bottom))
            .pl(px(style.padding.left));
    }
    if style.margin.any() {
        element = element
            .mt(px(style.margin.top))
            .mr(px(style.margin.right))
            .mb(px(style.margin.bottom))
            .ml(px(style.margin.left));
    }
    if style.margin_left_auto && style.margin_right_auto {
        element = element.mx_auto();
    }
    element
}

pub(super) fn apply_box_border(mut element: Div, style: &MailStyle) -> Div {
    let border_width = box_border_width(style);
    if !style.border_edges.any() && border_width > 0.0 {
        element = element
            .border(px(border_width))
            .border_color(style.border_color.map(rgb).unwrap_or(rgb(0xd8dde6)));
    }
    let border_color = box_border_color(style).map(rgb).unwrap_or(rgb(0xd8dde6));
    let top_width = box_border_top_width(style);
    if top_width > 0.0 {
        element = element.border_t(px(top_width)).border_color(border_color);
    }
    let right_width = box_border_right_width(style);
    if right_width > 0.0 {
        element = element.border_r(px(right_width)).border_color(border_color);
    }
    let bottom_width = box_border_bottom_width(style);
    if bottom_width > 0.0 {
        element = element
            .border_b(px(bottom_width))
            .border_color(border_color);
    }
    let left_width = box_border_left_width(style);
    if left_width > 0.0 {
        element = element.border_l(px(left_width)).border_color(border_color);
    }
    element
}

pub(super) fn apply_box_positioning(mut element: Div, style: &MailStyle) -> Div {
    // Only `overflow` clips. A `max-height` constrains the box and lets its
    // content spill out visibly, which is how a card's photo and its rating
    // badge sit inside a 15px-tall wrapper; clipping on the constraint alone
    // erased both. The preheader idiom that hides text still works, because it
    // always pairs `max-height: 0` with `overflow: hidden`.
    if matches!(
        style.overflow,
        MailOverflow::Hidden | MailOverflow::Scroll | MailOverflow::Auto
    ) {
        element = element.overflow_hidden();
    }
    if style.position == MailPosition::Relative {
        element = element.relative();
    }
    element
}

pub(super) fn box_border_width(style: &MailStyle) -> f32 {
    style.border_width.unwrap_or_default()
}

pub(super) fn box_border_top_width(style: &MailStyle) -> f32 {
    box_border_side_width(style.border_edges.top, style)
}

pub(super) fn box_border_right_width(style: &MailStyle) -> f32 {
    box_border_side_width(style.border_edges.right, style)
}

pub(super) fn box_border_bottom_width(style: &MailStyle) -> f32 {
    box_border_side_width(style.border_edges.bottom, style)
}

pub(super) fn box_border_left_width(style: &MailStyle) -> f32 {
    box_border_side_width(style.border_edges.left, style)
}

pub(super) fn box_border_side_width(side: MailBorderSide, style: &MailStyle) -> f32 {
    side.width
        .or(style.border_width)
        .or_else(|| side.color.map(|_| 1.0))
        .unwrap_or_default()
}

pub(super) fn box_border_color(style: &MailStyle) -> Option<u32> {
    style
        .border_edges
        .top
        .color
        .or(style.border_edges.right.color)
        .or(style.border_edges.bottom.color)
        .or(style.border_edges.left.color)
        .or(style.border_color)
}
