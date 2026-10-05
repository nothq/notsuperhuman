use super::dom::html_attr;
use crate::ui::types::*;
use html5ever::Attribute;
use std::cell::RefCell;

mod background;
mod border;
mod color;
mod declaration;
mod layout;
#[cfg(test)]
mod tests;
mod text;
mod vendor;

pub(super) use color::parse_css_color;
#[cfg(test)]
pub(super) use declaration::apply_style_declaration;
pub(super) use declaration::apply_style_declaration_with_inherited;

pub(super) fn apply_html_style_attrs(
    tag_name: &str,
    attrs: &RefCell<Vec<Attribute>>,
    style: &mut MailStyle,
) {
    if let Some(color) = html_attr(attrs, "bgcolor")
        .as_deref()
        .and_then(parse_css_color)
    {
        style.background_color = Some(color);
    }
    // `<td background="...">` is the presentational form of `background-image`,
    // and the one Outlook honours, so email carries every photo-backed cell in
    // it — often alongside the CSS form, sometimes instead of it.
    if let Some(url) = html_attr(attrs, "background")
        .map(|url| url.trim().to_string())
        .filter(|url| super::dom::safe_image_url(url))
    {
        style.background_image_url = Some(url);
    }
    if tag_name == "table" {
        apply_table_attrs(attrs, style);
    } else {
        apply_alignment_attrs(tag_name, attrs, style);
    }
    if let Some(width) = html_attr(attrs, "width").as_deref() {
        set_width(width, style);
    }
    if let Some(height) = html_attr(attrs, "height")
        .as_deref()
        .and_then(parse_css_length)
    {
        style.height = Some(height);
    }
    if let Some(border_width) = html_attr(attrs, "border")
        .as_deref()
        .and_then(parse_css_length)
    {
        style.border_width = Some(border_width);
    }
    if let Some(vertical_align) = html_attr(attrs, "valign")
        .as_deref()
        .and_then(layout::parse_css_vertical_align)
    {
        style.vertical_align = vertical_align;
    }
}

fn apply_table_attrs(attrs: &RefCell<Vec<Attribute>>, style: &mut MailStyle) {
    // The HTML rendering rules give tables `box-sizing: border-box`, so a
    // `max-width: 500px` table with a 1px border has 498px inside it, and
    // the image filling it is 498px tall, not 500.
    style.box_sizing = MailBoxSizing::BorderBox;
    // `align` on a table positions the table box itself, not its text:
    // `center` is the classic email centring idiom, `left`/`right` float it.
    match html_attr(attrs, "align").as_deref().map(str::trim) {
        Some(value) if value.eq_ignore_ascii_case("center") => {
            style.margin_left_auto = true;
            style.margin_right_auto = true;
        }
        Some(value) if value.eq_ignore_ascii_case("right") => style.float = MailFloat::Right,
        Some(value) if value.eq_ignore_ascii_case("left") => style.float = MailFloat::Left,
        _ => {}
    }
}

fn apply_alignment_attrs(tag_name: &str, attrs: &RefCell<Vec<Attribute>>, style: &mut MailStyle) {
    if let Some(align) = html_attr(attrs, "align")
        .as_deref()
        .and_then(parse_text_align)
    {
        style.text_align = align;
        style.text_align_from_table_cell_attr = matches!(tag_name, "td" | "th");
    }
    if tag_name == "img" {
        if let Some(float) = html_attr(attrs, "align")
            .as_deref()
            .and_then(parse_css_float)
        {
            style.float = float;
        }
    }
}

#[cfg(test)]
pub(super) fn apply_style_attr(style_attr: &str, style: &mut MailStyle) {
    for declaration in style_attr.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        apply_style_declaration(property.trim(), value.trim(), style);
    }
}

fn set_edge_with_em(value: &str, edge: &mut f32, em_size: f32) {
    if let Some(parsed) = parse_css_length_with_em(value, em_size) {
        *edge = parsed;
    }
}

fn apply_display(value: &str, style: &mut MailStyle) {
    if let Some(display) = layout::parse_css_display(value) {
        style.display = display;
        style.display_none = display == MailDisplay::None;
    }
}

fn set_margin_edge_with_em(value: &str, edge: &mut f32, auto: &mut bool, em_size: f32) {
    if clean_css_value(value).eq_ignore_ascii_case("auto") {
        *edge = 0.0;
        *auto = true;
        return;
    }
    if let Some(parsed) = parse_css_length_with_em(value, em_size) {
        *edge = parsed;
        *auto = false;
    }
}

fn set_width(value: &str, style: &mut MailStyle) {
    if clean_css_value(value).eq_ignore_ascii_case("auto") {
        style.width = None;
        style.width_percent = None;
        return;
    }
    if let Some(width) = parse_css_length(value) {
        style.width = Some(width);
        style.width_percent = None;
        return;
    }
    if let Some(width_percent) = parse_css_percent(value) {
        style.width = None;
        style.width_percent = Some(width_percent.clamp(0.0, 1.0));
    }
}

fn set_max_width(value: &str, style: &mut MailStyle) {
    // `none` is the initial value, so it clears the constraint. Responsive email
    // depends on this: a mobile stylesheet lifts a desktop `max-width` with
    // `max-width: none !important`, and ignoring the keyword left the cap in
    // place and the layout stuck at its desktop width.
    if clean_css_value(value).eq_ignore_ascii_case("none") {
        style.max_width = None;
        style.max_width_percent = None;
        return;
    }
    if let Some(max_width) = parse_css_length(value) {
        style.max_width = Some(max_width);
        style.max_width_percent = None;
        return;
    }
    if let Some(max_width_percent) = parse_css_percent(value) {
        style.max_width = None;
        style.max_width_percent = Some(max_width_percent.clamp(0.0, 1.0));
    }
}

fn set_max_height(value: &str, style: &mut MailStyle) {
    if clean_css_value(value).eq_ignore_ascii_case("none") {
        style.max_height = None;
        return;
    }
    if let Some(max_height) = parse_css_length(value) {
        style.max_height = Some(max_height);
    }
}

fn apply_margin(value: &str, style: &mut MailStyle, em_size: f32) {
    // `!important` belongs to the whole declaration, not to its last token:
    // splitting first turned `margin: 0 !important` into a two-value shorthand
    // whose horizontal half was empty, so the sides kept their earlier value.
    let cleaned = clean_css_value(value);
    let values = cleaned.split_whitespace().collect::<Vec<_>>();
    match values.as_slice() {
        [all] => apply_margin_values(style, [all, all, all, all], em_size),
        [vertical, horizontal] => {
            apply_margin_values(style, [vertical, horizontal, vertical, horizontal], em_size)
        }
        [top, horizontal, bottom] => {
            apply_margin_values(style, [top, horizontal, bottom, horizontal], em_size)
        }
        [top, right, bottom, left, ..] => {
            apply_margin_values(style, [top, right, bottom, left], em_size)
        }
        _ => {}
    }
}

/// `edges` are the margin values in CSS order: top, right, bottom, left.
fn apply_margin_values(style: &mut MailStyle, edges: [&str; 4], em_size: f32) {
    let [top, right, bottom, left] = edges;
    set_edge_with_em(top, &mut style.margin.top, em_size);
    set_margin_edge_with_em(
        right,
        &mut style.margin.right,
        &mut style.margin_right_auto,
        em_size,
    );
    set_edge_with_em(bottom, &mut style.margin.bottom, em_size);
    set_margin_edge_with_em(
        left,
        &mut style.margin.left,
        &mut style.margin_left_auto,
        em_size,
    );
}

fn parse_box_edges(value: &str, em_size: f32) -> Option<MailEdgeInsets> {
    let values = value
        .split_whitespace()
        .filter_map(|value| parse_css_length_with_em(value, em_size))
        .collect::<Vec<_>>();
    match values.as_slice() {
        [all] => Some(MailEdgeInsets {
            top: *all,
            right: *all,
            bottom: *all,
            left: *all,
        }),
        [vertical, horizontal] => Some(MailEdgeInsets {
            top: *vertical,
            right: *horizontal,
            bottom: *vertical,
            left: *horizontal,
        }),
        [top, horizontal, bottom] => Some(MailEdgeInsets {
            top: *top,
            right: *horizontal,
            bottom: *bottom,
            left: *horizontal,
        }),
        [top, right, bottom, left, ..] => Some(MailEdgeInsets {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        }),
        _ => None,
    }
}

pub(super) fn parse_css_length(value: &str) -> Option<f32> {
    let value = clean_css_value(value);
    if value.ends_with('%') || value.eq_ignore_ascii_case("auto") {
        return None;
    }
    // CSS absolute units, in CSS pixels. Outlook writes every measurement in
    // points and inches, so a signature's `padding: 0in` must read as zero
    // rather than fail and leave the cell its inherited padding.
    let (number, scale) = [
        ("px", 1.0),
        ("pt", 4.0 / 3.0),
        ("pc", 16.0),
        ("in", 96.0),
        ("cm", 96.0 / 2.54),
        ("mm", 96.0 / 25.4),
    ]
    .into_iter()
    .find_map(|(unit, scale)| value.strip_suffix(unit).map(|number| (number, scale)))
    .unwrap_or((value.as_str(), 1.0));
    number
        .parse::<f32>()
        .ok()
        .map(|number| number * scale)
        .filter(|value| value.is_finite())
}

pub(super) fn parse_css_length_with_em(value: &str, em_size: f32) -> Option<f32> {
    let value = clean_css_value(value);
    if let Some(value) = value.strip_suffix("rem") {
        return value
            .parse::<f32>()
            .ok()
            .map(|value| value * 16.0)
            .filter(|value| value.is_finite());
    }
    if let Some(value) = value.strip_suffix("em") {
        return value
            .parse::<f32>()
            .ok()
            .map(|value| value * em_size)
            .filter(|value| value.is_finite());
    }
    parse_css_length(&value)
}

pub(super) fn parse_css_percent(value: &str) -> Option<f32> {
    let value = clean_css_value(value);
    let value = value.strip_suffix('%')?;
    let percent = value.parse::<f32>().ok()? / 100.0;
    percent.is_finite().then_some(percent)
}

fn apply_line_height(value: &str, style: &mut MailStyle) {
    let value = clean_css_value(value);
    if value.eq_ignore_ascii_case("normal") {
        style.line_height = None;
        return;
    }
    if let Some(multiplier) = parse_css_percent(&value).filter(|value| *value >= 0.0) {
        style.line_height = Some(MailLineHeight::Percentage(multiplier));
        return;
    }
    if !value.ends_with("px") && !value.ends_with("pt") {
        if let Some(multiplier) = value
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite() && *value >= 0.0)
        {
            style.line_height = Some(MailLineHeight::Number(multiplier));
            return;
        }
    }
    if let Some(line_height) = parse_css_length(&value).filter(|value| *value >= 0.0) {
        style.line_height = Some(MailLineHeight::Pixels(line_height));
    }
}

fn parse_font_weight(value: &str) -> Option<MailFontWeight> {
    let value = clean_css_value(value);
    if value.eq_ignore_ascii_case("bold") {
        return Some(MailFontWeight::Bold);
    }
    if value.eq_ignore_ascii_case("normal") {
        return Some(MailFontWeight::Normal);
    }
    value.parse::<u16>().ok().map(|weight| {
        if weight >= 700 {
            MailFontWeight::Bold
        } else if weight >= 500 {
            MailFontWeight::Medium
        } else {
            MailFontWeight::Normal
        }
    })
}

fn parse_text_align(value: &str) -> Option<MailTextAlign> {
    match clean_css_value(value).to_ascii_lowercase().as_str() {
        "start" => Some(MailTextAlign::Start),
        "end" => Some(MailTextAlign::End),
        "center" => Some(MailTextAlign::Center),
        "right" => Some(MailTextAlign::Right),
        "left" => Some(MailTextAlign::Left),
        _ => None,
    }
}

fn parse_css_float(value: &str) -> Option<MailFloat> {
    match clean_css_value(value).as_str() {
        "left" => Some(MailFloat::Left),
        "right" => Some(MailFloat::Right),
        "none" => Some(MailFloat::None),
        _ => None,
    }
}

pub(super) fn clean_css_value(value: &str) -> String {
    let value = value.trim();
    let value = if value.to_ascii_lowercase().ends_with("!important") {
        &value[..value.len() - "!important".len()]
    } else {
        value
    };
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
}
