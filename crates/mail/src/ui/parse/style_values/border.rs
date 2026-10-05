use crate::ui::types::{MailBorderEdges, MailBorderSide, MailStyle, MailTableSpacing};

use super::{clean_css_value, parse_css_color, parse_css_length, parse_css_percent};

pub(super) fn parse_css_border_radius(value: &str, style: &MailStyle) -> Option<f32> {
    parse_css_length(value).or_else(|| {
        parse_css_percent(value).map(|percent| {
            style
                .width
                .or(style.height)
                .map(|size| size * percent)
                .unwrap_or(9999.0 * percent)
        })
    })
}

pub(super) fn parse_css_border_spacing(value: &str) -> Option<MailTableSpacing> {
    let values = value
        .split_whitespace()
        .filter_map(parse_css_length)
        .collect::<Vec<_>>();
    match values.as_slice() {
        [all] => Some(MailTableSpacing {
            horizontal: *all,
            vertical: *all,
        }),
        [horizontal, vertical, ..] => Some(MailTableSpacing {
            horizontal: *horizontal,
            vertical: *vertical,
        }),
        _ => None,
    }
}

pub(super) fn apply_border(value: &str, style: &mut MailStyle) {
    let side = parse_border_side(value);
    style.border_width = side.width;
    style.border_color = side.color;
    style.border_edges = MailBorderEdges {
        top: side,
        right: side,
        bottom: side,
        left: side,
    };
}

pub(super) fn apply_border_top(value: &str, style: &mut MailStyle) {
    style.border_edges.top = parse_border_side(value);
}

pub(super) fn apply_border_right(value: &str, style: &mut MailStyle) {
    style.border_edges.right = parse_border_side(value);
}

pub(super) fn apply_border_bottom(value: &str, style: &mut MailStyle) {
    style.border_edges.bottom = parse_border_side(value);
}

pub(super) fn apply_border_left(value: &str, style: &mut MailStyle) {
    style.border_edges.left = parse_border_side(value);
}

pub(super) fn set_border_side_width(value: &str, side: &mut MailBorderSide) {
    if let Some(width) = parse_css_length(value) {
        side.width = Some(width);
    }
}

pub(super) fn set_border_side_color(value: &str, side: &mut MailBorderSide) {
    if let Some(color) = parse_css_color(value) {
        side.color = Some(color);
    }
}

fn parse_border_side(value: &str) -> MailBorderSide {
    let parts = value
        .split_whitespace()
        .map(clean_css_value)
        .collect::<Vec<_>>();
    if parts.iter().any(|part| part == "none" || part == "hidden") {
        return MailBorderSide {
            width: Some(0.0),
            color: None,
        };
    }
    let mut side = MailBorderSide::default();
    for part in &parts {
        if side.width.is_none() {
            side.width = parse_css_length(part);
        }
        if side.color.is_none() {
            side.color = parse_css_color(part);
        }
    }
    if side.width.is_none() && !parts.is_empty() {
        side.width = Some(1.0);
    }
    side
}
