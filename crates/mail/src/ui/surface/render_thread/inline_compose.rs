use super::{
    alpha, div, if_light, img, mail_address_summary, mail_palette, mail_parse_addresses, px, rgb,
    AnyElement, Context, Div, FluentBuilder, FontWeight, InteractiveElement, IntoElement,
    MailComposeField, MouseButton, MouseDownEvent, ParentElement, Styled,
    SurfaceState, MAIL_FONT_FAMILY,
};
use crate::ui::LongFormEditorStyle;
use gpui::font;

struct InlineSvgIcon {
    view_box: &'static str,
    body: String,
    width: f32,
    height: f32,
}

impl SurfaceState {
    pub(crate) fn render_inline_reply_composer(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .id("mail-inline-reply-composer")
            .debug_selector(|| "mail-inline-reply-composer".to_string())
            .flex()
            .flex_col()
            .child(div().h(px(1.0)).w_full().bg(alpha(palette.text_rgb, 0.09)))
            .child(self.render_inline_reply_header(cx))
            .child(self.render_inline_reply_from())
            .child(self.render_inline_reply_body(cx))
            .child(self.render_inline_reply_quote_marker())
            .when_some(self.mail_compose_error.as_ref(), |this, error| {
                this.child(self.render_inline_reply_error(error.clone()))
            })
            .child(self.render_inline_reply_toolbar(cx))
            .into_any_element()
    }

    fn render_inline_reply_header(&self, cx: &mut Context<Self>) -> Div {
        div()
            .pt(px(24.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.0))
            .child(self.render_inline_reply_title())
            .child(self.render_inline_popout_button(cx))
    }

    fn render_inline_reply_title(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let recipient = mail_parse_addresses(&self.mail_compose_to)
            .first()
            .map(mail_address_summary)
            .filter(|recipient| !recipient.is_empty());
        div()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .overflow_hidden()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(14.0))
            .line_height(px(24.0))
            .font_weight(FontWeight::BOLD)
            .child(div().flex_none().text_color(rgb(0x6ebd77)).child("Draft"))
            .when_some(recipient, |this, recipient| {
                this.child(
                    div()
                        .min_w(px(0.0))
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_color(alpha(palette.text_rgb, 0.8))
                        .child(format!(" to {recipient}")),
                )
            })
    }

    fn render_inline_popout_button(&self, cx: &mut Context<Self>) -> AnyElement {
        let fill = mail_palette(self.appearance_mode).icon_rgb;
        div()
            .w(px(28.0))
            .h(px(23.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                div().opacity(0.3).child(self.render_inline_svg_icon(
                    InlineSvgIcon {
                        view_box: "0 0 12 10",
                        body: format!(
                            r##"<g fill="none" fill-rule="evenodd" stroke="#{fill:06X}"><path d="m5.5 4.5h6v5h-6z"></path><g stroke-linecap="square"><path d="m1.5.5h8"></path><path d="m9.5 1.5v2"></path><path d="m.5.5v6"></path><path d="m1.5 6.5h3"></path></g></g>"##
                        ),
                        width: 12.0,
                        height: 10.0,
                    },
                    cx,
                )),
            )
            .into_any_element()
    }

    fn render_inline_reply_body(&self, cx: &mut Context<Self>) -> Div {
        let focused = self.mail_compose_focused_field == MailComposeField::Body;
        div()
            .min_h(px(174.0))
            .pt(px(28.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.focus_mail_compose_field(MailComposeField::Body, cx);
                }),
            )
            .child(self.render_inline_reply_body_content(focused, cx))
    }

    fn render_inline_reply_body_content(
        &self,
        focused: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex()
            .items_start()
            .w_full()
            .child(
                self.mail_compose_body_editor_entity(
                    self.mail_compose_body.clone(),
                    "Tip: Hit ⌘J for AI",
                    self.inline_reply_body_editor_style(focused),
                    cx,
                )
                .into_any_element(),
            )
            .into_any_element()
    }

    fn inline_reply_body_editor_style(&self, focused: bool) -> LongFormEditorStyle {
        let palette = mail_palette(self.appearance_mode);
        LongFormEditorStyle {
            height: px(148.0),
            min_height: px(148.0),
            padding_x: px(0.0),
            padding_y: px(0.0),
            radius: px(0.0),
            background: alpha(palette.text_rgb, 0.0),
            border: alpha(palette.text_rgb, 0.0),
            focused_border: alpha(palette.text_rgb, 0.0),
            text: alpha(palette.text_rgb, if_light(self.appearance_mode, 0.88, 0.84)),
            placeholder: alpha(
                palette.text_rgb,
                if focused {
                    if_light(self.appearance_mode, 0.34, 0.3)
                } else {
                    if_light(self.appearance_mode, 0.3, 0.26)
                },
            ),
            selection: alpha(palette.compose_caret, 0.28),
            caret: rgb(palette.compose_caret).into(),
            font_size: px(14.0),
            line_height: px(21.0),
            font_family: Some(MAIL_FONT_FAMILY.into()),
            ..LongFormEditorStyle::default()
        }
    }

    fn render_inline_reply_quote_marker(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .h(px(22.0))
            .flex()
            .items_center()
            .text_size(px(16.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.32, 0.28),
            ))
            .child("...")
    }

    fn render_inline_reply_error(&self, error: String) -> Div {
        div()
            .pt(px(8.0))
            .text_size(px(12.0))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xc94f4f))
            .child(error)
    }

    fn render_inline_reply_toolbar(&self, cx: &mut Context<Self>) -> Div {
        div()
            .pt(px(76.0))
            .pb(px(5.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(22.0))
            .child(
                div()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .gap(px(18.0))
                    .child(self.render_inline_send_control(cx))
                    .child(self.render_inline_toolbar_text("Send later"))
                    .child(self.render_inline_toolbar_text("Remind me"))
                    .child(self.render_inline_toolbar_text("Share draft")),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(24.0))
                    .child(self.render_inline_ai_button(cx))
                    .child(self.render_inline_calendar_button(cx))
                    .child(self.render_inline_braces_button(cx))
                    .child(self.render_inline_attach_button(cx))
                    .child(self.render_inline_discard_button(cx)),
            )
    }

    fn render_inline_send_control(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .h(px(16.0))
            .flex_none()
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.send_mail_draft(cx);
                }),
            )
            .child(
                div()
                    .font(font(MAIL_FONT_FAMILY))
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(alpha(
                        palette.text_rgb,
                        if self.mail_compose_pending {
                            0.42
                        } else {
                            if_light(self.appearance_mode, 0.9, 0.86)
                        },
                    ))
                    .child(if self.mail_compose_pending {
                        "Sending"
                    } else {
                        "Send"
                    }),
            )
            .into_any_element()
    }

    fn render_inline_toolbar_text(&self, label: &'static str) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .h(px(16.0))
            .flex_none()
            .flex()
            .items_center()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(palette.text_rgb, 0.35))
            .cursor_pointer()
            .child(label)
    }

    fn render_inline_ai_button(&self, cx: &mut Context<Self>) -> Div {
        self.render_inline_icon_button(self.render_inline_svg_icon(
            InlineSvgIcon {
                view_box: "0 0 14 15",
                body: self.render_inline_ai_icon_body(),
                width: 12.5,
                height: 12.0,
            },
            cx,
        ))
    }

    fn render_inline_calendar_button(&self, cx: &mut Context<Self>) -> Div {
        self.render_inline_icon_button(self.render_inline_svg_icon(
            InlineSvgIcon {
                view_box: "0 0 14 14",
                body: self.render_inline_calendar_icon_body(),
                width: 14.0,
                height: 12.0,
            },
            cx,
        ))
    }

    fn render_inline_braces_button(&self, cx: &mut Context<Self>) -> Div {
        self.render_inline_icon_button(self.render_inline_svg_icon(
            InlineSvgIcon {
                view_box: "0 0 16 12",
                body: self.render_inline_snippet_icon_body(),
                width: 16.0,
                height: 12.0,
            },
            cx,
        ))
    }

    fn render_inline_icon_button(&self, icon: AnyElement) -> Div {
        div()
            .size(px(19.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(div().opacity(0.3).child(icon))
    }

    fn render_inline_attach_button(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size(px(19.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(div().opacity(0.3).child(self.render_inline_svg_icon(
                InlineSvgIcon {
                    view_box: "0 0 18 8",
                    body: self.render_inline_attach_icon_body(),
                    width: 19.0,
                    height: 8.0,
                },
                cx,
            )))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    this.prompt_for_mail_attachment_files(window, cx);
                }),
            )
            .into_any_element()
    }

    fn render_inline_discard_button(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size(px(19.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(div().opacity(0.3).child(self.render_inline_svg_icon(
                InlineSvgIcon {
                    view_box: "0 0 11 12",
                    body: self.render_inline_discard_icon_body(),
                    width: 11.0,
                    height: 12.0,
                },
                cx,
            )))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.discard_mail_draft(cx);
                }),
            )
            .into_any_element()
    }

    fn render_inline_svg_icon(&self, icon: InlineSvgIcon, cx: &mut Context<Self>) -> AnyElement {
        img(self.render_mail_svg_icon(icon.view_box, icon.body, cx))
            .w(px(icon.width))
            .h(px(icon.height))
            .into_any_element()
    }
}
