use super::{
    mail_body_display, mail_correspondent_address, MailBodyDisplay, MailMessage, MailThreadDetail,
    MailThreadMessageDetail, MailWorkspace, MAIL_DRAFT_TAG_FILL,
};
use crate::model::{MailAddress, MailThread};
use crate::ui::{
    avatar_initials, format_mail_received_at, mail_address_display, mail_local_now,
    mail_local_timestamp, mailbox_display_label, MailDocument,
};
use time::macros::format_description;

mod header;

use header::{mail_message_header_details, mail_message_is_received};

pub(crate) fn mail_thread_detail(
    workspace: &MailWorkspace,
    thread: &MailThread,
) -> MailThreadDetail {
    let latest = mail_latest_thread_message(thread)
        .cloned()
        .unwrap_or_else(MailMessage::default);
    let (sender, sender_email) = mail_correspondent_address(workspace, &latest)
        .map(|(address, contact)| (mail_address_display(Some(address)), contact.to_string()))
        .unwrap_or_else(|| {
            (
                mail_primary_sender_label(&latest),
                mail_primary_sender_email(&latest),
            )
        });
    let subject = mail_subject_label(latest.subject.as_str());
    let preview = mail_preview_label(&latest);
    let recipient_label = mail_recipient_label(&latest);
    let display_timestamp = mail_display_timestamp(&latest);
    let body = mail_body_display(&latest);
    let signature_lines = mail_signature_lines(&latest, &body);
    let attachments = latest.attachments.clone();
    let mailbox_labels = mail_mailbox_labels(workspace, &latest);
    let participant_labels = mail_participant_labels(thread);
    let to_labels = latest.to.iter().map(mail_address_summary).collect();
    let cc_labels = latest.cc.iter().map(mail_address_summary).collect();
    let message_details = mail_thread_message_details(workspace, thread, &latest);
    MailThreadDetail {
        id: thread.id.clone(),
        sender,
        sender_email,
        subject: subject.clone(),
        preview: preview.clone(),
        subtitle: preview,
        recipient_label,
        display_timestamp,
        tag_label: latest.is_draft.then(|| "Draft".to_string()),
        tag_fill: MAIL_DRAFT_TAG_FILL,
        unread: thread.messages.iter().any(|message| message.is_unread),
        starred: thread.messages.iter().any(|message| message.is_starred),
        message_details,
        body,
        signature_lines,
        attachments,
        mailbox_labels,
        participant_labels,
        to_labels,
        cc_labels,
    }
}

fn mail_thread_message_details(
    workspace: &MailWorkspace,
    thread: &MailThread,
    fallback: &MailMessage,
) -> Vec<MailThreadMessageDetail> {
    if thread.messages.is_empty() {
        return vec![mail_thread_message_detail(workspace, fallback)];
    }
    thread
        .messages
        .iter()
        .map(|message| mail_thread_message_detail(workspace, message))
        .collect()
}

fn mail_thread_message_detail(
    workspace: &MailWorkspace,
    message: &MailMessage,
) -> MailThreadMessageDetail {
    let body = mail_body_display(message);
    MailThreadMessageDetail {
        id: message.id.clone(),
        received: mail_message_is_received(workspace, message),
        sender: mail_primary_sender_label(message),
        sender_email: mail_primary_sender_email(message),
        header: mail_message_header_details(workspace, message),
        subject: mail_subject_label(message.subject.as_str()),
        preview: mail_preview_label(message),
        display_timestamp: mail_display_timestamp(message),
        signature_lines: mail_signature_lines(message, &body),
        attachments: message.attachments.clone(),
        body,
    }
}

pub(crate) fn mail_thread_detail_from_summary(
    workspace: &MailWorkspace,
    summary: &MailMessage,
) -> MailThreadDetail {
    mail_thread_detail(
        workspace,
        &MailThread {
            id: summary.thread_id.clone(),
            messages: vec![summary.clone()],
        },
    )
}

pub(crate) fn mail_search_thread_detail_from_summary(
    workspace: &MailWorkspace,
    summary: &MailMessage,
) -> MailThreadDetail {
    let mut detail = mail_thread_detail_from_summary(workspace, summary);
    let Some((address, contact)) = mail_correspondent_address(workspace, summary) else {
        return detail;
    };
    detail.sender = mail_address_display(Some(address));
    detail.sender_email = contact.to_string();
    detail
}

pub(crate) fn mail_message_card_row_count(message: &MailThreadMessageDetail) -> usize {
    1 + mail_message_document_row_count(&message.body.document)
        + usize::from(!message.attachments.is_empty())
        + usize::from(mail_thread_shows_auto_responses(message))
        + usize::from(!message.signature_lines.is_empty())
}

pub(crate) fn mail_message_document_row_count(document: &MailDocument) -> usize {
    if document.is_rich_layout {
        usize::from(!document.blocks.is_empty() || document.clipped)
    } else {
        document.blocks.len() + usize::from(document.clipped)
    }
}

pub(crate) fn mail_thread_shows_auto_responses(message: &MailThreadMessageDetail) -> bool {
    message.subject.contains("Gift a month")
        || message.sender_email.contains("superhuman.com")
        || message.preview.contains("email superpowers")
}

pub(crate) fn mail_primary_sender_label(message: &MailMessage) -> String {
    mail_address_display(
        message
            .sender
            .first()
            .or_else(|| message.from.first())
            .or_else(|| message.reply_to.first()),
    )
}

pub(crate) fn mail_primary_sender_email(message: &MailMessage) -> String {
    message
        .sender
        .first()
        .or_else(|| message.from.first())
        .or_else(|| message.reply_to.first())
        .map(|address| address.email.clone())
        .unwrap_or_default()
}

pub(crate) fn mail_subject_label(subject: &str) -> String {
    let subject = subject.trim();
    if subject.is_empty() {
        "(no subject)".to_string()
    } else {
        subject.to_string()
    }
}

pub(crate) fn mail_preview_label(message: &MailMessage) -> String {
    let preview = message.preview.trim();
    if !preview.is_empty() {
        return preview.to_string();
    }
    let body = message.body_text.trim();
    if body.is_empty() {
        "No message content".to_string()
    } else {
        body.lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("No message content")
            .to_string()
    }
}

pub(crate) fn mail_sender_initials(sender: &str) -> String {
    avatar_initials(sender)
}

pub(crate) fn mail_recipient_label(message: &MailMessage) -> String {
    let to = message
        .to
        .iter()
        .map(mail_address_summary)
        .collect::<Vec<_>>();
    let cc = message
        .cc
        .iter()
        .map(mail_address_summary)
        .collect::<Vec<_>>();
    if !to.is_empty() && cc.is_empty() {
        return format!(
            "{} to {}",
            mail_primary_sender_label(message),
            to.join(", ")
        );
    }
    if !to.is_empty() || !cc.is_empty() {
        let recipients = to.into_iter().chain(cc).collect::<Vec<_>>().join(", ");
        return format!("{} to {}", mail_primary_sender_label(message), recipients);
    }
    format!("{} to Me", mail_primary_sender_label(message))
}

pub(crate) fn mail_display_timestamp(message: &MailMessage) -> String {
    let Some(timestamp) = mail_local_timestamp(&message.received_at) else {
        return format_mail_received_at(&message.received_at);
    };
    let now = mail_local_now();
    if timestamp.date() == now.date() {
        return timestamp
            .format(&format_description!(
                "[hour repr:12]:[minute] [period case:upper]"
            ))
            .unwrap_or_else(|_| format_mail_received_at(&message.received_at));
    }
    timestamp
        .format(&format_description!("[month repr:short] [day]"))
        .unwrap_or_else(|_| format_mail_received_at(&message.received_at))
}

pub(crate) fn mail_signature_lines(_message: &MailMessage, _body: &MailBodyDisplay) -> Vec<String> {
    Vec::new()
}

pub(crate) fn mail_mailbox_labels(workspace: &MailWorkspace, message: &MailMessage) -> Vec<String> {
    workspace
        .mailboxes
        .iter()
        .filter(|mailbox| message.mailbox_ids.iter().any(|id| id == &mailbox.id))
        .map(mailbox_display_label)
        .collect()
}

pub(crate) fn mail_mailbox_id_for_role(workspace: &MailWorkspace, role: &str) -> Option<String> {
    workspace
        .mailboxes
        .iter()
        .find(|mailbox| mailbox.role.as_deref() == Some(role))
        .map(|mailbox| mailbox.id.clone())
}

pub(crate) fn mail_participant_labels(thread: &MailThread) -> Vec<String> {
    let mut labels = Vec::new();
    for message in &thread.messages {
        for label in message
            .from
            .iter()
            .chain(message.to.iter())
            .chain(message.cc.iter())
            .map(mail_address_summary)
        {
            if !label.is_empty() && labels.iter().all(|existing| existing != &label) {
                labels.push(label);
            }
        }
    }
    labels
}

pub(crate) fn mail_latest_thread_message(thread: &MailThread) -> Option<&MailMessage> {
    thread.messages.last().or_else(|| thread.messages.first())
}

pub(crate) fn mail_address_summary(address: &MailAddress) -> String {
    if address.name.trim().is_empty() {
        address.email.clone()
    } else {
        address.name.clone()
    }
}
