use super::{
    alpha, div, font, mail_palette, mail_thread_row_background, px, rgb, AnyElement, Context, Div,
    FluentBuilder, FontWeight, InteractiveElement, IntoElement, MailListThread, MailRowAction,
    MailTriageControl, MailTriageFlags, MouseButton, MouseDownEvent, ParentElement, Styled,
    SurfaceState, MAIL_ROW_HEIGHT,
};
use crate::ui::surface::MAIL_FONT_FAMILY;

impl SurfaceState {
    pub(super) fn render_mail_thread_row_ordinary(
        &self,
        thread: &MailListThread,
        selected: bool,
        hovered: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let thread_id_for_click = thread.id.clone();
        let thread_id_for_hover = thread.id.clone();
        let mut row = div()
            .id(format!("mail-thread-{}", thread.id))
            .debug_selector({
                let thread_id = thread.id.clone();
                move || format!("mail-thread-{thread_id}")
            })
            .w_full()
            .h(px(MAIL_ROW_HEIGHT))
            .rounded(px(2.0))
            .relative()
            .flex()
            .items_center()
            .pr(px(24.0))
            .cursor_pointer()
            .bg(mail_thread_row_background(
                self.appearance_mode,
                selected,
                hovered,
            ))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_mail_thread(thread_id_for_click.clone(), cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .child(self.render_mail_thread_row_unread(thread.unread))
            .child(self.render_mail_thread_row_sender(thread))
            .child(self.render_mail_thread_row_summary(thread))
            .child(self.render_mail_thread_meta(thread, selected, cx));
        if selected {
            row = row.child(self.render_mail_thread_row_selected_accent());
        }
        row.interactivity()
            .on_hover(cx.listener(move |this, is_hovered, _, cx| {
                this.set_mail_thread_hover(thread_id_for_hover.clone(), *is_hovered, cx);
            }));
        row.into_any_element()
    }

    pub(crate) fn render_mail_thread_subject(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .min_w(px(0.0))
            .child(
                div()
                    .max_w(px(380.0))
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .font(font(MAIL_FONT_FAMILY))
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(palette.text_rgb, 0.9))
                    .text_ellipsis()
                    .child(self.render_mail_search_highlighted_text(&thread.subject)),
            )
            .when_some(thread.tag_label.as_ref(), |this, tag_label| {
                this.child(
                    div()
                        .px(px(6.0))
                        .py(px(2.0))
                        .rounded(px(2.0))
                        .bg(rgb(thread.tag_fill))
                        .text_size(px(7.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgb(0x251a2e))
                        .child(tag_label.clone()),
                )
            })
    }

    pub(crate) fn render_mail_thread_preview(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .overflow_hidden()
            .whitespace_nowrap()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(palette.text_rgb, 0.5))
            .text_ellipsis()
            .child(self.render_mail_search_highlighted_text(&thread.preview))
    }

    pub(crate) fn render_mail_thread_meta(
        &self,
        thread: &MailListThread,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> Div {
        let triage = MailTriageFlags {
            unread: thread.unread,
            starred: thread.starred,
        };
        let meta_width = if selected { 134.0 } else { 146.0 };
        div()
            .w(px(meta_width))
            .min_w(px(meta_width))
            .max_w(px(meta_width))
            .flex_none()
            .flex()
            .items_center()
            .justify_end()
            .gap(px(3.0))
            .when(!selected, |this| {
                this.child(self.render_mail_thread_date_label(thread))
            })
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
    }

    fn render_mail_thread_date_label(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(palette.text_rgb, 0.5))
            .child(thread.date_label.clone())
    }

    pub(crate) fn render_mail_thread_row_unread(&self, unread: bool) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div().w(px(52.5)).h(px(16.0)).flex_none().relative().child(
            div()
                .absolute()
                .left(px(36.5))
                .top(px(4.5))
                .size(px(if unread { 7.0 } else { 0.0 }))
                .rounded_full()
                .bg(rgb(palette.unread_dot)),
        )
    }

    pub(crate) fn render_mail_thread_row_sender(&self, thread: &MailListThread) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w(px(195.21875))
            .min_w(px(195.21875))
            .max_w(px(195.21875))
            .pr(px(30.0))
            .flex_none()
            .overflow_hidden()
            .font(font(MAIL_FONT_FAMILY))
            .text_size(px(12.0))
            .line_height(px(16.0))
            .font_weight(FontWeight::BOLD)
            .text_color(alpha(palette.text_rgb, 0.85))
            .text_ellipsis()
            .child(self.render_mail_search_highlighted_text(&thread.sender))
    }

    pub(crate) fn render_mail_thread_row_summary(&self, thread: &MailListThread) -> Div {
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .pr(px(0.0))
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(15.0))
                    .min_w(px(0.0))
                    .child(self.render_mail_thread_subject(thread))
                    .child(self.render_mail_thread_preview(thread)),
            )
    }

    pub(crate) fn render_mail_thread_row_selected_accent(&self) -> Div {
        div()
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .w(px(3.0))
            .h_full()
            .bg(rgb(mail_palette(self.appearance_mode).selected_row_accent))
    }
}
