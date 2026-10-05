use std::collections::HashMap;

use serde::{de::DeserializeOwned, Deserialize, Deserializer};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapThread {
    pub(crate) id: String,
    #[serde(default, rename = "emailIds")]
    pub(crate) email_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapMailbox {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) role: Option<String>,
    #[serde(default, rename = "totalEmails")]
    pub(crate) total_emails: u64,
    #[serde(default, rename = "unreadEmails")]
    pub(crate) unread_emails: u64,
    #[serde(rename = "myRights")]
    pub(crate) my_rights: JmapMailboxRights,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(crate) struct JmapMailboxRights {
    #[serde(default, rename = "mayReadItems")]
    pub(crate) may_read_items: bool,
    #[serde(default, rename = "mayAddItems")]
    pub(crate) may_add_items: bool,
    #[serde(default, rename = "mayRemoveItems")]
    pub(crate) may_remove_items: bool,
    #[serde(default, rename = "maySetSeen")]
    pub(crate) may_set_seen: bool,
    #[serde(default, rename = "maySetKeywords")]
    pub(crate) may_set_keywords: bool,
    #[serde(default, rename = "mayCreateChild")]
    pub(crate) may_create_child: bool,
    #[serde(default, rename = "mayRename")]
    pub(crate) may_rename: bool,
    #[serde(default, rename = "mayDelete")]
    pub(crate) may_delete: bool,
    #[serde(default, rename = "maySubmit")]
    pub(crate) may_submit: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmail {
    pub(crate) id: String,
    #[serde(rename = "threadId")]
    pub(crate) thread_id: String,
    #[serde(
        default,
        rename = "messageId",
        deserialize_with = "deserialize_null_default"
    )]
    pub(crate) message_id: Vec<String>,
    #[serde(
        default,
        rename = "inReplyTo",
        deserialize_with = "deserialize_null_default"
    )]
    pub(crate) in_reply_to: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) references: Vec<String>,
    #[serde(default, rename = "receivedAt")]
    pub(crate) received_at: String,
    #[serde(default, rename = "sentAt")]
    pub(crate) sent_at: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) sender: Vec<JmapEmailAddress>,
    #[serde(
        default,
        rename = "replyTo",
        deserialize_with = "deserialize_null_default"
    )]
    pub(crate) reply_to: Vec<JmapEmailAddress>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) from: Vec<JmapEmailAddress>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) to: Vec<JmapEmailAddress>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) cc: Vec<JmapEmailAddress>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) bcc: Vec<JmapEmailAddress>,
    #[serde(
        default,
        rename = "mailboxIds",
        deserialize_with = "deserialize_null_default"
    )]
    pub(crate) mailbox_ids: HashMap<String, bool>,
    #[serde(default)]
    pub(crate) subject: Option<String>,
    #[serde(default)]
    pub(crate) preview: Option<String>,
    #[serde(default, rename = "hasAttachment")]
    pub(crate) has_attachment: bool,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) keywords: HashMap<String, bool>,
    #[serde(
        default,
        rename = "textBody",
        deserialize_with = "deserialize_null_default"
    )]
    pub(crate) text_body: Vec<JmapEmailBodyPart>,
    #[serde(
        default,
        rename = "htmlBody",
        deserialize_with = "deserialize_null_default"
    )]
    pub(crate) html_body: Vec<JmapEmailBodyPart>,
    #[serde(
        default,
        rename = "bodyValues",
        deserialize_with = "deserialize_null_default"
    )]
    pub(crate) body_values: HashMap<String, JmapEmailBodyValue>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) attachments: Vec<JmapEmailAttachment>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmailAddress {
    #[serde(default)]
    pub(crate) name: Option<String>,
    pub(crate) email: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmailBodyPart {
    #[serde(rename = "partId")]
    pub(crate) part_id: String,
    #[serde(default, rename = "type")]
    pub(crate) content_type: Option<String>,
    #[serde(default)]
    pub(crate) charset: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmailBodyValue {
    pub(crate) value: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapEmailAttachment {
    #[serde(default, rename = "blobId")]
    pub(crate) blob_id: Option<String>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(rename = "type")]
    pub(crate) content_type: String,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) disposition: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct JmapIdentity {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) name: String,
    pub(crate) email: String,
    #[serde(
        default,
        rename = "replyTo",
        deserialize_with = "deserialize_null_default"
    )]
    pub(crate) reply_to: Vec<JmapEmailAddress>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(crate) bcc: Vec<JmapEmailAddress>,
    #[serde(default, rename = "textSignature")]
    pub(crate) text_signature: String,
    #[serde(default, rename = "htmlSignature")]
    pub(crate) html_signature: String,
}

pub(crate) fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: DeserializeOwned + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}
