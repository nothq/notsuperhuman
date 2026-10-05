use crate::ui::types::{MailStyle, MailTableLayout};

use super::{
    apply_display, apply_line_height, apply_margin,
    background::{apply_background, apply_white_space, parse_css_background_size, parse_css_url},
    border::{
        apply_border, apply_border_bottom, apply_border_left, apply_border_right, apply_border_top,
        parse_css_border_radius, parse_css_border_spacing, set_border_side_color,
        set_border_side_width,
    },
    clean_css_value,
    layout::{
        parse_css_background_position, parse_css_background_repeat, parse_css_border_collapse,
        parse_css_box_sizing, parse_css_direction, parse_css_overflow, parse_css_position,
        parse_css_vertical_align,
    },
    parse_box_edges, parse_css_color, parse_css_float, parse_css_length, parse_css_length_with_em,
    parse_font_weight, parse_text_align, set_max_height, set_max_width, set_width,
    text::{
        parse_css_font_family, parse_css_font_style, parse_css_text_decoration,
        parse_css_text_transform, parse_css_word_break,
    },
    vendor::is_ignored_vendor_property,
};

pub(in crate::ui::parse) fn apply_style_declaration(
    property: &str,
    value: &str,
    style: &mut MailStyle,
) {
    let property = property.to_ascii_lowercase();
    if is_ignored_vendor_property(&property) {
        return;
    }
    if apply_text_declaration(property.as_str(), value, style) {
        return;
    }
    if apply_box_declaration(property.as_str(), value, style) {
        return;
    }
    apply_layout_declaration(property.as_str(), value, style);
}

pub(in crate::ui::parse) fn apply_style_declaration_with_inherited(
    property: &str,
    value: &str,
    inherited: &MailStyle,
    style: &mut MailStyle,
) {
    if clean_css_value(value) != "inherit" {
        apply_style_declaration(property, value, style);
        return;
    }
    let property = property.to_ascii_lowercase();
    let inherited_text = MailStyle::inherit_text(inherited);
    if apply_inherited_text_declaration(&property, &inherited_text, style)
        || apply_inherited_background_declaration(&property, inherited, style)
        || apply_inherited_spacing_declaration(&property, inherited, style)
        || apply_inherited_border_declaration(&property, inherited, style)
        || apply_inherited_sizing_declaration(&property, inherited, style)
    {
        return;
    }
    apply_inherited_layout_declaration(&property, inherited, &inherited_text, style);
}

fn apply_inherited_text_declaration(
    property: &str,
    inherited: &MailStyle,
    style: &mut MailStyle,
) -> bool {
    match property {
        "color" => style.color = inherited.color,
        "font-family" => style.font_family = inherited.font_family,
        "font-style" => style.font_style = inherited.font_style,
        "font-size" => style.font_size = inherited.font_size,
        "font-weight" => style.font_weight = inherited.font_weight,
        "line-height" => style.line_height = inherited.line_height,
        "text-align" => style.text_align = inherited.text_align,
        "text-decoration" => style.text_decoration = inherited.text_decoration,
        "text-transform" => style.text_transform = inherited.text_transform,
        "word-break" => style.word_break = inherited.word_break,
        "letter-spacing" => style.letter_spacing = inherited.letter_spacing,
        "white-space" => {
            style.preserve_whitespace = inherited.preserve_whitespace;
            style.nowrap = inherited.nowrap;
        }
        _ => return false,
    }
    true
}

fn apply_inherited_background_declaration(
    property: &str,
    inherited: &MailStyle,
    style: &mut MailStyle,
) -> bool {
    match property {
        "background" | "background-color" => style.background_color = inherited.background_color,
        "background-image" => style.background_image_url = inherited.background_image_url.clone(),
        "background-position" => style.background_position = inherited.background_position,
        "background-repeat" => style.background_repeat = inherited.background_repeat,
        "background-size" => style.background_size = inherited.background_size,
        _ => return false,
    }
    true
}

fn apply_inherited_spacing_declaration(
    property: &str,
    inherited: &MailStyle,
    style: &mut MailStyle,
) -> bool {
    match property {
        "padding" => style.padding = inherited.padding,
        "padding-top" => style.padding.top = inherited.padding.top,
        "padding-right" => style.padding.right = inherited.padding.right,
        "padding-bottom" => style.padding.bottom = inherited.padding.bottom,
        "padding-left" => style.padding.left = inherited.padding.left,
        "margin" => {
            style.margin = inherited.margin;
            style.margin_left_auto = inherited.margin_left_auto;
            style.margin_right_auto = inherited.margin_right_auto;
        }
        "margin-top" => style.margin.top = inherited.margin.top,
        "margin-right" => {
            style.margin.right = inherited.margin.right;
            style.margin_right_auto = inherited.margin_right_auto;
        }
        "margin-bottom" => style.margin.bottom = inherited.margin.bottom,
        "margin-left" => {
            style.margin.left = inherited.margin.left;
            style.margin_left_auto = inherited.margin_left_auto;
        }
        _ => return false,
    }
    true
}

fn apply_inherited_border_declaration(
    property: &str,
    inherited: &MailStyle,
    style: &mut MailStyle,
) -> bool {
    match property {
        "border" => {
            style.border_color = inherited.border_color;
            style.border_width = inherited.border_width;
            style.border_edges = inherited.border_edges;
        }
        "border-color" => style.border_color = inherited.border_color,
        "border-width" => style.border_width = inherited.border_width,
        "border-radius" => style.border_radius = inherited.border_radius,
        "border-collapse" => style.border_collapse = inherited.border_collapse,
        "border-spacing" => style.border_spacing = inherited.border_spacing,
        "box-sizing" => style.box_sizing = inherited.box_sizing,
        _ => return false,
    }
    true
}

fn apply_inherited_sizing_declaration(
    property: &str,
    inherited: &MailStyle,
    style: &mut MailStyle,
) -> bool {
    match property {
        "width" => {
            style.width = inherited.width;
            style.width_percent = inherited.width_percent;
        }
        "min-width" => style.min_width = inherited.min_width,
        "max-width" => {
            style.max_width = inherited.max_width;
            style.max_width_percent = inherited.max_width_percent;
        }
        "height" => style.height = inherited.height,
        "max-height" => style.max_height = inherited.max_height,
        _ => return false,
    }
    true
}

fn apply_inherited_layout_declaration(
    property: &str,
    inherited: &MailStyle,
    inherited_text: &MailStyle,
    style: &mut MailStyle,
) {
    match property {
        "direction" => style.direction = inherited_text.direction,
        "display" => {
            style.display = inherited.display;
            style.display_none = inherited.display_none;
        }
        "float" => style.float = inherited.float,
        "overflow" => style.overflow = inherited.overflow,
        "clip" => style.clip = inherited.clip.clone(),
        "position" => style.position = inherited.position,
        "vertical-align" => style.vertical_align = inherited.vertical_align,
        _ => {}
    }
}

fn apply_text_declaration(property: &str, value: &str, style: &mut MailStyle) -> bool {
    match property {
        "color" => style.color = parse_css_color(value),
        "font-family" => style.font_family = parse_css_font_family(value),
        "font-style" => {
            if let Some(font_style) = parse_css_font_style(value) {
                style.font_style = font_style;
            }
        }
        "font-size" => {
            style.font_size = parse_css_length_with_em(value, style.font_size.unwrap_or(16.0))
        }
        "font-weight" => style.font_weight = parse_font_weight(value),
        "line-height" => apply_line_height(value, style),
        "text-align" => {
            if let Some(align) = parse_text_align(value) {
                style.text_align = align;
                style.text_align_from_table_cell_attr = false;
            }
        }
        "text-decoration" => {
            if let Some(decoration) = parse_css_text_decoration(value) {
                style.text_decoration = decoration;
            }
        }
        "text-transform" => {
            if let Some(transform) = parse_css_text_transform(value) {
                style.text_transform = transform;
            }
        }
        // `normal` is no tracking, which is what failing to parse it gives.
        "letter-spacing" => {
            style.letter_spacing = parse_css_length_with_em(value, style.font_size.unwrap_or(16.0))
        }
        "word-break" => {
            if let Some(word_break) = parse_css_word_break(value) {
                style.word_break = word_break;
            }
        }
        "white-space" => apply_white_space(value, style),
        _ => return false,
    }
    true
}

fn apply_box_declaration(property: &str, value: &str, style: &mut MailStyle) -> bool {
    apply_background_declaration(property, value, style)
        || apply_spacing_declaration(property, value, style)
        || apply_border_declaration(property, value, style)
        || apply_sizing_declaration(property, value, style)
}

fn apply_background_declaration(property: &str, value: &str, style: &mut MailStyle) -> bool {
    match property {
        "background" => apply_background(value, style),
        "background-color" => {
            if let Some(color) = parse_css_color(value) {
                style.background_color = Some(color);
            }
        }
        "background-image" => style.background_image_url = parse_css_url(value),
        "background-position" => {
            if let Some(position) = parse_css_background_position(value) {
                style.background_position = position;
            }
        }
        "background-repeat" => {
            if let Some(repeat) = parse_css_background_repeat(value) {
                style.background_repeat = repeat;
            }
        }
        "background-size" => {
            if let Some(size) = parse_css_background_size(value) {
                style.background_size = size;
            }
        }
        _ => return false,
    }
    true
}

fn apply_spacing_declaration(property: &str, value: &str, style: &mut MailStyle) -> bool {
    let em_size = style.font_size.unwrap_or(16.0);
    match property {
        "padding" => {
            if let Some(edges) = parse_box_edges(value, em_size) {
                style.padding = edges;
            }
        }
        "padding-top" => super::set_edge_with_em(value, &mut style.padding.top, em_size),
        "padding-right" => super::set_edge_with_em(value, &mut style.padding.right, em_size),
        "padding-bottom" => super::set_edge_with_em(value, &mut style.padding.bottom, em_size),
        "padding-left" => super::set_edge_with_em(value, &mut style.padding.left, em_size),
        "margin" => apply_margin(value, style, em_size),
        "margin-top" => super::set_edge_with_em(value, &mut style.margin.top, em_size),
        "margin-right" => super::set_margin_edge_with_em(
            value,
            &mut style.margin.right,
            &mut style.margin_right_auto,
            em_size,
        ),
        "margin-bottom" => super::set_edge_with_em(value, &mut style.margin.bottom, em_size),
        "margin-left" => super::set_margin_edge_with_em(
            value,
            &mut style.margin.left,
            &mut style.margin_left_auto,
            em_size,
        ),
        _ => return false,
    }
    true
}

fn apply_border_declaration(property: &str, value: &str, style: &mut MailStyle) -> bool {
    match property {
        "border" => apply_border(value, style),
        "border-top" => apply_border_top(value, style),
        "border-right" => apply_border_right(value, style),
        "border-bottom" => apply_border_bottom(value, style),
        "border-left" => apply_border_left(value, style),
        "border-color" => style.border_color = parse_css_color(value),
        "border-top-color" => set_border_side_color(value, &mut style.border_edges.top),
        "border-right-color" => set_border_side_color(value, &mut style.border_edges.right),
        "border-bottom-color" => set_border_side_color(value, &mut style.border_edges.bottom),
        "border-left-color" => set_border_side_color(value, &mut style.border_edges.left),
        "border-width" => style.border_width = parse_css_length(value),
        "border-top-width" => set_border_side_width(value, &mut style.border_edges.top),
        "border-right-width" => set_border_side_width(value, &mut style.border_edges.right),
        "border-bottom-width" => set_border_side_width(value, &mut style.border_edges.bottom),
        "border-left-width" => set_border_side_width(value, &mut style.border_edges.left),
        "border-radius" => style.border_radius = parse_css_border_radius(value, style),
        "table-layout" => match value.trim().to_ascii_lowercase().as_str() {
            "fixed" => style.table_layout = MailTableLayout::Fixed,
            "auto" => style.table_layout = MailTableLayout::Auto,
            _ => {}
        },
        "border-collapse" => {
            if let Some(collapse) = parse_css_border_collapse(value) {
                style.border_collapse = collapse;
            }
        }
        "border-spacing" => style.border_spacing = parse_css_border_spacing(value),
        _ => return false,
    }
    true
}

fn apply_sizing_declaration(property: &str, value: &str, style: &mut MailStyle) -> bool {
    match property {
        "box-sizing" => {
            if let Some(box_sizing) = parse_css_box_sizing(value) {
                style.box_sizing = box_sizing;
            }
        }
        "width" => set_width(value, style),
        "min-width" => style.min_width = parse_css_length(value),
        "max-width" => set_max_width(value, style),
        "height" => style.height = parse_css_length(value),
        "max-height" => set_max_height(value, style),
        _ => return false,
    }
    true
}

fn apply_layout_declaration(property: &str, value: &str, style: &mut MailStyle) {
    match property {
        "direction" => {
            if let Some(direction) = parse_css_direction(value) {
                style.direction = direction;
            }
        }
        "display" => apply_display(value, style),
        "float" => {
            if let Some(float) = parse_css_float(value) {
                style.float = float;
            }
        }
        "overflow" => {
            if let Some(overflow) = parse_css_overflow(value) {
                style.overflow = overflow;
            }
        }
        "clip" => style.clip = Some(clean_css_value(value)),
        "position" => {
            if let Some(position) = parse_css_position(value) {
                style.position = position;
            }
        }
        "vertical-align" => {
            if let Some(align) = parse_css_vertical_align(value) {
                style.vertical_align = align;
            }
        }
        _ => {}
    }
}
