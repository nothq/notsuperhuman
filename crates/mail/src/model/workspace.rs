use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::{
    MailBlobId, MailSearchExpression, MailSearchPredicate, MailSearchQuery, MailSearchSnippet,
    MailSearchText,
};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailAccountInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub is_primary: bool,
    #[serde(default)]
    pub is_personal: bool,
    #[serde(default)]
    pub is_read_only: bool,
    #[serde(default)]
    pub can_submit: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailWorkspace {
    pub account_id: String,
    pub mailbox_email: Option<String>,
    pub display_name: String,
    pub owner_username: String,
    pub selected_mailbox_id: String,
    pub mailboxes: Vec<Mailbox>,
    pub messages: Vec<MailMessage>,
    #[serde(default)]
    pub message_next_position: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailMessagePage {
    pub mailbox_id: String,
    pub position: usize,
    pub messages: Vec<MailMessage>,
    pub next_position: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailSearchPage {
    pub position: usize,
    pub messages: Vec<MailMessage>,
    pub next_position: Option<usize>,
    #[serde(default)]
    pub snippets: HashMap<String, MailSearchSnippet>,
    #[serde(default)]
    pub thread_message_counts: HashMap<String, usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailThread {
    pub id: String,
    pub messages: Vec<MailMessage>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailReadState {
    Read,
    Unread,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailStarState {
    Starred,
    Unstarred,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailThreadTriageChange {
    Read(MailReadState),
    Star(MailStarState),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailThreadTriageRequest {
    pub mailbox_id: String,
    pub thread_id: String,
    pub change: MailThreadTriageChange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailThreadTriageResult {
    pub workspace: MailWorkspace,
    pub thread: MailThread,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mailbox {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    pub unread_emails: u64,
    pub total_emails: u64,
    #[serde(default)]
    pub rights: MailboxRights,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailboxRights {
    pub may_read_items: bool,
    pub may_add_items: bool,
    pub may_remove_items: bool,
    pub may_set_seen: bool,
    pub may_set_keywords: bool,
    pub may_create_child: bool,
    pub may_rename: bool,
    pub may_delete: bool,
    pub may_submit: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailMessage {
    pub id: String,
    pub thread_id: String,
    #[serde(default)]
    pub message_id: Vec<String>,
    #[serde(default)]
    pub in_reply_to: Vec<String>,
    #[serde(default)]
    pub references: Vec<String>,
    pub subject: String,
    pub preview: String,
    pub received_at: String,
    #[serde(default)]
    pub sent_at: String,
    #[serde(default)]
    pub sender: Vec<MailAddress>,
    #[serde(default)]
    pub reply_to: Vec<MailAddress>,
    pub from: Vec<MailAddress>,
    pub to: Vec<MailAddress>,
    #[serde(default)]
    pub cc: Vec<MailAddress>,
    #[serde(default)]
    pub bcc: Vec<MailAddress>,
    #[serde(default)]
    pub mailbox_ids: Vec<String>,
    pub has_attachment: bool,
    pub is_unread: bool,
    pub is_starred: bool,
    #[serde(default)]
    pub is_draft: bool,
    pub body_text: String,
    #[serde(default)]
    pub body_html: Option<String>,
    #[serde(default)]
    pub display_body: MailDisplayBody,
    #[serde(default)]
    pub body_loaded: bool,
    pub attachments: Vec<MailAttachment>,
}

impl MailMessage {
    pub fn matches_search_query(&self, query: &MailSearchQuery) -> bool {
        self.matches_search_expression(query.expression())
    }

    fn matches_search_expression(&self, expression: &MailSearchExpression) -> bool {
        match expression {
            MailSearchExpression::All(expressions) => expressions
                .iter()
                .all(|expression| self.matches_search_expression(expression)),
            MailSearchExpression::Any(expressions) => expressions
                .iter()
                .any(|expression| self.matches_search_expression(expression)),
            MailSearchExpression::Not(expression) => !self.matches_search_expression(expression),
            MailSearchExpression::Predicate(predicate) => self.matches_search_predicate(predicate),
        }
    }

    fn matches_search_predicate(&self, predicate: &MailSearchPredicate) -> bool {
        match predicate {
            MailSearchPredicate::Text(text) => self.matches_search_text(text),
            MailSearchPredicate::From(text) => addresses_match(&self.from, text),
            MailSearchPredicate::To(text) => addresses_match(&self.to, text),
            MailSearchPredicate::Subject(text) => value_matches(&self.subject, text),
            MailSearchPredicate::HasAttachment => self.has_attachment,
            MailSearchPredicate::InMailbox(mailbox) => self
                .mailbox_ids
                .iter()
                .any(|mailbox_id| mailbox_id.eq_ignore_ascii_case(mailbox.as_str())),
            MailSearchPredicate::Unread => self.is_unread,
            MailSearchPredicate::Starred => self.is_starred,
            MailSearchPredicate::Shared => false,
            MailSearchPredicate::Before(date) => message_date(self)
                .is_some_and(|message_date| message_date < date.iso_date().as_str()),
            MailSearchPredicate::After(date) => message_date(self)
                .is_some_and(|message_date| message_date > date.iso_date().as_str()),
            MailSearchPredicate::OlderThan(_) | MailSearchPredicate::NewerThan(_) => true,
        }
    }

    fn matches_search_text(&self, query: &MailSearchText) -> bool {
        [
            self.subject.as_str(),
            self.preview.as_str(),
            self.body_text.as_str(),
        ]
        .into_iter()
        .any(|value| value_matches(value, query))
            || [
                self.sender.as_slice(),
                self.reply_to.as_slice(),
                self.from.as_slice(),
                self.to.as_slice(),
                self.cc.as_slice(),
                self.bcc.as_slice(),
            ]
            .into_iter()
            .flatten()
            .any(|address| {
                value_matches(&address.name, query) || value_matches(&address.email, query)
            })
    }
}

fn addresses_match(addresses: &[MailAddress], query: &MailSearchText) -> bool {
    addresses
        .iter()
        .any(|address| value_matches(&address.name, query) || value_matches(&address.email, query))
}

fn value_matches(value: &str, query: &MailSearchText) -> bool {
    value
        .to_lowercase()
        .contains(query.value().to_lowercase().as_str())
}

fn message_date(message: &MailMessage) -> Option<&str> {
    message.received_at.get(..10)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MailDisplayBody {
    PlainText {
        text: String,
        source: MailBodySource,
    },
    Html {
        html: String,
        source: MailBodySource,
    },
}

impl Default for MailDisplayBody {
    fn default() -> Self {
        Self::PlainText {
            text: String::new(),
            source: MailBodySource::Unknown,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MailBodySource {
    #[default]
    Unknown,
    JmapTextBody,
    JmapHtmlBody,
    CompatibilityRepair,
    Preview,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailAddress {
    pub name: String,
    pub email: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailAttachment {
    #[serde(default, deserialize_with = "deserialize_optional_mail_blob_id")]
    pub blob_id: Option<MailBlobId>,
    pub name: String,
    pub content_type: String,
    pub size: u64,
    #[serde(default)]
    pub disposition: Option<String>,
}

fn deserialize_optional_mail_blob_id<'de, D>(
    deserializer: D,
) -> Result<Option<MailBlobId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
        .map(|blob_id| blob_id.and_then(|blob_id| MailBlobId::parse(blob_id).ok()))
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailIdentity {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub reply_to: Vec<MailAddress>,
    #[serde(default)]
    pub bcc: Vec<MailAddress>,
    #[serde(default)]
    pub text_signature: String,
    #[serde(default)]
    pub html_signature: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailUploadFile {
    pub name: String,
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailDraftRequest {
    pub identity_id: String,
    pub from: MailAddress,
    #[serde(default)]
    pub reply_to: Vec<MailAddress>,
    #[serde(default)]
    pub to: Vec<MailAddress>,
    #[serde(default)]
    pub cc: Vec<MailAddress>,
    pub subject: String,
    pub body_text: String,
    #[serde(default)]
    pub attachments: Vec<MailAttachment>,
    #[serde(default)]
    pub in_reply_to: Vec<String>,
    #[serde(default)]
    pub references: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MailSendResult {
    pub draft_id: String,
    pub thread_id: String,
    pub sent_mailbox_id: String,
}
