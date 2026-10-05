use gpui::{
    div, point, px, rgb, AnyElement, BoxShadow, Div, FontWeight, IntoElement, KeyDownEvent,
    ParentElement, Styled,
};

use app_model::SurfaceTheme;

use crate::{alpha, rgba};

pub fn command_menu_panel_shell(theme: impl Into<SurfaceTheme>) -> Div {
    let theme = theme.into();
    div()
        .w(px(258.0))
        .rounded(px(10.0))
        .border_1()
        .border_color(rgba(theme.surface_border))
        .bg(rgb(theme.elevated_surface_bg))
        .shadow(vec![BoxShadow {
            color: rgba(theme.card_shadow),
            offset: point(px(0.0), px(12.0)),
            blur_radius: px(28.0),
            spread_radius: px(-10.0),
            inset: false,
        }])
        .pt(px(6.0))
        .pb(px(6.0))
}

pub fn command_menu_section_title(theme: impl Into<SurfaceTheme>, title: &'static str) -> Div {
    let theme = theme.into();
    div()
        .px(px(12.0))
        .pb(px(6.0))
        .text_size(px(11.0))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(theme.text_hint))
        .child(title)
}

pub fn command_menu_empty_state(theme: impl Into<SurfaceTheme>, label: &'static str) -> AnyElement {
    let theme = theme.into();
    div()
        .mx(px(6.0))
        .rounded(px(6.0))
        .px(px(8.0))
        .py(px(6.0))
        .text_size(px(12.0))
        .text_color(rgb(theme.text_hint))
        .child(label)
        .into_any_element()
}

pub fn command_menu_row_shell(theme: impl Into<SurfaceTheme>, is_selected: bool) -> Div {
    let theme = theme.into();
    div()
        .mx(px(6.0))
        .h(px(30.0))
        .rounded(px(6.0))
        .px(px(8.0))
        .flex()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .bg(if is_selected {
            rgba(theme.command_menu_active_bg)
        } else {
            alpha(theme.elevated_surface_bg, 0.0)
        })
}

pub fn keystroke_input_text(event: &KeyDownEvent) -> Option<&str> {
    let modifiers = &event.keystroke.modifiers;
    if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
        return None;
    }
    if let Some(key_char) = event.keystroke.key_char.as_deref() {
        return (!key_char.is_empty()).then_some(key_char);
    }
    match event.keystroke.key.as_str() {
        "space" => Some(" "),
        key if key.chars().count() == 1 => Some(key),
        _ => None,
    }
}
