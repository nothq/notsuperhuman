//! Maps Gmail labels and messages onto the records the rest of the app reads.
//! Messages become the same JMAP-shaped `JmapEmail` the JMAP backend decodes,
//! so subjects, previews, plain and rich bodies all go through one pipeline.

use std::collections::HashMap;

use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use time::{
    format_description::well_known::{Rfc2822, Rfc3339},
    OffsetDateTime,
};

use super::api::{GmailLabel, GmailMessage, GmailPart};
use crate::live::types::{
    JmapEmail, JmapEmailAddress, JmapEmailAttachment, JmapEmailBodyPart, JmapEmailBodyValue,
};
use crate::model::{Mailbox, MailboxRights};

pub(crate) const INBOX: &str = "INBOX";
pub(crate) const DRAFT: &str = "DRAFT";
pub(crate) const SENT: &str = "SENT";
pub(crate) const SPAM: &str = "SPAM";
pub(crate) const TRASH: &str = "TRASH";
pub(crate) const STARRED: &str = "STARRED";
pub(crate) const UNREAD: &str = "UNREAD";
/// Gmail has no archive label: archived mail is mail that left the inbox.
pub(crate) const ARCHIVE: &str = "ARCHIVE";
/// Snoozes are kept by this app, so the folder is too.
pub(crate) const SNOOZED: &str = "SNOOZED";

/// The system labels shown as folders, in Gmail's order, with their roles.
const SYSTEM_MAILBOXES: &[(&str, &str, Option<&str>)] = &[
    (INBOX, "Inbox", Some("inbox")),
    // The shared interface already provides a Starred search tab.
    ("IMPORTANT", "Important", None),
    (SNOOZED, "Snoozed", Some("snoozed")),
    (SENT, "Sent", Some("sent")),
    (DRAFT, "Drafts", Some("drafts")),
    (ARCHIVE, "Done", Some("archive")),
    (SPAM, "Spam", Some("junk")),
    (TRASH, "Trash", Some("trash")),
];

pub(crate) fn is_system_mailbox(id: &str) -> bool {
    id == STARRED
        || SYSTEM_MAILBOXES
            .iter()
            .any(|(system_id, _, _)| *system_id == id)
}

/// Folders for a Gmail account: the useful system labels, then the user's
/// visible labels. `counts` carries the per-label totals Gmail only returns
/// from `labels.get`.
pub(crate) fn gmail_mailboxes(
    labels: &[GmailLabel],
    counts: &HashMap<String, GmailLabel>,
) -> Vec<Mailbox> {
    let mut mailboxes = SYSTEM_MAILBOXES
        .iter()
        .map(|(id, name, role)| {
            let counted = counts.get(*id);
            Mailbox {
                id: (*id).to_string(),
                name: (*name).to_string(),
                role: role.map(str::to_string),
                unread_emails: counted.map_or(0, |label| label.threads_unread),
                total_emails: counted.map_or(0, |label| label.threads_total),
                rights: mailbox_rights(*id),
            }
        })
        .collect::<Vec<_>>();
    let mut user_labels = labels
        .iter()
        .filter(|label| label.kind == "user")
        .filter(|label| label.label_list_visibility.as_deref() != Some("labelHide"))
        .map(|label| Mailbox {
            id: label.id.clone(),
            name: label.name.clone(),
            role: None,
            unread_emails: label.threads_unread,
            total_emails: label.threads_total,
            rights: mailbox_rights(label.id.as_str()),
        })
        .collect::<Vec<_>>();
    user_labels.sort_by_key(|mailbox| mailbox.name.to_lowercase());
    mailboxes.extend(user_labels);
    mailboxes
}

fn mailbox_rights(id: &str) -> MailboxRights {
    let system = is_system_mailbox(id);
    MailboxRights {
        may_read_items: true,
        may_add_items: !matches!(id, SENT | DRAFT),
        may_remove_items: true,
        may_set_seen: true,
        may_set_keywords: true,
        may_create_child: !system,
        may_rename: !system,
        may_delete: !system,
        may_submit: false,
    }
}

/// A message as the shared mail pipeline expects it. `bodies` carries the
/// decoded text of each body part, keyed by Gmail part path, for messages
/// fetched in full.
pub(crate) fn gmail_email(message: &GmailMessage, bodies: &HashMap<String, String>) -> JmapEmail {
    let payload = &message.payload;
    let header = |name: &str| payload.header(name).unwrap_or_default();
    let received_at = internal_date(message.internal_date.as_str());
    let sent_at = parse_mail_date(header("Date")).unwrap_or_else(|| received_at.clone());
    let mut layout = BodyLayout::default();
    collect_parts(payload, "0", false, &mut layout);
    let mut body_values = HashMap::new();
    for part in layout.text.iter().chain(layout.html.iter()) {
        if let Some(text) = bodies.get(part.part_id.as_str()) {
            body_values.insert(
                part.part_id.clone(),
                JmapEmailBodyValue {
                    value: text.clone(),
                },
            );
        }
    }
    let labels = message.label_ids.as_slice();
    let mut mailbox_ids = labels
        .iter()
        .map(|label| (label.clone(), true))
        .collect::<HashMap<_, _>>();
    if !labels
        .iter()
        .any(|label| matches!(label.as_str(), INBOX | SPAM | TRASH | DRAFT))
    {
        mailbox_ids.insert(ARCHIVE.to_string(), true);
    }
    let mut keywords = HashMap::new();
    keywords.insert("$seen".to_string(), !has_label(labels, UNREAD));
    keywords.insert("$flagged".to_string(), has_label(labels, STARRED));
    keywords.insert("$draft".to_string(), has_label(labels, DRAFT));
    let attachments = layout
        .attachments
        .into_iter()
        .map(|attachment| JmapEmailAttachment {
            blob_id: attachment
                .attachment_id
                .map(|attachment_id| attachment_blob_id(message.id.as_str(), &attachment_id)),
            ..attachment.record
        })
        .collect::<Vec<_>>();
    JmapEmail {
        id: message.id.clone(),
        thread_id: message.thread_id.clone(),
        message_id: message_ids(header("Message-ID")),
        in_reply_to: message_ids(header("In-Reply-To")),
        references: message_ids(header("References")),
        received_at,
        sent_at,
        sender: parse_address_list(header("Sender")),
        reply_to: parse_address_list(header("Reply-To")),
        from: parse_address_list(header("From")),
        to: parse_address_list(header("To")),
        cc: parse_address_list(header("Cc")),
        bcc: parse_address_list(header("Bcc")),
        mailbox_ids,
        subject: Some(header("Subject").to_string()),
        preview: Some(unescape_html(message.snippet.as_str())),
        has_attachment: !attachments.is_empty()
            || payload.mime_type.eq_ignore_ascii_case("multipart/mixed"),
        keywords,
        text_body: layout.text,
        html_body: layout.html,
        body_values,
        attachments,
    }
}

fn has_label(labels: &[String], label: &str) -> bool {
    labels.iter().any(|candidate| candidate == label)
}

/// An attachment's blob ID names the message and Gmail's attachment handle,
/// which is all `messages.attachments.get` needs.
pub(crate) fn attachment_blob_id(message_id: &str, attachment_id: &str) -> String {
    format!("{message_id}/{attachment_id}")
}

pub(crate) fn split_attachment_blob_id(blob_id: &str) -> Option<(&str, &str)> {
    blob_id
        .split_once('/')
        .filter(|(message_id, attachment_id)| !message_id.is_empty() && !attachment_id.is_empty())
}

#[derive(Default)]
struct BodyLayout {
    text: Vec<JmapEmailBodyPart>,
    html: Vec<JmapEmailBodyPart>,
    attachments: Vec<PartAttachment>,
}

struct PartAttachment {
    attachment_id: Option<String>,
    record: JmapEmailAttachment,
}

/// Body parts that still need their text fetched: inline data is decoded
/// straight away, larger parts come back with an attachment handle.
pub(crate) struct PendingBody {
    pub(crate) part_id: String,
    pub(crate) attachment_id: String,
    pub(crate) charset: Option<String>,
}

/// Decodes the text of every body part of a full message. Parts Gmail moved
/// out of line are returned for the caller to fetch.
pub(crate) fn inline_bodies(message: &GmailMessage) -> (HashMap<String, String>, Vec<PendingBody>) {
    let mut layout = BodyLayout::default();
    collect_parts(&message.payload, "0", false, &mut layout);
    let mut bodies = HashMap::new();
    let mut pending = Vec::new();
    for part in layout.text.iter().chain(layout.html.iter()) {
        let Some(source) = find_part(&message.payload, part.part_id.as_str()) else {
            continue;
        };
        match (&source.body.data, &source.body.attachment_id) {
            (Some(data), _) => {
                bodies.insert(
                    part.part_id.clone(),
                    decode_text(data, part.charset.as_deref()),
                );
            }
            (None, Some(attachment_id)) => pending.push(PendingBody {
                part_id: part.part_id.clone(),
                attachment_id: attachment_id.clone(),
                charset: part.charset.clone(),
            }),
            (None, None) => {}
        }
    }
    (bodies, pending)
}

fn collect_parts(part: &GmailPart, path: &str, in_alternative: bool, layout: &mut BodyLayout) {
    let mime_type = part.mime_type.to_ascii_lowercase();
    if mime_type.starts_with("multipart/") {
        let alternative = mime_type == "multipart/alternative";
        for (index, child) in part.parts.iter().enumerate() {
            collect_parts(child, &format!("{path}.{index}"), alternative, layout);
        }
        return;
    }
    let disposition = part.header("Content-Disposition").map(|value| {
        value
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
    });
    let is_attachment = !part.filename.is_empty() || disposition.as_deref() == Some("attachment");
    let is_text = mime_type == "text/plain" || mime_type == "text/html";
    if is_text && !is_attachment {
        let body_part = JmapEmailBodyPart {
            part_id: path.to_string(),
            content_type: Some(mime_type.clone()),
            charset: content_type_param(part.header("Content-Type"), "charset"),
        };
        if mime_type == "text/html" {
            layout.html.push(body_part);
        } else {
            if !in_alternative {
                layout.html.push(body_part.clone());
            }
            layout.text.push(body_part);
        }
        return;
    }
    if part.body.attachment_id.is_none() && part.filename.is_empty() {
        return;
    }
    layout.attachments.push(PartAttachment {
        attachment_id: part.body.attachment_id.clone(),
        record: JmapEmailAttachment {
            blob_id: None,
            name: (!part.filename.is_empty()).then(|| part.filename.clone()),
            content_type: if mime_type.is_empty() {
                "application/octet-stream".to_string()
            } else {
                mime_type
            },
            size: part.body.size,
            disposition: disposition.or_else(|| Some("attachment".to_string())),
        },
    });
}

fn find_part<'a>(root: &'a GmailPart, path: &str) -> Option<&'a GmailPart> {
    let mut part = root;
    for index in path.split('.').skip(1) {
        part = part.parts.get(index.parse::<usize>().ok()?)?;
    }
    Some(part)
}

fn content_type_param(content_type: Option<&str>, name: &str) -> Option<String> {
    content_type?
        .split(';')
        .skip(1)
        .filter_map(|param| param.split_once('='))
        .find(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim().trim_matches('"').to_string())
}

/// Gmail hands body data back as base64url of the part's bytes, still in the
/// part's own charset.
pub(crate) fn decode_text(data: &str, charset: Option<&str>) -> String {
    let bytes = decode_base64url(data);
    let encoding = charset
        .and_then(|label| encoding_rs::Encoding::for_label(label.as_bytes()))
        .unwrap_or(encoding_rs::UTF_8);
    encoding.decode(&bytes).0.into_owned()
}

pub(crate) fn decode_base64url(data: &str) -> Vec<u8> {
    let cleaned = data.trim().trim_end_matches('=');
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(cleaned)
        .or_else(|_| URL_SAFE.decode(data.trim()))
        .unwrap_or_default()
}

fn internal_date(millis: &str) -> String {
    millis
        .parse::<i128>()
        .ok()
        .and_then(|millis| OffsetDateTime::from_unix_timestamp_nanos(millis * 1_000_000).ok())
        .and_then(|date| date.replace_nanosecond(0).ok())
        .and_then(|date| date.format(&Rfc3339).ok())
        .unwrap_or_default()
}

fn parse_mail_date(value: &str) -> Option<String> {
    let value = value.trim();
    // Drop a trailing "(UTC)"-style comment, which RFC 2822 parsers reject.
    let value = value
        .split_once(" (")
        .map_or(value, |(date, _)| date)
        .trim();
    OffsetDateTime::parse(value, &Rfc2822)
        .ok()
        .and_then(|date| date.to_offset(time::UtcOffset::UTC).format(&Rfc3339).ok())
}

fn message_ids(value: &str) -> Vec<String> {
    value
        .split(|character: char| character.is_whitespace() || character == ',')
        .map(|id| id.trim().trim_start_matches('<').trim_end_matches('>'))
        .filter(|id| !id.is_empty())
        .map(str::to_string)
        .collect()
}

/// Splits an address header such as `"Lee, Ada" <ada@example.com>, bo@x.io`
/// into names and addresses, respecting quotes and angle brackets.
pub(crate) fn parse_address_list(value: &str) -> Vec<JmapEmailAddress> {
    let mut entries = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut in_angle = false;
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if in_quotes => {
                current.push(character);
                escaped = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                current.push(character);
            }
            '<' if !in_quotes => {
                in_angle = true;
                current.push(character);
            }
            '>' if !in_quotes => {
                in_angle = false;
                current.push(character);
            }
            ',' | ';' if !in_quotes && !in_angle => {
                entries.push(std::mem::take(&mut current));
            }
            _ => current.push(character),
        }
    }
    entries.push(current);
    entries
        .iter()
        .filter_map(|entry| parse_address(entry))
        .collect()
}

fn parse_address(entry: &str) -> Option<JmapEmailAddress> {
    let entry = entry.trim();
    if entry.is_empty() {
        return None;
    }
    let (name, email) = match (entry.rfind('<'), entry.rfind('>')) {
        (Some(start), Some(end)) if start < end => {
            (entry[..start].trim(), entry[start + 1..end].trim())
        }
        _ => ("", entry),
    };
    let email = email.trim_matches(|character| character == '<' || character == '>');
    if !email.contains('@') {
        return None;
    }
    let name = name
        .trim()
        .trim_matches('"')
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .trim()
        .to_string();
    Some(JmapEmailAddress {
        name: (!name.is_empty()).then_some(name),
        email: email.to_string(),
    })
}

/// Gmail snippets are HTML-escaped.
pub(crate) fn unescape_html(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        output.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';').filter(|end| *end <= 10) else {
            output.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                .and_then(char::from_u32),
        };
        match decoded {
            Some(character) => {
                output.push(character);
                rest = &rest[end + 1..];
            }
            None => {
                output.push('&');
                rest = &rest[1..];
            }
        }
    }
    output.push_str(rest);
    output
}

#[cfg(test)]
mod tests {
    use super::{
        gmail_email, inline_bodies, parse_address_list, parse_mail_date, unescape_html, ARCHIVE,
    };
    use crate::live::gmail::api::GmailMessage;

    fn message(json: serde_json::Value) -> GmailMessage {
        serde_json::from_value(json).expect("gmail message")
    }

    #[test]
    fn address_lists_keep_quoted_commas_together() {
        let addresses =
            parse_address_list("\"Lee, Ada\" <ada@example.com>, bo@example.org; Cy <cy@x.io>");
        let pairs = addresses
            .iter()
            .map(|address| (address.name.clone(), address.email.clone()))
            .collect::<Vec<_>>();
        assert_eq!(
            pairs,
            vec![
                (Some("Lee, Ada".to_string()), "ada@example.com".to_string()),
                (None, "bo@example.org".to_string()),
                (Some("Cy".to_string()), "cy@x.io".to_string()),
            ]
        );
    }

    #[test]
    fn mail_dates_become_utc_rfc3339() {
        assert_eq!(
            parse_mail_date("Mon, 5 Oct 2026 09:30:00 -0700 (PDT)").as_deref(),
            Some("2026-10-05T16:30:00Z")
        );
    }

    #[test]
    fn snippets_are_unescaped() {
        assert_eq!(
            unescape_html("Tom &amp; Jerry&#39;s &lt;plan&gt; &#x2014; ok"),
            "Tom & Jerry's <plan> \u{2014} ok"
        );
    }

    #[test]
    fn full_messages_map_bodies_attachments_and_flags() {
        let message = message(serde_json::json!({
            "id": "m1",
            "threadId": "t1",
            "labelIds": ["UNREAD", "STARRED", "CATEGORY_UPDATES"],
            "snippet": "Hi &amp; welcome",
            "internalDate": "1791185400000",
            "payload": {
                "mimeType": "multipart/mixed",
                "headers": [
                    {"name": "From", "value": "Ada Lee <ada@example.com>"},
                    {"name": "To", "value": "bo@example.org"},
                    {"name": "Subject", "value": "Plans"},
                    {"name": "Message-ID", "value": "<abc@example.com>"}
                ],
                "parts": [
                    {
                        "mimeType": "multipart/alternative",
                        "parts": [
                            {"mimeType": "text/plain", "headers": [{"name": "Content-Type", "value": "text/plain; charset=\"UTF-8\""}], "body": {"size": 5, "data": "SGVsbG8"}},
                            {"mimeType": "text/html", "body": {"size": 12, "data": "PGI-SGVsbG88L2I-"}}
                        ]
                    },
                    {"mimeType": "application/pdf", "filename": "plan.pdf", "body": {"size": 2048, "attachmentId": "att-1"}}
                ]
            }
        }));
        let (bodies, pending) = inline_bodies(&message);
        assert!(pending.is_empty());
        let email = gmail_email(&message, &bodies);
        assert_eq!(email.message_id, vec!["abc@example.com".to_string()]);
        assert_eq!(email.received_at, "2026-10-05T07:30:00Z");
        assert_eq!(email.preview.as_deref(), Some("Hi & welcome"));
        assert_eq!(email.keywords.get("$seen"), Some(&false));
        assert_eq!(email.keywords.get("$flagged"), Some(&true));
        assert!(email.mailbox_ids.contains_key(ARCHIVE));
        assert_eq!(email.text_body.len(), 1);
        assert_eq!(email.html_body.len(), 1);
        let html_id = email.html_body[0].part_id.as_str();
        assert_eq!(email.body_values[html_id].value, "<b>Hello</b>");
        assert_eq!(email.attachments.len(), 1);
        assert_eq!(email.attachments[0].blob_id.as_deref(), Some("m1/att-1"));
        assert_eq!(email.attachments[0].name.as_deref(), Some("plan.pdf"));
        assert!(email.has_attachment);
    }
}
