use crate::model::{MailAddress, MailAttachment, MailSplitId};
use crate::ui::MailDocument;
use gpui::SharedString;
use std::{ops::Range, sync::Arc};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailTab {
    pub source: MailListSource,
    pub label: String,
    pub count: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MailListSource {
    Mailbox(String),
    Starred,
    Split(MailSplitId),
    Other,
}

impl MailListSource {
    pub(crate) fn debug_id(&self) -> &str {
        match self {
            Self::Mailbox(mailbox_id) => mailbox_id.as_str(),
            Self::Starred => "starred-view",
            Self::Split(split_id) => split_id.as_str(),
            Self::Other => "other-view",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailListThread {
    pub id: String,
    pub message_id: String,
    pub section_label: Option<SharedString>,
    pub sender: MailListText,
    pub subject: MailListText,
    pub preview: MailListText,
    pub date_label: SharedString,
    pub tag_label: Option<SharedString>,
    pub tag_fill: u32,
    pub unread: bool,
    pub starred: bool,
    pub has_attachment: bool,
    pub attachments: Arc<[MailListAttachment]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailListText {
    pub text: SharedString,
    pub highlight_ranges: Arc<[Range<usize>]>,
}

impl MailListText {
    pub(crate) fn plain(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            highlight_ranges: Arc::from([]),
        }
    }

    pub(crate) fn highlighted(
        text: impl Into<SharedString>,
        highlight_ranges: Vec<Range<usize>>,
    ) -> Self {
        Self {
            text: text.into(),
            highlight_ranges: Arc::from(highlight_ranges),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailListAttachment {
    pub type_label: SharedString,
    pub type_fill: u32,
    pub display_name: SharedString,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailContactHistoryRow {
    pub thread_id: String,
    pub subject: String,
    pub preview: String,
    pub timestamp: String,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailBodyParagraph {
    pub segments: Vec<MailBodySegment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailBodySegment {
    pub text: String,
    pub kind: MailBodySegmentKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MailBodySegmentKind {
    Text,
    Link,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MailBodyDisplay {
    pub document: MailDocument,
    pub paragraphs: Vec<MailBodyParagraph>,
    pub clipped: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MailThreadMessageDetail {
    pub id: String,
    pub received: bool,
    pub sender: String,
    pub sender_email: String,
    pub header: MailMessageHeaderDetails,
    pub subject: String,
    pub preview: String,
    pub display_timestamp: String,
    pub body: MailBodyDisplay,
    pub signature_lines: Vec<String>,
    pub attachments: Vec<MailAttachment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailMessageHeaderDetails {
    pub compact_label: SharedString,
    pub from: Arc<[MailMessageHeaderAddress]>,
    pub to: Arc<[MailMessageHeaderAddress]>,
    pub cc: Arc<[MailMessageHeaderAddress]>,
    pub bcc: Arc<[MailMessageHeaderAddress]>,
    pub full_timestamp: SharedString,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailMessageHeaderAddress {
    pub name: SharedString,
    pub email: Option<SharedString>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MailThreadDetail {
    pub id: String,
    pub sender: String,
    pub sender_email: String,
    pub subject: String,
    pub preview: String,
    pub subtitle: String,
    pub recipient_label: String,
    pub display_timestamp: String,
    pub tag_label: Option<String>,
    pub tag_fill: u32,
    pub unread: bool,
    pub starred: bool,
    pub message_details: Vec<MailThreadMessageDetail>,
    pub body: MailBodyDisplay,
    pub signature_lines: Vec<String>,
    pub attachments: Vec<MailAttachment>,
    pub mailbox_labels: Vec<String>,
    pub participant_labels: Vec<String>,
    pub to_labels: Vec<String>,
    pub cc_labels: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailComposeAutocompleteItem {
    pub address: MailAddress,
    pub label: String,
    pub detail: String,
}
