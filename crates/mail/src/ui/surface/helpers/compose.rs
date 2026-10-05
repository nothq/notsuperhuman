use super::{
    mail_display_timestamp, mail_latest_thread_message, mail_preview_label,
    mail_primary_sender_label, mail_subject_label,
};
use crate::model::{MailAddress, MailAttachment, MailThread};
use crate::ui::format_mail_address_list;

pub(crate) fn mail_parse_addresses(value: &str) -> Vec<MailAddress> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| match part.rsplit_once('<') {
            Some((name, email)) if part.ends_with('>') => MailAddress {
                name: name.trim().trim_matches('"').to_string(),
                email: email.trim_end_matches('>').trim().to_string(),
            },
            _ => MailAddress {
                name: String::new(),
                email: part.to_string(),
            },
        })
        .collect()
}

pub(crate) fn mail_format_addresses(addresses: &[MailAddress]) -> String {
    format_mail_address_list(addresses)
}

pub(crate) fn mail_split_compose_addresses(value: &str) -> (Vec<MailAddress>, String) {
    if value.trim().is_empty() {
        return (Vec::new(), String::new());
    }
    if value.trim_end().ends_with(',') {
        return (mail_parse_addresses(value), String::new());
    }
    let Some((complete, active)) = value.rsplit_once(',') else {
        return (Vec::new(), value.trim().to_string());
    };
    (mail_parse_addresses(complete), active.trim().to_string())
}

pub(crate) fn mail_replace_active_compose_address(value: &mut String, address: &MailAddress) {
    let replacement = mail_format_addresses(std::slice::from_ref(address));
    if let Some(index) = value.rfind(',') {
        let prefix = value[..=index].trim_end();
        *value = if prefix.is_empty() {
            format!("{replacement}, ")
        } else {
            format!("{prefix} {replacement}, ")
        };
        return;
    }
    *value = format!("{replacement}, ");
}

pub(crate) fn mail_reply_subject(subject: &str) -> String {
    let subject = mail_subject_label(subject);
    if subject.to_ascii_lowercase().starts_with("re:") {
        subject
    } else {
        format!("Re: {subject}")
    }
}

pub(crate) fn mail_forward_subject(subject: &str) -> String {
    let subject = mail_subject_label(subject);
    if subject.to_ascii_lowercase().starts_with("fwd:") {
        subject
    } else {
        format!("Fwd: {subject}")
    }
}

pub(crate) fn mail_thread_quote(thread: &MailThread) -> String {
    let Some(message) = mail_latest_thread_message(thread) else {
        return String::new();
    };
    let body = if message.body_text.trim().is_empty() {
        mail_preview_label(message)
    } else {
        message.body_text.trim().to_string()
    };
    let sender = mail_primary_sender_label(message);
    let timestamp = mail_display_timestamp(message);
    format!(
        "\n\nOn {timestamp}, {sender} wrote:\n{}",
        body.lines()
            .map(|line| format!("> {line}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

pub(crate) fn mail_thread_references(thread: &MailThread) -> Vec<String> {
    let mut references = Vec::new();
    for message in &thread.messages {
        for reference in message
            .references
            .iter()
            .chain(message.message_id.iter())
            .chain(message.in_reply_to.iter())
        {
            if references.iter().all(|existing| existing != reference) {
                references.push(reference.clone());
            }
        }
    }
    references
}

pub(crate) fn mail_reply_recipients(
    thread: &MailThread,
    self_email: &str,
    reply_all: bool,
) -> (Vec<MailAddress>, Vec<MailAddress>) {
    let Some(message) = mail_latest_thread_message(thread) else {
        return (Vec::new(), Vec::new());
    };
    let mut to = message
        .reply_to
        .iter()
        .cloned()
        .chain(message.from.iter().cloned())
        .filter(|address| !mail_matches_self(address, self_email))
        .collect::<Vec<_>>();
    mail_dedup_addresses(&mut to);
    if to.is_empty() {
        to = message
            .to
            .iter()
            .filter(|&address| !mail_matches_self(address, self_email))
            .cloned()
            .collect::<Vec<_>>();
        mail_dedup_addresses(&mut to);
    }
    if !reply_all {
        return (to, Vec::new());
    }
    let mut cc = message
        .to
        .iter()
        .cloned()
        .chain(message.cc.iter().cloned())
        .filter(|address| !mail_matches_self(address, self_email))
        .filter(|address| {
            to.iter()
                .all(|existing| !mail_same_address(existing, address))
        })
        .collect::<Vec<_>>();
    mail_dedup_addresses(&mut cc);
    (to, cc)
}

pub(crate) fn mail_forward_attachments(thread: &MailThread) -> Vec<MailAttachment> {
    mail_latest_thread_message(thread)
        .map(|message| message.attachments.clone())
        .unwrap_or_default()
}

fn mail_matches_self(address: &MailAddress, self_email: &str) -> bool {
    !self_email.is_empty() && address.email.eq_ignore_ascii_case(self_email)
}

fn mail_same_address(left: &MailAddress, right: &MailAddress) -> bool {
    left.email.eq_ignore_ascii_case(&right.email)
}

fn mail_dedup_addresses(addresses: &mut Vec<MailAddress>) {
    let mut deduped = Vec::new();
    for address in addresses.drain(..) {
        if deduped
            .iter()
            .all(|existing| !mail_same_address(existing, &address))
        {
            deduped.push(address);
        }
    }
    *addresses = deduped;
}
