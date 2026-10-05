use super::{
    alpha, div, if_light, img, mail_palette, px, rgb, AnyElement, AppearanceMode, Context, Div,
    FontWeight, InteractiveElement, IntoElement, MouseButton,
    MouseDownEvent, ParentElement, Styled, SurfaceState,
};

impl SurfaceState {
    pub(super) fn render_mail_compose_attachments(&self, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(alpha(palette.text_rgb, 0.42))
                    .child("Attachments"),
            )
            .children(
                self.mail_compose_attachments
                    .iter()
                    .map(|attachment| self.render_mail_compose_attachment_row(attachment, cx)),
            )
    }

    fn render_mail_compose_attachment_row(
        &self,
        attachment: &crate::model::MailAttachment,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let attachment_name = attachment.name.clone();
        div()
            .px(px(10.0))
            .py(px(8.0))
            .rounded(px(8.0))
            .bg(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.05, 0.08),
            ))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(alpha(palette.text_rgb, 0.76))
                    .child(format!(
                        "{}  {}",
                        attachment.name,
                        crate::ui::format_attachment_size(attachment.size)
                    )),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.52, 0.46),
                    ))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                            this.remove_mail_attachment(attachment_name.as_str(), cx);
                        }),
                    )
                    .child("Remove"),
            )
    }

    pub(super) fn render_mail_compose_actions(
        &self,
        full_view: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        if full_view {
            return self.render_mail_compose_full_actions(cx);
        }
        let palette = mail_palette(self.appearance_mode);
        div()
            .pt(px(20.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(14.0))
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.38, 0.32),
                    ))
                    .child(if self.mail_compose_pending {
                        "Syncing draft…"
                    } else {
                        "Draft sync stays live as you type"
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(self.render_mail_compose_attach_button(cx))
                    .child(self.render_mail_compose_discard_button(cx))
                    .child(self.render_mail_compose_send_button(cx)),
            )
    }

    fn render_mail_compose_full_actions(&self, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .h(px(68.0))
            .min_h(px(68.0))
            .flex_none()
            .pt(px(16.0))
            .child(
                div()
                    .h(px(52.0))
                    .border_t_1()
                    .border_color(alpha(palette.text_rgb, 0.18))
                    .pt(px(16.0))
                    .flex()
                    .items_start()
                    .child(self.render_mail_compose_full_send_action(cx))
                    .child(self.render_mail_compose_full_secondary_action("Send later"))
                    .child(self.render_mail_compose_full_secondary_action("Remind me"))
                    .child(self.render_mail_compose_full_secondary_action("Share draft"))
                    .child(div().flex_grow(1.0))
                    .child(self.render_mail_compose_full_text_icon("ai"))
                    .child(self.render_mail_compose_full_calendar_icon())
                    .child(self.render_mail_compose_full_text_icon("{ }"))
                    .child(self.render_mail_compose_full_attach_icon(cx))
                    .child(self.render_mail_compose_full_discard_icon(cx)),
            )
    }

    fn render_mail_compose_full_send_action(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        (div()
            .h(px(36.0))
            .pr(px(18.0))
            .cursor_pointer()
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(palette.text_rgb, 0.8))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.send_mail_draft(cx);
                }),
            )
            .child(if self.mail_compose_pending {
                "Sending…"
            } else {
                "Send"
            }))
        .into_any_element()
    }

    fn render_mail_compose_full_secondary_action(&self, label: &'static str) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .h(px(36.0))
            .pr(px(18.0))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(palette.text_rgb, 0.35))
            .child(label)
    }

    fn render_mail_compose_full_text_icon(&self, label: &'static str) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w(px(42.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_start()
            .justify_center()
            .text_size(px(14.0))
            .line_height(px(18.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(palette.text_rgb, 0.3))
            .child(label)
    }

    fn render_mail_compose_full_calendar_icon(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w(px(42.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_start()
            .justify_center()
            .child(
                div()
                    .mt(px(1.0))
                    .w(px(14.0))
                    .h(px(14.0))
                    .border_1()
                    .border_color(alpha(palette.text_rgb, 0.3))
                    .text_size(px(7.0))
                    .line_height(px(12.0))
                    .text_color(alpha(palette.text_rgb, 0.3))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("31"),
            )
    }

    fn render_mail_compose_full_attach_icon(&self, cx: &mut Context<Self>) -> AnyElement {
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        (div()
            .id("mail-compose-attach".to_string())
            .w(px(42.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_start()
            .justify_center()
            .cursor_pointer()
            .child(
                img(self.render_mail_svg_icon(
                    "0 0 18 18",
                    format!(
                        r##"<path d="M6.25 9.75L10.95 5.05C12.05 3.95 13.85 3.95 14.95 5.05C16.05 6.15 16.05 7.95 14.95 9.05L8.55 15.45C6.9 17.1 4.2 17.1 2.55 15.45C0.9 13.8 0.9 11.1 2.55 9.45L9.05 2.95" fill="none" stroke="#{icon:06X}" stroke-width="1.2" stroke-linecap="round"/>"##
                    ),
                    cx,
                ))
                .mt(px(1.0))
                .opacity(0.3)
                .size(px(16.0)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    this.prompt_for_mail_attachment_files(window, cx);
                }),
            ))
        .into_any_element()
    }

    fn render_mail_compose_full_discard_icon(&self, cx: &mut Context<Self>) -> AnyElement {
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        (div()
            .id("mail-compose-discard".to_string())
            .w(px(20.0))
            .h(px(32.0))
            .flex_none()
            .flex()
            .items_start()
            .justify_end()
            .cursor_pointer()
            .child(
                img(self.render_mail_svg_icon(
                    "0 0 16 16",
                    format!(
                        r##"<path d="M3 4.5H13M6 2.5H10M4.5 4.5L5 14H11L11.5 4.5M7 7V11.5M9 7V11.5" fill="none" stroke="#{icon:06X}" stroke-width="1.1" stroke-linecap="round" stroke-linejoin="round"/>"##
                    ),
                    cx,
                ))
                .mt(px(1.0))
                .opacity(0.3)
                .size(px(16.0)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.discard_mail_draft(cx);
                }),
            ))
        .into_any_element()
    }

    fn render_mail_compose_attach_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        (div()
            .h(px(34.0))
            .px(px(12.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.1, 0.12),
            ))
            .text_size(px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(alpha(palette.text_rgb, 0.74))
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    this.prompt_for_mail_attachment_files(window, cx);
                }),
            )
            .child("Add Files"))
        .into_any_element()
    }

    fn render_mail_compose_discard_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        (div()
            .h(px(34.0))
            .px(px(12.0))
            .rounded(px(9.0))
            .border_1()
            .border_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.1, 0.12),
            ))
            .text_size(px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.52, 0.46),
            ))
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.discard_mail_draft(cx);
                }),
            )
            .child("Discard"))
        .into_any_element()
    }

    fn render_mail_compose_send_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let primary_bg = if self.appearance_mode == AppearanceMode::Light {
            0x20242a
        } else {
            0xf2f3f6
        };
        let primary_text = if self.appearance_mode == AppearanceMode::Light {
            0xffffff
        } else {
            0x13161a
        };
        (div()
            .h(px(34.0))
            .px(px(14.0))
            .rounded(px(9.0))
            .bg(alpha(
                primary_bg,
                if self.mail_compose_pending { 0.56 } else { 1.0 },
            ))
            .text_size(px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(primary_text))
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.send_mail_draft(cx);
                }),
            )
            .child(if self.mail_compose_pending {
                "Sending…"
            } else {
                "Send"
            }))
        .into_any_element()
    }
}
