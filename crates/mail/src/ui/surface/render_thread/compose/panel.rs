use super::{
    alpha, div, if_light, mail_address_summary, mail_palette, mail_parse_addresses, point, px, rgb,
    BoxShadow, Context, Div, FluentBuilder, FontWeight, MailComposeField, MailComposeMode,
    MailComposeRecipientRowRequest, ParentElement, Styled, SurfaceState,
};

impl SurfaceState {
    pub(super) fn render_mail_compose_panel(&self, full_view: bool, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .relative()
            .rounded(px(if full_view { 4.0 } else { 0.0 }))
            .when(full_view, |this| {
                this.h(px(372.0)).shadow(vec![
                    BoxShadow {
                        color: alpha(0x000000, 0.03),
                        offset: point(px(0.0), px(0.0)),
                        blur_radius: px(12.0),
                        spread_radius: px(0.0),
                        inset: false,
                    },
                    BoxShadow {
                        color: alpha(0x000000, 0.1),
                        offset: point(px(0.0), px(12.0)),
                        blur_radius: px(24.0),
                        spread_radius: px(0.0),
                        inset: false,
                    },
                    BoxShadow {
                        color: alpha(0xffffff, 0.1),
                        offset: point(px(0.15), px(0.5)),
                        blur_radius: px(0.0),
                        spread_radius: px(0.0),
                        inset: true,
                    },
                ])
            })
            .bg(rgb(palette.compose_panel_bg))
            .child(self.render_mail_compose_panel_content(full_view, cx))
    }

    fn render_mail_compose_panel_content(&self, full_view: bool, cx: &mut Context<Self>) -> Div {
        let show_cc_row = self.mail_compose_focused_field == MailComposeField::Cc
            || !self.mail_compose_cc.is_empty();
        div()
            .when(full_view, |this| this.h_full().px(px(30.0)).pb(px(4.0)))
            .when(!full_view, |this| this.px(px(18.0)).py(px(14.0)))
            .flex()
            .flex_col()
            .when(!full_view, |this| {
                this.child(self.render_mail_compose_inline_header())
                    .child(self.render_mail_compose_from_row())
                    .child(self.render_mail_compose_divider())
            })
            .child(self.render_mail_compose_recipient_row(
                MailComposeRecipientRowRequest {
                    field: MailComposeField::To,
                    label: "To",
                    placeholder: "Type a name or email",
                    full_view,
                },
                cx,
            ))
            .when(show_cc_row, |this| {
                this.when(!full_view, |this| {
                    this.child(self.render_mail_compose_divider())
                })
                .child(self.render_mail_compose_recipient_row(
                    MailComposeRecipientRowRequest {
                        field: MailComposeField::Cc,
                        label: "Cc",
                        placeholder: "Add copy recipients",
                        full_view,
                    },
                    cx,
                ))
            })
            .when(!full_view, |this| {
                this.child(self.render_mail_compose_divider())
            })
            .child(self.render_mail_compose_subject_row(full_view, cx))
            .when(!full_view, |this| {
                this.child(self.render_mail_compose_divider())
            })
            .child(self.render_mail_compose_body_field(full_view, cx))
            .when(!self.mail_compose_attachments.is_empty(), |this| {
                this.child(
                    div()
                        .pt(px(14.0))
                        .child(self.render_mail_compose_attachments(cx)),
                )
            })
            .when_some(self.mail_compose_error.as_ref(), |this, error| {
                this.child(self.render_mail_compose_error(error.clone()))
            })
            .child(self.render_mail_compose_actions(full_view, cx))
    }

    fn render_mail_compose_error(&self, error: String) -> Div {
        div()
            .pt(px(12.0))
            .text_size(px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(0xc94f4f))
            .child(error)
    }

    pub(super) fn render_mail_compose_standalone_header(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w_full()
            .max_w(px(760.0))
            .h(px(76.0))
            .min_h(px(76.0))
            .mx_auto()
            .px(px(60.0))
            .pt(px(24.0))
            .pb(px(20.0))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(24.0))
            .child(
                div()
                    .text_size(px(21.0))
                    .line_height(px(32.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(palette.text_rgb, 0.9))
                    .child(self.mail_compose_panel_title()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .text_size(px(20.0))
                    .line_height(px(20.0))
                    .font_weight(FontWeight::LIGHT)
                    .text_color(alpha(palette.text_rgb, 0.28))
                    .child("⌃")
                    .child("⌄"),
            )
    }

    fn render_mail_compose_inline_header(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .pb(px(12.0))
            .text_size(px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.72, 0.7),
            ))
            .child(self.mail_compose_inline_title())
    }

    fn render_mail_compose_divider(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div().h(px(1.0)).w_full().bg(alpha(
            palette.compose_panel_divider,
            if_light(self.appearance_mode, 1.0, 0.75),
        ))
    }

    fn mail_compose_panel_title(&self) -> &'static str {
        match self.mail_compose_mode {
            MailComposeMode::Closed => "Compose",
            MailComposeMode::New => {
                if self
                    .mail_active_message()
                    .is_some_and(|message| message.is_draft)
                {
                    "Edit Draft"
                } else {
                    "New Message"
                }
            }
            MailComposeMode::Reply => "Reply",
            MailComposeMode::ReplyAll => "Reply All",
            MailComposeMode::Forward => "Forward",
        }
    }

    fn mail_compose_inline_title(&self) -> String {
        let primary_to = mail_parse_addresses(&self.mail_compose_to)
            .first()
            .map(mail_address_summary);
        match primary_to {
            Some(recipient) if !recipient.is_empty() => format!("Draft to {recipient}"),
            _ => self.mail_compose_panel_title().to_string(),
        }
    }
}
