use super::{
    alpha, div, if_light, mail_palette, px, AnyElement, Context, Div, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailContactHistoryRow, MailContactHistoryStatus, MailPalette,
    MailThreadDetail, MouseButton, MouseDownEvent, ParentElement, Styled, SurfaceState,
};

impl SurfaceState {
    pub(crate) fn render_mail_contact_context(
        &self,
        thread: &MailThreadDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let history_status = self
            .mail_selected_contact_address
            .as_ref()
            .and_then(|contact| self.mail_contact_histories.get(contact))
            .map(|history| history.status.clone());
        let contact_rows = mail_contact_detail_rows(thread);
        div()
            .flex()
            .flex_col()
            .when_some(history_status, |this, status| {
                this.child(self.render_mail_contact_history_status(status, cx))
            })
            .when(!contact_rows.is_empty(), |this| {
                this.child(self.render_mail_contact_detail_rows(contact_rows, 18.0))
            })
    }

    fn render_mail_contact_history_status(
        &self,
        status: MailContactHistoryStatus,
        cx: &mut Context<Self>,
    ) -> Div {
        match status {
            MailContactHistoryStatus::Loading => self.render_mail_contact_history_skeleton(),
            MailContactHistoryStatus::Ready(rows) => {
                self.render_mail_contact_history(rows.as_ref(), cx)
            }
            MailContactHistoryStatus::Error(error) => {
                self.render_mail_contact_history_error(error.as_str(), cx)
            }
        }
    }

    fn render_mail_contact_history(
        &self,
        rows: &[MailContactHistoryRow],
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .px(px(20.0))
            .pt(px(12.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .children(
                rows.iter()
                    .cloned()
                    .map(|row| self.render_mail_contact_history_row(row, cx)),
            )
    }

    fn render_mail_contact_history_row(
        &self,
        row: MailContactHistoryRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let thread_id = row.thread_id.clone();
        let selected =
            row.selected || self.mail_open_thread_id.as_deref() == Some(row.thread_id.as_str());
        div()
            .id(format!("mail-contact-history-{}", row.thread_id))
            .rounded(px(4.0))
            .px(px(8.0))
            .py(px(7.0))
            .cursor_pointer()
            .bg(alpha(
                palette.text_rgb,
                if selected {
                    if_light(self.appearance_mode, 0.045, 0.08)
                } else {
                    0.0
                },
            ))
            .flex()
            .items_center()
            .gap(px(9.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    this.open_mail_thread(thread_id.clone(), cx);
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            )
            .child(self.render_mail_contact_history_icon(&row, &palette))
            .child(self.render_mail_contact_history_body(&row, &palette))
            .child(self.render_mail_contact_history_timestamp(&row, &palette))
            .into_any_element()
    }

    fn render_mail_contact_history_skeleton(&self) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let skeleton = alpha(palette.text_rgb, if_light(self.appearance_mode, 0.07, 0.11));
        div()
            .px(px(28.0))
            .pt(px(12.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .children((0..3).map(|index| {
                div()
                    .h(px(27.0))
                    .flex()
                    .items_center()
                    .gap(px(9.0))
                    .child(div().size(px(18.0)).rounded_full().bg(skeleton))
                    .child(
                        div()
                            .min_w(px(0.0))
                            .flex_grow(1.0)
                            .flex()
                            .flex_col()
                            .gap(px(5.0))
                            .child(
                                div()
                                    .w(px(104.0 + index as f32 * 13.0))
                                    .h(px(7.0))
                                    .rounded(px(2.0))
                                    .bg(skeleton),
                            )
                            .child(
                                div()
                                    .w(px(148.0 - index as f32 * 12.0))
                                    .h(px(6.0))
                                    .rounded(px(2.0))
                                    .bg(skeleton),
                            ),
                    )
            }))
    }

    fn render_mail_contact_history_error(&self, error: &str, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .px(px(28.0))
            .pt(px(12.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_ellipsis()
                    .text_size(px(10.5))
                    .text_color(alpha(palette.text_rgb, 0.38))
                    .child(mail_contact_history_error_label(error)),
            )
            .child(
                div()
                    .id("mail-contact-history-retry")
                    .flex_none()
                    .cursor_pointer()
                    .text_size(px(10.5))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(alpha(palette.text_rgb, 0.62))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.retry_mail_contact_history(cx);
                        }),
                    )
                    .child("Retry"),
            )
    }

    fn render_mail_contact_history_icon(
        &self,
        row: &MailContactHistoryRow,
        palette: &MailPalette,
    ) -> Div {
        div()
            .size(px(18.0))
            .flex_none()
            .rounded_full()
            .bg(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.08, 0.12),
            ))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(8.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.42, 0.36),
            ))
            .child(mail_contact_row_icon(row.subject.as_str()))
    }

    fn render_mail_contact_history_body(
        &self,
        row: &MailContactHistoryRow,
        palette: &MailPalette,
    ) -> Div {
        div()
            .min_w(px(0.0))
            .flex_grow(1.0)
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(self.render_mail_contact_history_text(&row.subject, 12.0, 0.62, palette))
            .child(self.render_mail_contact_history_text(&row.preview, 11.0, 0.4, palette))
    }

    fn render_mail_contact_history_text(
        &self,
        text: &str,
        size: f32,
        opacity: f32,
        palette: &MailPalette,
    ) -> Div {
        div()
            .min_w(px(0.0))
            .overflow_hidden()
            .text_size(px(size))
            .line_height(px(size + 3.0))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(palette.text_rgb, opacity))
            .text_ellipsis()
            .child(text.to_string())
    }

    fn render_mail_contact_history_timestamp(
        &self,
        row: &MailContactHistoryRow,
        palette: &MailPalette,
    ) -> Div {
        div()
            .flex_none()
            .text_size(px(10.5))
            .font_weight(FontWeight::NORMAL)
            .text_color(alpha(
                palette.text_rgb,
                if_light(self.appearance_mode, 0.36, 0.32),
            ))
            .child(row.timestamp.clone())
    }

    fn render_mail_contact_detail_rows(&self, rows: Vec<String>, top: f32) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .px(px(28.0))
            .pt(px(top))
            .flex()
            .flex_col()
            .gap(px(11.0))
            .children(rows.into_iter().map(|label| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(
                        div()
                            .size(px(18.0))
                            .rounded_full()
                            .bg(alpha(
                                palette.text_rgb,
                                if_light(self.appearance_mode, 0.08, 0.12),
                            ))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(8.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(alpha(
                                palette.text_rgb,
                                if_light(self.appearance_mode, 0.42, 0.36),
                            ))
                            .child(mail_contact_row_icon(label.as_str())),
                    )
                    .child(
                        div()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(alpha(palette.text_rgb, 0.58))
                            .text_ellipsis()
                            .child(label),
                    )
            }))
    }
}

fn mail_contact_history_error_label(error: &str) -> String {
    let error = error.trim();
    if error.is_empty() {
        return "History unavailable".to_string();
    }
    let concise = error.chars().take(56).collect::<String>();
    if error.chars().count() <= 56 {
        format!("History unavailable: {concise}")
    } else {
        format!("History unavailable: {concise}…")
    }
}

fn mail_contact_detail_rows(thread: &MailThreadDetail) -> Vec<String> {
    thread
        .sender_email
        .split_once('@')
        .map(|(_, domain)| vec![domain.to_string()])
        .unwrap_or_default()
}

fn mail_contact_row_icon(label: &str) -> String {
    if label.starts_with('@') {
        return "@".to_string();
    }
    label
        .chars()
        .find(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase().to_string())
        .unwrap_or_default()
}
