use crate::ui::types::{
    MailFontFamily, MailFontStyle, MailTextDecoration, MailTextTransform, MailWordBreak,
};

use super::clean_css_value;

pub(super) fn parse_css_font_family(value: &str) -> Option<MailFontFamily> {
    let value = clean_css_value(value);
    // Each family is quoted on its own: `'Helvetica Neue', Helvetica` names
    // two faces, and the first has to be read without its quotes.
    for family in value
        .split(',')
        .map(|family| family.trim().trim_matches(|c| c == '"' || c == '\'').trim())
    {
        let supported = match family {
            "arial" => Some(MailFontFamily::Arial),
            "helvetica" => Some(MailFontFamily::Helvetica),
            "helvetica neue" | "helveticaneue" => Some(MailFontFamily::HelveticaNeue),
            "sans-serif" => Some(MailFontFamily::SansSerif),
            "serif" => Some(MailFontFamily::Serif),
            "verdana" => Some(MailFontFamily::Verdana),
            "geneva" => Some(MailFontFamily::Geneva),
            "tahoma" => Some(MailFontFamily::Tahoma),
            "trebuchet ms" => Some(MailFontFamily::TrebuchetMs),
            "georgia" => Some(MailFontFamily::Georgia),
            "lucida grande" => Some(MailFontFamily::LucidaGrande),
            "courier new" => Some(MailFontFamily::CourierNew),
            "monospace" | "courier" => Some(MailFontFamily::Monospace),
            _ => None,
        };
        if supported.is_some() {
            return supported;
        }
    }
    // CSS-wide keywords name no font; the inherited family stands.
    (!value.is_empty() && !matches!(value.as_str(), "inherit" | "initial" | "unset" | "revert"))
        .then_some(MailFontFamily::Other)
}

pub(super) fn parse_css_font_style(value: &str) -> Option<MailFontStyle> {
    match clean_css_value(value).as_str() {
        "italic" | "oblique" => Some(MailFontStyle::Italic),
        "normal" => Some(MailFontStyle::Normal),
        _ => None,
    }
}

pub(super) fn parse_css_text_decoration(value: &str) -> Option<MailTextDecoration> {
    let value = clean_css_value(value);
    if value == "none" {
        return Some(MailTextDecoration::default());
    }
    let mut decoration = MailTextDecoration::default();
    for part in value.split_whitespace() {
        match part {
            "underline" => decoration.underline = true,
            "line-through" => decoration.line_through = true,
            _ => {}
        }
    }
    (decoration.underline || decoration.line_through).then_some(decoration)
}

pub(super) fn parse_css_text_transform(value: &str) -> Option<MailTextTransform> {
    match clean_css_value(value).as_str() {
        "uppercase" => Some(MailTextTransform::Uppercase),
        "lowercase" => Some(MailTextTransform::Lowercase),
        "capitalize" => Some(MailTextTransform::Capitalize),
        "none" => Some(MailTextTransform::None),
        _ => None,
    }
}

pub(super) fn parse_css_word_break(value: &str) -> Option<MailWordBreak> {
    match clean_css_value(value).as_str() {
        "break-word" => Some(MailWordBreak::BreakWord),
        "break-all" => Some(MailWordBreak::BreakAll),
        "normal" => Some(MailWordBreak::Normal),
        _ => None,
    }
}
