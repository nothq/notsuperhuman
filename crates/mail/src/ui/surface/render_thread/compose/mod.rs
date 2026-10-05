use super::super::{
    alpha, div, if_light, img, mail_address_summary, mail_palette, mail_parse_addresses, point, px,
    rgb, AnyElement, AppearanceMode, BoxShadow, Context, Div, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailComposeAutocompleteItem, MailComposeField,
    MailComposeMode, MailPalette, MouseButton, MouseDownEvent, ParentElement,
    StatefulInteractiveElement, Styled, SurfaceState,
};

mod actions;
mod fields;
mod open_thread;
mod panel;
mod suggestions;

struct MailComposeRecipientRowRequest {
    field: MailComposeField,
    label: &'static str,
    placeholder: &'static str,
    full_view: bool,
}

impl SurfaceState {
    pub(crate) fn render_mail_compose_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .bg(rgb(palette.viewer_bg))
            .child(
                div()
                    .relative()
                    .h_full()
                    .min_w(px(0.0))
                    .flex_basis(px(0.0))
                    .flex_grow(5.25)
                    .flex_shrink_0()
                    .child(self.render_mail_compose_back_button(cx)),
            )
            .child(
                div()
                    .h_full()
                    .min_w(px(0.0))
                    .flex_basis(px(760.0))
                    .flex_grow(1.0)
                    .flex_shrink(1.0)
                    .flex()
                    .flex_col()
                    .child(self.render_mail_compose_standalone_header())
                    .child(
                        div()
                            .id("mail-compose-view-scroll")
                            .flex_grow(1.0)
                            .min_h(px(0.0))
                            .overflow_scroll()
                            .w_full()
                            .child(
                                div()
                                    .w_full()
                                    .max_w(px(700.0))
                                    .mx_auto()
                                    .pb(px(28.0))
                                    .child(self.render_mail_compose_panel(true, cx)),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_mail_compose_back_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        (div()
            .id("mail-compose-back".to_string())
            .absolute()
            .left(px(30.0))
            .top(px(25.0))
            .w(px(32.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                img(self.render_mail_svg_icon(
                    "0 0 16 16",
                    format!(
                        r##"<path d="M8.75 1L1.75 8.00011M1.75 8.00011L8.75 15M1.75 8.00011L14.25 8" fill="none" stroke="#{icon:06X}" stroke-linecap="round" stroke-linejoin="round"></path>"##
                    ),
                    cx,
                ))
                .opacity(0.3)
                .size(px(16.0)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.close_mail_compose_view(cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            ))
        .into_any_element()
    }
}
