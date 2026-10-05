use super::super::{
    alpha, div, if_light, mail_palette, px, Context, Div, FluentBuilder, FontWeight,
    InteractiveElement, MailThreadMessageDetail, MouseButton, MouseDownEvent, ParentElement,
    Styled, SurfaceState,
};
use crate::model::MailAttachment;
use crate::ui::format_attachment_size;
use crate::ui::surface::MailAttachmentMessageKey;

struct MailAttachmentRow<'a> {
    thread_id: &'a str,
    message_id: &'a str,
    received: bool,
    attachment: &'a MailAttachment,
    download_pending: bool,
}

impl SurfaceState {
    pub(crate) fn render_mail_open_message_card_attachments(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let download_pending = self.mail_attachment_download_pending();
        let downloadable = message.received
            && message
                .attachments
                .iter()
                .any(|attachment| attachment.blob_id.is_some());
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .when(downloadable, |this| {
                this.child(self.render_mail_attachment_download_all(
                    thread_id,
                    message,
                    download_pending,
                    cx,
                ))
            })
            .children(message.attachments.iter().map(|attachment| {
                self.render_mail_attachment_row(
                    MailAttachmentRow {
                        thread_id,
                        message_id: message.id.as_str(),
                        received: message.received,
                        attachment,
                        download_pending,
                    },
                    cx,
                )
            }))
    }

    fn render_mail_attachment_download_all(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        download_pending: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let download_all_message = MailAttachmentMessageKey {
            thread_id: thread_id.to_string(),
            message_id: message.id.clone(),
        };
        let download_all_attachments = message.attachments.clone();
        div().flex().justify_end().child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(alpha(
                    palette.text_rgb,
                    if download_pending { 0.34 } else { 0.68 },
                ))
                .when(!download_pending, |this| {
                    this.cursor_pointer().on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.prompt_download_all_received_mail_attachments(
                                download_all_message.clone(),
                                download_all_attachments.clone(),
                                window,
                                cx,
                            );
                        }),
                    )
                })
                .child("Download all"),
        )
    }

    fn render_mail_attachment_row(
        &self,
        row: MailAttachmentRow<'_>,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let row_message = MailAttachmentMessageKey {
            thread_id: row.thread_id.to_string(),
            message_id: row.message_id.to_string(),
        };
        let row_attachment = row.attachment.clone();
        let actionable = row.received && row.attachment.blob_id.is_some();
        div()
            .px(px(10.0))
            .py(px(8.0))
            .rounded(px(6.0))
            .bg(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.05, 0.08),
            ))
            .text_size(px(11.5))
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.72, 0.68),
            ))
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .when(actionable && !row.download_pending, |this| {
                this.cursor_pointer().on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.prompt_download_received_mail_attachment(
                            row_message.clone(),
                            row_attachment.clone(),
                            window,
                            cx,
                        );
                    }),
                )
            })
            .child(mail_attachment_name(row.attachment))
            .when(actionable, |this| {
                this.child(self.render_mail_attachment_download_label(row.download_pending))
            })
    }

    fn render_mail_attachment_download_label(&self, download_pending: bool) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_none()
            .font_weight(FontWeight::MEDIUM)
            .text_color(alpha(
                palette.text_rgb,
                if download_pending { 0.32 } else { 0.64 },
            ))
            .child("Download")
    }
}

fn mail_attachment_name(attachment: &MailAttachment) -> Div {
    div()
        .min_w(px(0.0))
        .overflow_hidden()
        .text_ellipsis()
        .child(format!(
            "{}  {}",
            attachment.name,
            format_attachment_size(attachment.size)
        ))
}
