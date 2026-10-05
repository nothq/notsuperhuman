use super::{div, px, rgb, Div, FontWeight, MailThreadDetail, ParentElement, Styled};

pub(super) fn mail_subject_header_parts(subject: &str) -> (String, Option<String>) {
    let subject = subject.trim();
    subject
        .strip_suffix("🎁")
        .map(|title| (title.trim_end().to_string(), Some("🎁".to_string())))
        .unwrap_or_else(|| (subject.to_string(), None))
}

pub(super) fn mail_thread_shows_unsubscribe(thread: &MailThreadDetail) -> bool {
    thread.sender_email.contains("superhuman.com")
        || thread.subject.contains("Gift a month")
        || thread.preview.contains("Superhuman")
}

pub(super) fn mail_thread_tag(label: &str, fill: u32) -> Div {
    div()
        .px(px(6.0))
        .py(px(2.0))
        .rounded(px(2.0))
        .bg(rgb(fill))
        .text_size(px(7.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(0x251a2e))
        .child(label.to_string())
}
