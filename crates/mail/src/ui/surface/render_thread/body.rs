use super::super::{
    div, list, mail_message_card_row_count, mail_message_document_row_count,
    mail_thread_shows_auto_responses, px, AnyElement, Arc, Context, IntoElement,
    ListSizingBehavior, MailThreadDetail, MailThreadMessageDetail, ParentElement, Styled,
    SurfaceState,
};
use super::card::MailMessageCardContentFrameState;
use crate::ui::MAIL_MESSAGE_CARD_MAX_WIDTH;

struct MailOpenThreadCardRow<'a> {
    message: &'a MailThreadMessageDetail,
    card_index: usize,
    card_row_count: usize,
    latest: bool,
}

enum MailOpenThreadBodyRow<'a> {
    CollapsedHistory(&'a MailThreadMessageDetail),
    MessageCard(MailOpenThreadCardRow<'a>),
}

impl SurfaceState {
    pub(crate) fn render_mail_open_thread_body(
        &self,
        thread: Arc<MailThreadDetail>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let list_state = self.mail_open_thread_body_list_state.clone();
        let list_view = cx.entity();
        div()
            .size_full()
            .child(
                list(list_state, move |index, _window, cx| {
                    let thread = thread.clone();
                    list_view.update(cx, |this, cx| {
                        this.render_mail_open_thread_body_row(&thread, index, cx)
                    })
                })
                .with_sizing_behavior(ListSizingBehavior::Auto)
                .size_full(),
            )
            .into_any_element()
    }

    pub(crate) fn render_mail_open_thread_body_row(
        &self,
        thread: &MailThreadDetail,
        index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let row_count = self.open_thread_body_row_count(thread);
        let Some(body_row) = self.mail_open_thread_body_row(thread, index) else {
            return div().into_any_element();
        };
        let content = div()
            .w_full()
            .max_w(px(MAIL_MESSAGE_CARD_MAX_WIDTH))
            .pt(px(if index == 0 { 28.0 } else { 0.0 }))
            .pb(px(self.mail_open_thread_body_row_bottom_padding(
                index, row_count, &body_row,
            )));
        let row = div().w_full().min_w(px(0.0)).flex().justify_center();
        match body_row {
            MailOpenThreadBodyRow::CollapsedHistory(message) => row
                .child(content.child(self.render_mail_open_thread_history_row(
                    thread.id.as_str(),
                    message,
                    cx,
                )))
                .into_any_element(),
            MailOpenThreadBodyRow::MessageCard(card_row) => row
                .child(
                    content.child(self.render_mail_open_message_card_virtual_row(
                        thread.id.as_str(),
                        card_row,
                        cx,
                    )),
                )
                .into_any_element(),
        }
    }

    fn mail_open_thread_body_row<'a>(
        &self,
        thread: &'a MailThreadDetail,
        mut index: usize,
    ) -> Option<MailOpenThreadBodyRow<'a>> {
        let message_count = thread.message_details.len();
        for (message_index, message) in thread.message_details.iter().enumerate() {
            let latest = message_index + 1 == message_count;
            let expanded = self
                .mail_expanded_history_message_ids
                .contains(message.id.as_str());
            if !latest && !expanded {
                if index == 0 {
                    return Some(MailOpenThreadBodyRow::CollapsedHistory(message));
                }
                index -= 1;
                continue;
            }
            let card_row_count =
                self.open_message_card_row_count(thread.id.as_str(), message, latest);
            if index < card_row_count {
                return Some(MailOpenThreadBodyRow::MessageCard(MailOpenThreadCardRow {
                    message,
                    card_index: index,
                    card_row_count,
                    latest,
                }));
            }
            index -= card_row_count;
        }
        None
    }

    fn mail_open_thread_body_row_bottom_padding(
        &self,
        index: usize,
        row_count: usize,
        row: &MailOpenThreadBodyRow<'_>,
    ) -> f32 {
        if index + 1 == row_count {
            return 32.0;
        }
        match row {
            MailOpenThreadBodyRow::CollapsedHistory(_) => 4.0,
            MailOpenThreadBodyRow::MessageCard(card_row)
                if card_row.card_index + 1 == card_row.card_row_count =>
            {
                12.0
            }
            MailOpenThreadBodyRow::MessageCard(_) => 0.0,
        }
    }

    fn render_mail_open_message_card_virtual_row(
        &self,
        thread_id: &str,
        card_row: MailOpenThreadCardRow<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let message = card_row.message;
        let base_card_row_count = mail_message_card_row_count(message);
        let first = card_row.card_index == 0;
        let last = card_row.card_index + 1 == card_row.card_row_count;
        let rich_layout = message.body.document.is_rich_layout;
        let content = self.render_open_message_card_content_frame(
            MailMessageCardContentFrameState {
                thread_id: thread_id.to_string(),
                message_id: message.id.clone(),
                card_index: card_row.card_index,
                rich_layout,
                first,
                bottom_padding: self.mail_message_card_row_bottom_padding(
                    message,
                    card_row.card_index,
                    last,
                ),
            },
            cx,
        );
        self.render_mail_open_message_card_row_shell(thread_id, first, last, cx)
            .child(content.child(
                if card_row.latest && card_row.card_index == base_card_row_count {
                    self.render_inline_reply_composer(cx)
                } else {
                    self.render_mail_open_message_card_row_content(
                        thread_id,
                        message,
                        card_row.card_index,
                        cx,
                    )
                },
            ))
            .into_any_element()
    }

    fn render_mail_open_message_card_row_content(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        card_index: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if card_index == 0 {
            return self
                .render_mail_open_message_card_meta(thread_id, message, cx)
                .into_any_element();
        }
        let mut index = card_index - 1;
        let document = &message.body.document;
        let document_row_count = mail_message_document_row_count(document);
        if index < document_row_count {
            let options = self.mail_open_message_render_options(document.is_rich_layout);
            if document.is_rich_layout {
                return crate::ui::render_mail_document(document, options);
            }
            if let Some(block) = document.blocks.get(index) {
                return crate::ui::render_mail_document_block(block, options);
            }
            if document.clipped && index == document.blocks.len() {
                return crate::ui::render_mail_document_clipped_marker(options);
            }
        }
        index -= document_row_count;
        if !message.attachments.is_empty() {
            if index == 0 {
                return self
                    .render_mail_open_message_card_attachments(thread_id, message, cx)
                    .into_any_element();
            }
            index -= 1;
        }
        if mail_thread_shows_auto_responses(message) {
            if index == 0 {
                return self
                    .render_mail_open_message_card_auto_responses()
                    .into_any_element();
            }
            index -= 1;
        }
        if !message.signature_lines.is_empty() && index == 0 {
            return self
                .render_mail_open_message_card_signature(message.signature_lines.clone())
                .into_any_element();
        }
        div().into_any_element()
    }

    fn mail_message_card_row_bottom_padding(
        &self,
        message: &MailThreadMessageDetail,
        card_index: usize,
        last: bool,
    ) -> f32 {
        if last {
            return 18.0;
        }
        if card_index == 0 {
            return 16.0;
        }
        if message.body.document.is_rich_layout {
            0.0
        } else {
            12.0
        }
    }
}
