use crate::ui::types::{
    MailBackgroundPosition, MailBackgroundRepeat, MailBorderCollapse, MailBoxSizing, MailDirection,
    MailDisplay, MailOverflow, MailPosition, MailVerticalAlign,
};

use super::clean_css_value;

pub(super) fn parse_css_direction(value: &str) -> Option<MailDirection> {
    match clean_css_value(value).as_str() {
        "ltr" => Some(MailDirection::Ltr),
        "rtl" => Some(MailDirection::Rtl),
        _ => None,
    }
}

pub(super) fn parse_css_display(value: &str) -> Option<MailDisplay> {
    match clean_css_value(value).as_str() {
        "none" => Some(MailDisplay::None),
        "inline" => Some(MailDisplay::Inline),
        "inline-block" => Some(MailDisplay::InlineBlock),
        "block" => Some(MailDisplay::Block),
        "table" => Some(MailDisplay::Table),
        "table-cell" => Some(MailDisplay::TableCell),
        _ => None,
    }
}

pub(super) fn parse_css_vertical_align(value: &str) -> Option<MailVerticalAlign> {
    match clean_css_value(value).as_str() {
        "top" => Some(MailVerticalAlign::Top),
        "middle" => Some(MailVerticalAlign::Middle),
        "bottom" => Some(MailVerticalAlign::Bottom),
        "baseline" => Some(MailVerticalAlign::Baseline),
        _ => None,
    }
}

pub(super) fn parse_css_border_collapse(value: &str) -> Option<MailBorderCollapse> {
    match clean_css_value(value).as_str() {
        "collapse" => Some(MailBorderCollapse::Collapse),
        "separate" => Some(MailBorderCollapse::Separate),
        _ => None,
    }
}

pub(super) fn parse_css_box_sizing(value: &str) -> Option<MailBoxSizing> {
    match clean_css_value(value).as_str() {
        "border-box" => Some(MailBoxSizing::BorderBox),
        "content-box" => Some(MailBoxSizing::ContentBox),
        _ => None,
    }
}

pub(super) fn parse_css_overflow(value: &str) -> Option<MailOverflow> {
    match clean_css_value(value).as_str() {
        "hidden" => Some(MailOverflow::Hidden),
        "visible" => Some(MailOverflow::Visible),
        "scroll" => Some(MailOverflow::Scroll),
        "auto" => Some(MailOverflow::Auto),
        _ => None,
    }
}

pub(super) fn parse_css_position(value: &str) -> Option<MailPosition> {
    match clean_css_value(value).as_str() {
        "absolute" => Some(MailPosition::Absolute),
        "relative" => Some(MailPosition::Relative),
        "static" => Some(MailPosition::Static),
        "fixed" => Some(MailPosition::Fixed),
        _ => None,
    }
}

pub(super) fn parse_css_background_repeat(value: &str) -> Option<MailBackgroundRepeat> {
    match clean_css_value(value).as_str() {
        "no-repeat" => Some(MailBackgroundRepeat::NoRepeat),
        "repeat" => Some(MailBackgroundRepeat::Repeat),
        "repeat-x" => Some(MailBackgroundRepeat::RepeatX),
        "repeat-y" => Some(MailBackgroundRepeat::RepeatY),
        _ => None,
    }
}

pub(super) fn parse_css_background_position(value: &str) -> Option<MailBackgroundPosition> {
    match clean_css_value(value).as_str() {
        "center" | "center center" => Some(MailBackgroundPosition::Center),
        "top" | "center top" | "top center" => Some(MailBackgroundPosition::Top),
        "right" | "right center" | "center right" => Some(MailBackgroundPosition::Right),
        "bottom" | "center bottom" | "bottom center" => Some(MailBackgroundPosition::Bottom),
        "left" | "left center" | "center left" => Some(MailBackgroundPosition::Left),
        _ => None,
    }
}
