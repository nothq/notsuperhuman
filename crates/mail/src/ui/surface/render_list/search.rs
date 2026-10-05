use super::{
    alpha, div, font, mail_palette, px, rgb, AnyElement, AppearanceMode, Context, Div,
    FluentBuilder, FontWeight, InteractiveElement, IntoElement, MailListAttachment, MailListThread,
    MailRowAction, MailTriageControl, MailTriageFlags, MouseButton, MouseDownEvent, ParentElement,
    Styled, SurfaceState, MAIL_SEARCH_FONT_FAMILY,
};

const MAIL_SEARCH_LEADING_WIDTH: f32 = 52.5;
const MAIL_SEARCH_SENDER_WIDTH: f32 = 193.867;
const MAIL_SEARCH_META_WIDTH: f32 = 140.0;

impl SurfaceState {
    pub(super) fn render_mail_search_thread_row(
        &self,
        thread: &MailListThread,
        selected: bool,
        hovered: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let has_attachment_line = !thread.attachments.is_empty();
        let thread_id_for_click = thread.id.clone();
        let thread_id_for_hover = thread.id.clone();
        let mut row = div()
            .id(format!("mail-thread-{}", thread.id))
            .debug_selector({
                let thread_id = thread.id.clone();
                move || format!("mail-thread-{thread_id}")
            })
            .w_full()
            .h(px(if has_attachment_line { 68.0 } else { 36.0 }))
            .rounded(px(0.0))
            .relative()
            .flex()
            .cursor_pointer()
            .bg(self.mail_search_thread_row_background(selected, hovered))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_mail_thread(thread_id_for_click.clone(), cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .child(self.render_mail_search_thread_primary(thread, selected, cx))
            .when(has_attachment_line, |this| {
                this.child(self.render_mail_search_thread_attachments(thread))
            })
            .when(selected, |this| {
                this.child(self.render_mail_search_thread_selected_accent())
            });
        row.interactivity()
            .on_hover(cx.listener(move |this, is_hovered, _, cx| {
                this.set_mail_thread_hover(thread_id_for_hover.clone(), *is_hovered, cx);
            }));
        row.into_any_element()
    }

    fn render_mail_search_thread_primary(
        &self,
        thread: &MailListThread,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .w_full()
            .h(px(36.0))
            .flex_none()
            .flex()
            .items_center()
            .child(self.render_mail_search_thread_unread(thread.unread))
            .child(self.render_mail_search_thread_sender(thread))
            .child(self.render_mail_search_thread_summary(thread))
            .child(self.render_mail_search_thread_meta(thread, selected, cx))
    }

    fn render_mail_search_thread_subject(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .min_w(px(0.0))
            .when_some(thread.tag_label.as_ref(), |this, tag_label| {
                this.child(
                    div()
                        .h(px(16.0))
                        .px(px(4.0))
                        .pt(px(3.0))
                        .pb(px(1.0))
                        .rounded(px(2.0))
                        .bg(rgb(0xbbc5ef))
                        .font(font(MAIL_SEARCH_FONT_FAMILY))
                        .text_size(px(10.0))
                        .line_height(px(12.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0x27292d))
                        .child(tag_label.clone()),
                )
            })
            .child(
                div()
                    .max_w(px(380.0))
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .font(font(MAIL_SEARCH_FONT_FAMILY))
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(palette.text_rgb, 0.9))
                    .text_ellipsis()
                    .child(self.render_mail_search_highlighted_text(&thread.subject)),
            )
    }

    fn render_mail_search_thread_preview(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .font(font(MAIL_SEARCH_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(palette.text_rgb, 0.5))
            .text_ellipsis()
            .child(self.render_mail_search_highlighted_text(&thread.preview))
    }

    fn render_mail_search_thread_meta(
        &self,
        thread: &MailListThread,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let triage = MailTriageFlags {
            unread: thread.unread,
            starred: thread.starred,
        };
        div()
            .w(px(MAIL_SEARCH_META_WIDTH))
            .min_w(px(MAIL_SEARCH_META_WIDTH))
            .max_w(px(MAIL_SEARCH_META_WIDTH))
            .pr(px(10.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(3.0))
            .child(self.render_mail_triage_button(
                thread.id.as_str(),
                MailTriageControl::Star,
                triage,
                cx,
            ))
            .child(self.render_mail_triage_button(
                thread.id.as_str(),
                MailTriageControl::Read,
                triage,
                cx,
            ))
            .when(selected, |this| {
                this.child(self.render_mail_thread_action_button(
                    thread.id.as_str(),
                    MailRowAction::MarkDone,
                    cx,
                ))
                .child(self.render_mail_thread_action_button(
                    thread.id.as_str(),
                    MailRowAction::RemindMe,
                    cx,
                ))
                .child(self.render_mail_thread_action_button(
                    thread.id.as_str(),
                    MailRowAction::Move,
                    cx,
                ))
            })
            .when(!selected && thread.has_attachment, |this| {
                this.child(self.render_mail_thread_attachment_icon(cx))
            })
            .when(!selected, |this| {
                this.child(self.render_mail_thread_action_button(
                    thread.id.as_str(),
                    MailRowAction::MarkDone,
                    cx,
                ))
                .child(self.render_mail_search_thread_date_label(thread))
            })
    }

    fn render_mail_search_thread_date_label(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .font(font(MAIL_SEARCH_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(palette.text_rgb, 0.5))
            .child(thread.date_label.clone())
    }

    fn render_mail_search_thread_unread(&self, unread: bool) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w(px(MAIL_SEARCH_LEADING_WIDTH))
            .flex_none()
            .flex()
            .justify_start()
            .pl(px(9.0))
            .child(
                div()
                    .size(px(if unread { 7.0 } else { 0.0 }))
                    .rounded_full()
                    .bg(rgb(palette.unread_dot)),
            )
    }

    fn render_mail_search_thread_sender(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w(px(MAIL_SEARCH_SENDER_WIDTH))
            .min_w(px(MAIL_SEARCH_SENDER_WIDTH))
            .max_w(px(MAIL_SEARCH_SENDER_WIDTH))
            .flex_none()
            .overflow_hidden()
            .font(font(MAIL_SEARCH_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(alpha(palette.text_rgb, 0.85))
            .text_ellipsis()
            .child(self.render_mail_search_highlighted_text(&thread.sender))
    }

    fn render_mail_search_thread_summary(&self, thread: &MailListThread) -> Div {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .pr(px(8.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .min_w(px(0.0))
                    .child(self.render_mail_search_thread_subject(thread))
                    .child(self.render_mail_search_thread_preview(thread)),
            )
    }

    fn render_mail_search_thread_attachments(&self, thread: &MailListThread) -> Div {
        div()
            .absolute()
            .left(px(MAIL_SEARCH_LEADING_WIDTH + MAIL_SEARCH_SENDER_WIDTH))
            .right(px(MAIL_SEARCH_META_WIDTH))
            .top(px(27.0))
            .h(px(41.0))
            .pt(px(2.0))
            .pb(px(4.0))
            .overflow_hidden()
            .flex()
            .items_start()
            .gap(px(6.0))
            .children(
                thread
                    .attachments
                    .iter()
                    .map(render_mail_search_attachment_card),
            )
    }

    fn render_mail_search_thread_selected_accent(&self) -> Div {
        div()
            .absolute()
            .left(px(0.0))
            .top(px(4.0))
            .w(px(3.0))
            .h(px(28.0))
            .bg(rgb(0xb3b5df))
    }

    fn mail_search_thread_row_background(&self, selected: bool, hovered: bool) -> gpui::Hsla {
        if selected && self.appearance_mode == AppearanceMode::Dark {
            alpha(0x474b52, 1.0)
        } else {
            super::mail_thread_row_background(self.appearance_mode, selected, hovered)
        }
    }
}

fn render_mail_search_attachment_card(attachment: &MailListAttachment) -> Div {
    div()
        .h(px(28.0))
        .min_h(px(28.0))
        .max_h(px(28.0))
        .max_w(px(230.0))
        .px(px(6.0))
        .rounded(px(4.0))
        .bg(rgb(0x555b64))
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(
            div()
                .flex_none()
                .font(font(MAIL_SEARCH_FONT_FAMILY))
                .text_size(px(12.0))
                .line_height(px(12.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(attachment.type_fill))
                .child(attachment.type_label.clone()),
        )
        .child(
            div()
                .min_w(px(0.0))
                .overflow_hidden()
                .whitespace_nowrap()
                .font(font(MAIL_SEARCH_FONT_FAMILY))
                .text_size(px(12.0))
                .line_height(px(16.0))
                .font_weight(FontWeight::NORMAL)
                .text_color(alpha(0xffffff, 0.7))
                .text_ellipsis()
                .child(attachment.display_name.clone()),
        )
}
