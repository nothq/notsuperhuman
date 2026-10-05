use super::{
    alpha, div, mail_palette, px, Div, FluentBuilder, FontWeight, ParentElement, Styled,
    SurfaceState,
};

impl SurfaceState {
    pub(super) fn render_inline_reply_from(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .pt(px(8.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .text_size(px(11.5))
            .line_height(px(16.0))
            .child(
                div()
                    .font_weight(FontWeight::BOLD)
                    .text_color(alpha(palette.text_rgb, 0.38))
                    .child("From"),
            )
            .when_some(self.mail_compose_from.as_ref(), |this, from| {
                let label = if from.name.trim().is_empty() {
                    from.email.clone()
                } else {
                    format!("{} <{}>", from.name, from.email)
                };
                this.child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_color(alpha(palette.text_rgb, 0.72))
                        .child(label),
                )
            })
            .when(self.mail_compose_from.is_none(), |this| {
                this.child(
                    div()
                        .h(px(7.0))
                        .w(px(144.0))
                        .rounded(px(4.0))
                        .bg(alpha(palette.text_rgb, 0.07)),
                )
            })
    }
}
