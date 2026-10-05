use super::{mail_account_display_name, mail_account_initials};
use crate::ui::{
    alpha, div, if_light, mail_palette, px, AnyElement, Context, Div, FontWeight,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Styled,
    SurfaceState,
};

impl SurfaceState {
    pub(crate) fn render_mail_account_chip(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(account) = self.mail_displayed_account_info() else {
            return div().into_any_element();
        };
        let palette = mail_palette(self.appearance_mode);
        let label = account
            .address
            .clone()
            .unwrap_or_else(|| mail_account_display_name(&account));
        div()
            .id("mail-account-chip")
            .debug_selector(|| "mail-account-chip".to_string())
            .h(px(48.0))
            .w_full()
            .px(px(52.5))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(10.0))
            .cursor_pointer()
            .hover(|style| {
                style.bg(alpha(
                    palette.text_rgb,
                    if_light(self.appearance_mode, 0.035, 0.07),
                ))
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_mail_account_palette(cx);
                    cx.focus_self(window);
                }),
            )
            .child(self.render_mail_account_chip_avatar(mail_account_initials(&account)))
            .child(self.render_mail_account_chip_label(label))
            .child(self.render_mail_account_chip_disclosure())
            .into_any_element()
    }

    fn render_mail_account_chip_avatar(&self, initials: String) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .size(px(24.0))
            .rounded_full()
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .bg(alpha(palette.text_rgb, 0.11))
            .text_size(px(10.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(alpha(palette.text_rgb, 0.72))
            .child(initials)
    }

    fn render_mail_account_chip_label(&self, label: String) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .overflow_hidden()
            .whitespace_nowrap()
            .text_ellipsis()
            .text_size(px(14.0))
            .line_height(px(20.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(alpha(palette.text_rgb, 0.76))
            .child(label)
    }

    fn render_mail_account_chip_disclosure(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_none()
            .text_size(px(9.0))
            .text_color(alpha(palette.text_rgb, 0.34))
            .child("▾")
    }
}
