use super::super::dom::safe_image_url;
use super::{clean_css_value, parse_css_color};
use crate::ui::types::*;

pub(super) fn apply_white_space(value: &str, style: &mut MailStyle) {
    // `white-space` controls two things: whether runs of whitespace are preserved,
    // and whether the text may wrap. Dropping the second made `nowrap` content
    // wrappable, so a button or code label could be squeezed to one character wide.
    match clean_css_value(value).as_str() {
        "pre" => {
            style.preserve_whitespace = true;
            style.nowrap = true;
        }
        "pre-wrap" | "pre-line" => {
            style.preserve_whitespace = true;
            style.nowrap = false;
        }
        "nowrap" => {
            style.preserve_whitespace = false;
            style.nowrap = true;
        }
        "normal" => {
            style.preserve_whitespace = false;
            style.nowrap = false;
        }
        _ => {}
    }
}

pub(super) fn apply_background(value: &str, style: &mut MailStyle) {
    if let Some(color) = parse_css_color(value) {
        style.background_color = Some(color);
    }
    if let Some(url) = parse_css_url(value) {
        style.background_image_url = Some(url);
    }
}

pub(super) fn parse_css_background_size(value: &str) -> Option<MailBackgroundSize> {
    clean_css_value(value)
        .to_ascii_lowercase()
        .split(|character: char| character.is_ascii_whitespace() || matches!(character, '/' | ','))
        .find_map(|part| match part {
            "contain" => Some(MailBackgroundSize::Contain),
            "cover" => Some(MailBackgroundSize::Cover),
            _ => None,
        })
}

pub(super) fn parse_css_url(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_end_matches("!important").trim();
    let lower = trimmed.to_ascii_lowercase();
    let start = lower.find("url(")? + 4;
    let end = trimmed[start..].find(')').map(|offset| start + offset)?;
    let url = trimmed[start..end]
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_string();
    safe_image_url(&url).then_some(url)
}
