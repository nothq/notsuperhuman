use std::collections::HashMap;

mod body;

use crate::model::{
    MailAddress, MailAttachment, MailDisplayBody, MailIdentity, MailMessage, MailThread, Mailbox,
};
use body::{extract_mail_body_text, extract_mail_display_body};
use serde::Deserialize;
use serde_json::Value;

use crate::live::types::{
    JmapAccountResponse, JmapEmail, JmapEmailAddress, JmapEmailAttachment, JmapIdentity,
    JmapMailbox, JmapResponse, JmapThread,
};

pub(crate) fn decode_jmap_response<T: for<'de> Deserialize<'de>>(
    response: &JmapResponse,
    call_id: &str,
    expected_method: &str,
) -> Result<T, String> {
    let (method, payload, _) = response
        .method_responses
        .iter()
        .find(|(_, _, id)| id == call_id)
        .ok_or_else(|| format!("missing JMAP response for call {call_id}"))?;
    if method == "error" {
        let kind = payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("error");
        let description = payload
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if description.is_empty() {
            return Err(format!("JMAP call {call_id} failed: {kind}"));
        }
        return Err(format!("JMAP call {call_id} failed: {kind}: {description}"));
    }
    if method != expected_method {
        return Err(format!(
            "unexpected JMAP method {method} for call {call_id}; expected {expected_method}"
        ));
    }
    serde_json::from_value(payload.clone())
        .map_err(|error| format!("invalid {expected_method} payload: {error}"))
}

pub(crate) fn decode_account_jmap_response<T>(
    response: &JmapResponse,
    call_id: &str,
    expected_method: &str,
    expected_account_id: &str,
) -> Result<T, String>
where
    T: for<'de> Deserialize<'de> + JmapAccountResponse,
{
    let payload = decode_jmap_response::<T>(response, call_id, expected_method)?;
    validate_jmap_account_response(expected_method, expected_account_id, payload.account_id())?;
    Ok(payload)
}

pub(crate) fn validate_jmap_account_response(
    method: &str,
    expected_account_id: &str,
    returned_account_id: &str,
) -> Result<(), String> {
    if returned_account_id != expected_account_id {
        return Err(format!(
            "{method} returned account {returned_account_id} for requested account {expected_account_id}"
        ));
    }
    Ok(())
}

pub(crate) fn find_initial_mailbox(mailboxes: &[JmapMailbox]) -> Result<&JmapMailbox, String> {
    mailboxes
        .iter()
        .find(|mailbox| mailbox.role.as_deref() == Some("inbox"))
        .or_else(|| {
            mailboxes
                .iter()
                .find(|mailbox| mailbox.name.eq_ignore_ascii_case("inbox"))
        })
        .or_else(|| mailboxes.first())
        .ok_or_else(|| "JMAP mailbox list was empty".to_string())
}

pub(crate) fn sort_jmap_mailboxes(mailboxes: &mut [JmapMailbox]) {
    mailboxes.sort_by(|left, right| {
        mailbox_sort_rank(left.role.as_deref())
            .cmp(&mailbox_sort_rank(right.role.as_deref()))
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn mailbox_sort_rank(role: Option<&str>) -> usize {
    match role.unwrap_or_default() {
        "inbox" => 0,
        "drafts" => 1,
        "sent" => 2,
        "archive" => 3,
        "snoozed" => 4,
        "junk" => 5,
        "trash" => 6,
        "all" => 7,
        _ => usize::MAX,
    }
}

pub(crate) fn into_mailbox(mailbox: JmapMailbox) -> Mailbox {
    Mailbox {
        id: mailbox.id,
        name: mailbox.name,
        role: mailbox.role,
        unread_emails: mailbox.unread_emails,
        total_emails: mailbox.total_emails,
        rights: crate::model::MailboxRights {
            may_read_items: mailbox.my_rights.may_read_items,
            may_add_items: mailbox.my_rights.may_add_items,
            may_remove_items: mailbox.my_rights.may_remove_items,
            may_set_seen: mailbox.my_rights.may_set_seen,
            may_set_keywords: mailbox.my_rights.may_set_keywords,
            may_create_child: mailbox.my_rights.may_create_child,
            may_rename: mailbox.my_rights.may_rename,
            may_delete: mailbox.my_rights.may_delete,
            may_submit: mailbox.my_rights.may_submit,
        },
    }
}

pub(crate) fn into_mail_message_summary(message: JmapEmail) -> MailMessage {
    into_mail_message_base(message, false)
}

pub(crate) fn into_mail_message(message: JmapEmail) -> MailMessage {
    into_mail_message_base(message, true)
}

fn into_mail_message_base(message: JmapEmail, body_loaded: bool) -> MailMessage {
    let display_body = if body_loaded {
        extract_mail_display_body(&message)
    } else {
        MailDisplayBody::default()
    };
    let body_text = if body_loaded {
        extract_mail_body_text(&message, &display_body)
    } else {
        String::new()
    };
    let body_html = match &display_body {
        MailDisplayBody::Html { html, .. } if body_loaded => Some(html.clone()),
        MailDisplayBody::PlainText { .. } | MailDisplayBody::Html { .. } => None,
    };
    MailMessage {
        id: message.id,
        thread_id: message.thread_id,
        message_id: message.message_id,
        in_reply_to: message.in_reply_to,
        references: message.references,
        subject: cleaned_subject(message.subject.as_deref()),
        preview: cleaned_preview(message.preview.as_deref()),
        received_at: message.received_at,
        sent_at: message.sent_at,
        sender: message.sender.into_iter().map(into_mail_address).collect(),
        reply_to: message
            .reply_to
            .into_iter()
            .map(into_mail_address)
            .collect(),
        from: message.from.into_iter().map(into_mail_address).collect(),
        to: message.to.into_iter().map(into_mail_address).collect(),
        cc: message.cc.into_iter().map(into_mail_address).collect(),
        bcc: message.bcc.into_iter().map(into_mail_address).collect(),
        mailbox_ids: message
            .mailbox_ids
            .into_iter()
            .filter_map(|(mailbox_id, included)| included.then_some(mailbox_id))
            .collect(),
        has_attachment: message.has_attachment,
        is_unread: !message.keywords.get("$seen").copied().unwrap_or(false),
        is_starred: message.keywords.get("$flagged").copied().unwrap_or(false),
        is_draft: message.keywords.get("$draft").copied().unwrap_or(false),
        body_text,
        body_html,
        display_body,
        body_loaded,
        attachments: message
            .attachments
            .into_iter()
            .map(into_mail_attachment)
            .collect(),
    }
}

fn into_mail_attachment(attachment: JmapEmailAttachment) -> MailAttachment {
    MailAttachment {
        blob_id: attachment
            .blob_id
            .and_then(|blob_id| crate::model::MailBlobId::parse(blob_id).ok()),
        name: attachment
            .name
            .unwrap_or_else(|| attachment.content_type.clone()),
        content_type: attachment.content_type,
        size: attachment.size,
        disposition: attachment.disposition,
    }
}

pub fn sanitize_cached_mail_summary_messages(messages: Vec<MailMessage>) -> Vec<MailMessage> {
    messages
        .into_iter()
        .map(|mut message| {
            message.body_text.clear();
            message.body_html = None;
            message.display_body = MailDisplayBody::default();
            message.body_loaded = false;
            message.attachments.clear();
            message
        })
        .collect()
}

pub(crate) fn into_mail_thread(thread: JmapThread, mut messages: Vec<MailMessage>) -> MailThread {
    let order = thread
        .email_ids
        .iter()
        .enumerate()
        .map(|(index, email_id)| (email_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    messages.sort_by_key(|message| {
        order
            .get(message.id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    MailThread {
        id: thread.id,
        messages,
    }
}

pub(crate) fn into_mail_identity(identity: JmapIdentity) -> MailIdentity {
    MailIdentity {
        id: identity.id,
        name: identity.name,
        email: identity.email,
        reply_to: identity
            .reply_to
            .into_iter()
            .map(into_mail_address)
            .collect(),
        bcc: identity.bcc.into_iter().map(into_mail_address).collect(),
        text_signature: identity.text_signature,
        html_signature: identity.html_signature,
    }
}

fn into_mail_address(address: JmapEmailAddress) -> MailAddress {
    MailAddress {
        name: address.name.unwrap_or_default(),
        email: address.email,
    }
}

fn cleaned_subject(subject: Option<&str>) -> String {
    let trimmed = subject.unwrap_or_default().trim();
    if trimmed.is_empty() {
        "(no subject)".to_string()
    } else {
        trimmed.to_string()
    }
}

fn cleaned_preview(preview: Option<&str>) -> String {
    preview
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests;
