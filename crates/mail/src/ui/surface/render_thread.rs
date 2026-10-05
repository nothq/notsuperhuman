use super::{
    alpha, div, if_light, img, mail_address_summary, mail_palette, mail_parse_addresses, point, px,
    rgb, AnyElement, BoxShadow, Context, Div, FluentBuilder, FontWeight, InteractiveElement,
    IntoElement, MailComposeField, MailMessageAction, MailMessageHeaderAddress,
    MailPalette, MailThreadMessageDetail, MouseButton, MouseDownEvent,
    ParentElement, Styled, SurfaceState, MAIL_FONT_FAMILY,
};

mod body;
pub(crate) mod card;
mod compose;
mod header;
mod history;
mod inline_compose;
mod inline_compose_from;
mod inline_compose_icons;
mod message_actions;
mod message_header;

impl SurfaceState {
    pub(crate) fn render_mail_open_thread_view(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let Some(thread) = self.mail_open_thread_detail() else {
            return div()
                .flex_grow(1.0)
                .min_h(px(0.0))
                .flex()
                .items_center()
                .justify_center()
                .bg(rgb(palette.viewer_bg))
                .child(
                    div()
                        .text_size(px(13.0))
                        .text_color(alpha(palette.text_rgb, 0.48))
                        .child("Loading thread…"),
                )
                .into_any_element();
        };
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
                    .child(self.render_mail_open_thread_back_button(cx)),
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
                    .child(self.render_mail_open_thread_header(&thread, cx))
                    .child(
                        div()
                            .id("mail-thread-view-scroll")
                            .flex_grow(1.0)
                            .min_h(px(0.0))
                            .overflow_hidden()
                            .child(self.render_mail_open_thread_body(thread.clone(), cx)),
                    )
                    .child(self.render_mail_open_thread_composer(cx)),
            )
            .into_any_element()
    }
}
