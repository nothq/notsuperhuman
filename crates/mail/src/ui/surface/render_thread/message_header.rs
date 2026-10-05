use super::{
    alpha, div, mail_palette, px, AnyElement, Context, Div, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailMessageHeaderAddress, MailThreadMessageDetail,
    MouseButton, MouseDownEvent, ParentElement, Styled, SurfaceState, MAIL_FONT_FAMILY,
};
use gpui::{font, img, KeyDownEvent, Role, StatefulInteractiveElement};

use super::card::mail_message_card_id;

impl SurfaceState {
    pub(crate) fn render_mail_open_message_card_meta(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self
            .mail_expanded_message_header_ids
            .contains(message.id.as_str())
        {
            self.render_mail_expanded_message_header(thread_id, message, cx)
                .into_any_element()
        } else {
            self.render_mail_compact_message_header(thread_id, message, cx)
                .into_any_element()
        }
    }

    fn render_mail_compact_message_header(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let message_card_id = mail_message_card_id(thread_id, message.id.as_str());
        let controls_visible = self.mail_message_card_controls_visible(message_card_id.as_str());
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.0))
            .child(self.render_mail_compact_message_participants(thread_id, message, cx))
            .child(self.render_mail_open_message_card_right_meta(
                thread_id,
                message,
                controls_visible,
                cx,
            ))
    }

    fn render_mail_compact_message_participants(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let palette = mail_palette(self.appearance_mode);
        let click_message_id = message.id.clone();
        let key_message_id = message.id.clone();
        div()
            .id(format!("mail-message-header-{thread_id}-{}", message.id))
            .role(Role::Button)
            .aria_label("Expand message header")
            .focusable()
            .tab_stop(true)
            .min_w(px(0.0))
            .overflow_hidden()
            .flex()
            .items_center()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(15.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(palette.text_rgb, 0.8))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.toggle_mail_message_header(click_message_id.clone(), cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if !matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    return;
                }
                window.prevent_default();
                cx.stop_propagation();
                this.toggle_mail_message_header(key_message_id.clone(), cx);
                this.mail_shortcuts_focused = true;
                cx.focus_self(window);
            }))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(message.header.compact_label.clone()),
            )
            .child(self.render_mail_message_header_icon(false, cx))
    }

    fn render_mail_expanded_message_header(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        div()
            .id(format!(
                "mail-message-header-details-{thread_id}-{}",
                message.id
            ))
            .w_full()
            .flex()
            .flex_col()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(15.0))
            .line_height(px(24.0))
            .font_weight(FontWeight::BOLD)
            .when(!message.header.from.is_empty(), |this| {
                this.child(
                    self.render_mail_message_header_address_row(
                        "From",
                        message.header.from.as_ref(),
                    ),
                )
            })
            .when(!message.header.to.is_empty(), |this| {
                this.child(
                    self.render_mail_message_header_address_row("To", message.header.to.as_ref()),
                )
            })
            .when(!message.header.cc.is_empty(), |this| {
                this.child(
                    self.render_mail_message_header_address_row("Cc", message.header.cc.as_ref()),
                )
            })
            .when(!message.header.bcc.is_empty(), |this| {
                this.child(
                    self.render_mail_message_header_address_row("Bcc", message.header.bcc.as_ref()),
                )
            })
            .child(self.render_mail_message_header_timestamp(message, cx))
    }

    fn render_mail_message_header_address_row(
        &self,
        label: &'static str,
        addresses: &[MailMessageHeaderAddress],
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w_full()
            .flex()
            .items_start()
            .child(
                div()
                    .w(px(60.0))
                    .flex_none()
                    .pr(px(15.0))
                    .text_color(alpha(palette.text_rgb, 0.5))
                    .child(label),
            )
            .child(div().min_w(px(0.0)).flex().flex_wrap().children(
                addresses.iter().enumerate().map(|(index, address)| {
                    self.render_mail_message_header_address(address, index + 1 < addresses.len())
                }),
            ))
    }

    fn render_mail_message_header_address(
        &self,
        address: &MailMessageHeaderAddress,
        trailing_comma: bool,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_none()
            .flex()
            .items_center()
            .whitespace_nowrap()
            .mr(px(if trailing_comma { 4.0 } else { 0.0 }))
            .text_color(alpha(palette.text_rgb, 0.8))
            .child(address.name.clone())
            .when_some(address.email.clone(), |this, email| {
                this.child(div().text_color(alpha(palette.text_rgb, 0.5)).child(email))
            })
            .when(trailing_comma, |this| {
                this.child(div().text_color(alpha(palette.text_rgb, 0.5)).child(","))
            })
    }

    fn render_mail_message_header_timestamp(
        &self,
        message: &MailThreadMessageDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let click_message_id = message.id.clone();
        let key_message_id = message.id.clone();
        div()
            .flex()
            .items_center()
            .text_color(alpha(palette.text_rgb, 0.5))
            .child(message.header.full_timestamp.clone())
            .child(
                div()
                    .id(format!("mail-message-header-collapse-{}", message.id))
                    .role(Role::Button)
                    .aria_label("Collapse message header")
                    .focusable()
                    .tab_stop(true)
                    .ml(px(4.0))
                    .size(px(24.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.toggle_mail_message_header(click_message_id.clone(), cx);
                            this.mail_shortcuts_focused = true;
                            cx.focus_self(window);
                        }),
                    )
                    .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                        if !matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            return;
                        }
                        window.prevent_default();
                        cx.stop_propagation();
                        this.toggle_mail_message_header(key_message_id.clone(), cx);
                        this.mail_shortcuts_focused = true;
                        cx.focus_self(window);
                    }))
                    .child(self.render_mail_message_header_icon(true, cx)),
            )
    }

    fn render_mail_message_header_icon(
        &self,
        expanded: bool,
        cx: &mut Context<Self>,
    ) -> impl gpui::IntoElement {
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        let body = if expanded {
            format!(
                r##"<path d="M1 0L5 4L9 0M1 12L5 8L9 12" fill="none" stroke="#{icon:06X}" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>"##
            )
        } else {
            format!(
                r##"<path d="M1 4L5 0L9 4M1 8L5 12L9 8" fill="none" stroke="#{icon:06X}" stroke-width="1" stroke-linecap="round" stroke-linejoin="round"/>"##
            )
        };
        div()
            .size(px(24.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .child(
                img(self.render_mail_svg_icon("0 0 10 12", body, cx))
                    .w(px(10.0))
                    .h(px(12.0))
                    .opacity(0.3),
            )
    }

    fn mail_message_card_controls_visible(&self, message_card_id: &str) -> bool {
        self.mail_hovered_message_card_id.as_deref() == Some(message_card_id)
            || self.mail_hovered_message_action.as_ref().is_some_and(
                |(hovered_message_card_id, _)| hovered_message_card_id == message_card_id,
            )
    }

    fn render_mail_open_message_card_right_meta(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        controls_visible: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(11.0))
            .when(controls_visible, |this| {
                this.child(self.render_mail_message_action_group(
                    thread_id,
                    message.id.as_str(),
                    cx,
                ))
            })
            .child(
                div()
                    .flex_none()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(palette.text_rgb, 0.5))
                    .child(message.display_timestamp.clone()),
            )
    }
}
