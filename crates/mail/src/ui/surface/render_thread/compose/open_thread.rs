use super::super::super::{
    alpha, div, if_light, mail_palette, px, rgb, Context, Div, FontWeight, MailComposeMode,
    MailPalette, ParentElement, Styled, SurfaceState, MAIL_OPEN_COMPOSER_HEIGHT,
};

impl SurfaceState {
    pub(in crate::ui::surface::render_thread) fn render_mail_open_thread_composer(
        &self,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        if self
            .mail_open_thread_id
            .as_deref()
            .is_some_and(|thread_id| self.open_thread_inline_compose_visible(thread_id))
        {
            return div();
        }
        if self.mail_compose_mode == MailComposeMode::Closed {
            return self.render_mail_closed_thread_share_composer();
        }
        div()
            .border_t_1()
            .border_color(rgb(palette.viewer_border))
            .bg(rgb(palette.composer_bg))
            .child(
                div()
                    .w_full()
                    .max_w(px(760.0))
                    .mx_auto()
                    .px(px(30.0))
                    .py(px(14.0))
                    .child(self.render_mail_compose_panel(false, cx)),
            )
    }

    pub(super) fn render_mail_closed_thread_share_composer(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .h(px(MAIL_OPEN_COMPOSER_HEIGHT))
            .min_h(px(MAIL_OPEN_COMPOSER_HEIGHT))
            .border_t_1()
            .border_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.05, 0.06),
            ))
            .bg(rgb(palette.composer_bg))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(762.0))
                    .h_full()
                    .px(px(30.0))
                    .flex()
                    .items_center()
                    .child(self.render_mail_closed_thread_share_box(&palette)),
            )
    }

    fn render_mail_closed_thread_share_box(&self, palette: &MailPalette) -> Div {
        div()
            .w_full()
            .h(px(41.0))
            .rounded(px(5.0))
            .border_1()
            .border_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.1, 0.12),
            ))
            .bg(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.015, 0.05),
            ))
            .px(px(14.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .child(self.render_mail_closed_thread_share_placeholder(palette))
            .child(self.render_mail_closed_thread_share_send(palette))
    }

    fn render_mail_closed_thread_share_placeholder(&self, palette: &MailPalette) -> Div {
        div()
            .min_w(px(0.0))
            .overflow_hidden()
            .text_size(px(13.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.4, 0.36),
            ))
            .text_ellipsis()
            .child("@mention anyone and share conversation")
    }

    fn render_mail_closed_thread_share_send(&self, palette: &MailPalette) -> Div {
        div()
            .size(px(22.0))
            .flex_none()
            .rounded_full()
            .border_1()
            .border_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.1, 0.13),
            ))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.5, 0.46),
            ))
            .child("↑")
    }
}
