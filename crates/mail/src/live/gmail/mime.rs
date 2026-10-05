//! Builds the RFC 5322 message behind a Gmail draft: a plain-text body,
//! optional attachments, and the threading headers replies need.

use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE},
    Engine as _,
};
use rand::{distributions::Alphanumeric, Rng};

use crate::model::{MailAddress, MailDraftRequest};

/// A file to attach, already in memory.
pub(crate) struct MimeAttachment {
    pub(crate) name: String,
    pub(crate) content_type: String,
    pub(crate) bytes: Vec<u8>,
}

/// The draft as Gmail's `raw` field wants it: base64url of the whole message.
pub(crate) fn raw_draft(request: &MailDraftRequest, attachments: &[MimeAttachment]) -> String {
    URL_SAFE.encode(draft_message(request, attachments))
}

pub(crate) fn draft_message(request: &MailDraftRequest, attachments: &[MimeAttachment]) -> String {
    let mut message = String::new();
    header(
        &mut message,
        "From",
        &address_list(std::slice::from_ref(&request.from)),
    );
    if !request.to.is_empty() {
        header(&mut message, "To", &address_list(&request.to));
    }
    if !request.cc.is_empty() {
        header(&mut message, "Cc", &address_list(&request.cc));
    }
    if !request.reply_to.is_empty() {
        header(&mut message, "Reply-To", &address_list(&request.reply_to));
    }
    header(
        &mut message,
        "Subject",
        &encode_word(request.subject.as_str()),
    );
    if let Some(parent) = request.in_reply_to.last() {
        header(&mut message, "In-Reply-To", &format!("<{parent}>"));
    }
    if !request.references.is_empty() {
        let references = request
            .references
            .iter()
            .map(|id| format!("<{id}>"))
            .collect::<Vec<_>>()
            .join(" ");
        header(&mut message, "References", &references);
    }
    header(&mut message, "MIME-Version", "1.0");
    let body = text_part(request.body_text.as_str());
    if attachments.is_empty() {
        message.push_str(&body);
        return message;
    }
    let boundary = format!(
        "notsuperhuman-{}",
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect::<String>()
    );
    header(
        &mut message,
        "Content-Type",
        &format!("multipart/mixed; boundary=\"{boundary}\""),
    );
    message.push_str("\r\n");
    message.push_str(&format!("--{boundary}\r\n"));
    message.push_str(&body);
    for attachment in attachments {
        message.push_str(&format!("\r\n--{boundary}\r\n"));
        let name = encode_word(attachment.name.as_str()).replace('"', "");
        message.push_str(&format!(
            "Content-Type: {}; name=\"{name}\"\r\n",
            attachment.content_type
        ));
        message.push_str(&format!(
            "Content-Disposition: attachment; filename=\"{name}\"\r\n"
        ));
        message.push_str("Content-Transfer-Encoding: base64\r\n\r\n");
        message.push_str(&wrapped_base64(&attachment.bytes));
    }
    message.push_str(&format!("\r\n--{boundary}--\r\n"));
    message
}

fn text_part(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\n', "\r\n");
    format!(
        "Content-Type: text/plain; charset=\"UTF-8\"\r\nContent-Transfer-Encoding: base64\r\n\r\n{}",
        wrapped_base64(normalized.as_bytes())
    )
}

fn header(message: &mut String, name: &str, value: &str) {
    let value = value.replace(['\r', '\n'], " ");
    message.push_str(name);
    message.push_str(": ");
    message.push_str(value.trim());
    message.push_str("\r\n");
}

fn address_list(addresses: &[MailAddress]) -> String {
    addresses
        .iter()
        .filter(|address| !address.email.trim().is_empty())
        .map(|address| {
            let name = address.name.trim();
            let email = address.email.trim();
            if name.is_empty() {
                email.to_string()
            } else if name.is_ascii() {
                let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
                format!("\"{escaped}\" <{email}>")
            } else {
                format!("{} <{email}>", encode_word(name))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// RFC 2047 encoded word for header text that is not plain ASCII.
fn encode_word(value: &str) -> String {
    if value.is_ascii() {
        return value.to_string();
    }
    format!("=?UTF-8?B?{}?=", STANDARD.encode(value.as_bytes()))
}

fn wrapped_base64(bytes: &[u8]) -> String {
    let encoded = STANDARD.encode(bytes);
    let mut wrapped = String::with_capacity(encoded.len() + encoded.len() / 76 * 2 + 2);
    for line in encoded.as_bytes().chunks(76) {
        wrapped.push_str(std::str::from_utf8(line).unwrap_or_default());
        wrapped.push_str("\r\n");
    }
    wrapped
}

#[cfg(test)]
mod tests {
    use super::{draft_message, MimeAttachment};
    use crate::model::{MailAddress, MailDraftRequest};

    fn request() -> MailDraftRequest {
        MailDraftRequest {
            identity_id: "ada@example.com".to_string(),
            from: MailAddress {
                name: "Ada Lee".to_string(),
                email: "ada@example.com".to_string(),
            },
            to: vec![MailAddress {
                name: "Bo".to_string(),
                email: "bo@example.org".to_string(),
            }],
            subject: "Café plans".to_string(),
            body_text: "See you\nsoon".to_string(),
            in_reply_to: vec!["parent@example.org".to_string()],
            references: vec![
                "root@example.org".to_string(),
                "parent@example.org".to_string(),
            ],
            ..MailDraftRequest::default()
        }
    }

    #[test]
    fn plain_drafts_carry_threading_headers() {
        let message = draft_message(&request(), &[]);
        assert!(message.contains("From: \"Ada Lee\" <ada@example.com>\r\n"));
        assert!(message.contains("To: \"Bo\" <bo@example.org>\r\n"));
        assert!(message.contains("Subject: =?UTF-8?B?Q2Fmw6kgcGxhbnM=?=\r\n"));
        assert!(message.contains("In-Reply-To: <parent@example.org>\r\n"));
        assert!(message.contains("References: <root@example.org> <parent@example.org>\r\n"));
        assert!(message.contains("Content-Type: text/plain; charset=\"UTF-8\""));
        assert!(!message.contains("multipart"));
    }

    #[test]
    fn attachments_make_a_multipart_message() {
        let attachment = MimeAttachment {
            name: "plan.txt".to_string(),
            content_type: "text/plain".to_string(),
            bytes: b"hello".to_vec(),
        };
        let message = draft_message(&request(), &[attachment]);
        assert!(message.contains("Content-Type: multipart/mixed; boundary=\"notsuperhuman-"));
        assert!(message.contains("Content-Disposition: attachment; filename=\"plan.txt\""));
        assert!(message.contains("aGVsbG8=\r\n"));
        assert!(message.trim_end().ends_with("--"));
    }
}
