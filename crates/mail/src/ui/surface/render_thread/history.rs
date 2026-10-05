use super::{
    alpha, div, if_light, mail_palette, point, px, rgb, AnyElement, BoxShadow, Context, Div,
    FluentBuilder, FontWeight, InteractiveElement, IntoElement, MailPalette,
    MailThreadMessageDetail, MouseButton, MouseDownEvent, ParentElement, Styled, SurfaceState,
};

struct MailHistoryTextSpec<'a> {
    text: &'a str,
    width: f32,
    weight: FontWeight,
    opacity: f32,
}

impl SurfaceState {
    pub(crate) fn render_mail_open_thread_history_row(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let hovered = self.mail_hovered_history_message_id.as_deref() == Some(message.id.as_str());
        let hover_message_id = message.id.clone();
        let click_message_id = message.id.clone();
        let selector_thread_id = thread_id.to_string();
        let selector_message_id = message.id.clone();
        let mut row = div()
            .id(format!(
                "mail-thread-history-{selector_thread_id}-{selector_message_id}",
            ))
            .debug_selector(move || {
                format!("mail-thread-history-{selector_thread_id}-{selector_message_id}")
            })
            .h(px(40.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(alpha(
                palette.text_rgb,
                if hovered {
                    if_light(self.appearance_mode, 0.06, 0.08)
                } else {
                    0.0
                },
            ))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.0))
            .px(px(24.0))
            .cursor_pointer()
            .when(hovered, |this| {
                this.bg(rgb(palette.message_card_bg))
                    .shadow(self.mail_history_hover_shadow())
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.expand_mail_history_message(click_message_id.clone(), cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .child(self.render_mail_history_sender_preview(message, hovered, &palette))
            .child(self.render_mail_history_timestamp(message, hovered, &palette));
        row.interactivity()
            .on_hover(cx.listener(move |this, is_hovered, _, cx| {
                this.set_mail_history_message_hover(hover_message_id.clone(), *is_hovered, cx);
            }));
        row.into_any_element()
    }

    fn mail_history_hover_shadow(&self) -> Vec<BoxShadow> {
        vec![
            BoxShadow {
                color: alpha(0x000000, if_light(self.appearance_mode, 0.12, 0.28)),
                offset: point(px(0.0), px(2.0)),
                blur_radius: px(8.0),
                spread_radius: px(-2.0),
                inset: false,
            },
            BoxShadow {
                color: alpha(0x000000, if_light(self.appearance_mode, 0.08, 0.18)),
                offset: point(px(0.0), px(0.0)),
                blur_radius: px(0.5),
                spread_radius: px(0.0),
                inset: false,
            },
        ]
    }

    fn render_mail_history_sender_preview(
        &self,
        message: &MailThreadMessageDetail,
        hovered: bool,
        palette: &MailPalette,
    ) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(18.0))
            .min_w(px(0.0))
            .child(self.render_mail_history_text(
                MailHistoryTextSpec {
                    text: message.sender.as_str(),
                    width: 78.0,
                    weight: FontWeight::MEDIUM,
                    opacity: if hovered { 0.9 } else { 0.82 },
                },
                palette,
            ))
            .child(
                self.render_mail_history_text(
                    MailHistoryTextSpec {
                        text: message.preview.as_str(),
                        width: 0.0,
                        weight: FontWeight::NORMAL,
                        opacity: if hovered { 0.62 } else { 0.5 },
                    },
                    palette,
                )
                .flex_grow(1.0),
            )
    }

    fn render_mail_history_text(
        &self,
        spec: MailHistoryTextSpec<'_>,
        palette: &MailPalette,
    ) -> Div {
        let mut element = div()
            .min_w(px(0.0))
            .overflow_hidden()
            .text_size(px(12.0))
            .font_weight(spec.weight)
            .text_color(alpha(palette.text_rgb, spec.opacity))
            .text_ellipsis()
            .child(spec.text.to_string());
        if spec.width > 0.0 {
            element = element.w(px(spec.width)).flex_none();
        }
        element
    }

    fn render_mail_history_timestamp(
        &self,
        message: &MailThreadMessageDetail,
        hovered: bool,
        palette: &MailPalette,
    ) -> Div {
        div()
            .flex_none()
            .text_size(px(11.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(
                palette.text_rgb,
                if_light(
                    self.appearance_mode,
                    if hovered { 0.58 } else { 0.5 },
                    if hovered { 0.42 } else { 0.32 },
                ),
            ))
            .child(message.display_timestamp.clone())
    }
}
