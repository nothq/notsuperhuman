use serde_json::json;

use super::into_mail_message;
use crate::live::types::{JmapEmail, JmapEmailGetResponse};
use crate::model::{MailBodySource, MailDisplayBody};

#[test]
fn jmap_email_get_response_accepts_null_collections() {
    let payload = serde_json::from_value::<JmapEmailGetResponse>(json!({
        "accountId": "account-1",
        "list": [{
            "id": "email-1",
            "threadId": "thread-1",
            "receivedAt": "2026-04-05T22:15:00Z",
            "from": null,
            "to": null,
            "cc": null,
            "subject": null,
            "preview": null,
            "hasAttachment": false,
            "keywords": null,
            "textBody": null,
            "htmlBody": null,
            "bodyValues": null,
            "attachments": null
        }]
    }))
    .expect("decode email get response");
    let email = &payload.list[0];
    assert!(email.from.is_empty());
    assert!(email.to.is_empty());
    assert!(email.cc.is_empty());
    assert!(email.keywords.is_empty());
    assert!(email.text_body.is_empty());
    assert!(email.html_body.is_empty());
    assert!(email.body_values.is_empty());
    assert!(email.attachments.is_empty());
    assert_eq!(email.subject, None);
    assert_eq!(email.preview, None);
}

#[test]
fn into_mail_message_extracts_html_body() {
    let message = into_mail_message(
        serde_json::from_value::<JmapEmail>(json!({
            "id": "email-html",
            "threadId": "thread-html",
            "receivedAt": "2026-04-08T23:10:00Z",
            "from": [{ "name": "HTML Sender", "email": "html@example.com" }],
            "to": [{ "name": "Alex", "email": "alex@example.com" }],
            "cc": [],
            "subject": "HTML mail",
            "preview": "Rich content available.",
            "hasAttachment": false,
            "keywords": { "$seen": false },
            "textBody": [],
            "htmlBody": [{ "partId": "html-1" }],
            "bodyValues": {
                "html-1": {
                    "value": "<div><p>Hello <strong>world</strong>.</p><p><a href=\"https://example.com\">notsuperhuman</a></p></div>"
                }
            },
            "attachments": []
        }))
        .expect("decode html email"),
    );
    assert_eq!(
        message.body_html.as_deref(),
        Some(
            "<div><p>Hello <strong>world</strong>.</p><p><a href=\"https://example.com\">notsuperhuman</a></p></div>"
        )
    );
    assert_eq!(
        message.display_body,
        MailDisplayBody::Html {
            html: "<div><p>Hello <strong>world</strong>.</p><p><a href=\"https://example.com\">notsuperhuman</a></p></div>".to_string(),
            source: MailBodySource::JmapHtmlBody,
        }
    );
    assert_eq!(message.body_text, "Rich content available.");
}

#[test]
fn into_mail_message_repairs_html_body_without_authored_markup_to_plaintext() {
    let plain_body = "Activate your Parcel Developer Account\n\nUse this URL to activate your account:\nhttps://parcel.example/activate?token=redacted\n\nThanks.";
    let message = into_mail_message(
        serde_json::from_value::<JmapEmail>(json!({
            "id": "email-parcel",
            "threadId": "thread-parcel",
            "receivedAt": "2026-06-29T04:21:59Z",
            "from": [{ "name": "Parcel API", "email": "api@parcel.example" }],
            "to": [{ "name": "Alex", "email": "i@example.com" }],
            "cc": [],
            "subject": "Activate your Parcel Developer Account",
            "preview": "Activate your Parcel Developer Account Use this URL to activate your account.",
            "hasAttachment": false,
            "keywords": { "$seen": false },
            "textBody": [{ "partId": "body-1", "type": "text/html", "charset": "utf-8" }],
            "htmlBody": [{ "partId": "body-1", "type": "text/html", "charset": "utf-8" }],
            "bodyValues": {
                "body-1": {
                    "value": plain_body
                }
            },
            "attachments": []
        }))
        .expect("decode html-labeled plaintext email"),
    );

    assert_eq!(message.body_html, None);
    assert_eq!(message.body_text, plain_body);
    assert_eq!(
        message.display_body,
        MailDisplayBody::PlainText {
            text: plain_body.to_string(),
            source: MailBodySource::CompatibilityRepair,
        }
    );
}
