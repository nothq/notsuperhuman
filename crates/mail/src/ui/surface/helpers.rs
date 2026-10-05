#[cfg(any(test, feature = "test-support"))]
use super::MailContactHistoryRow;
use super::{
    MailBodyDisplay, MailBodyParagraph, MailBodySegment, MailBodySegmentKind,
    MailListAttachment, MailListSource, MailListText, MailListThread,
    MailMessage, MailSearchQuery, MailSplitDefinition, MailTab, MailThreadDetail,
    MailThreadMessageDetail, MailWorkspace,
};
use crate::model::{MailSearchSnippet, MailSearchSnippetText};
use crate::ui::{
    format_mail_received_at, mail_address_display, mailbox_display_label, MailAttachment,
};
use gpui::SharedString;
use std::{collections::HashMap, path::Path, sync::Arc};

mod body;
mod compose;
mod contact;
mod detail;
mod grouping;
mod search_highlight;

use self::search_highlight::{mail_search_highlight_ranges, MailSearchHighlightTerms};

pub(crate) use self::body::*;
pub(crate) use self::compose::*;
pub(crate) use self::contact::*;
pub(crate) use self::detail::*;
pub(crate) use self::grouping::*;

const MAIL_DRAFT_TAG_FILL: u32 = 0xd9d5f5;
const MAIL_SEARCH_TAG_FILL: u32 = 0xbbc5ef;

pub fn default_mail_selected_tab_id() -> String {
    String::new()
}

pub fn default_mail_selected_thread_id() -> Option<String> {
    None
}

pub(crate) fn mail_tabs_with_splits(
    workspace: &MailWorkspace,
    splits: &[MailSplitDefinition],
    splits_supported: bool,
) -> Vec<MailTab> {
    let mut tabs = workspace
        .mailboxes
        .iter()
        .map(|mailbox| MailTab {
            source: MailListSource::Mailbox(mailbox.id.clone()),
            label: mailbox_display_label(mailbox),
            count: if mailbox.unread_emails == 0 {
                String::new()
            } else {
                mailbox.unread_emails.to_string()
            },
        })
        .collect::<Vec<_>>();
    let inbox_index = workspace
        .mailboxes
        .iter()
        .position(|mailbox| mailbox.role.as_deref() == Some("inbox"));
    let insertion_index = inbox_index.map(|index| index + 1).unwrap_or(0);
    let mut special_tabs = Vec::new();
    if splits_supported && inbox_index.is_some() {
        special_tabs.extend(
            splits
                .iter()
                .filter(|split| split.is_enabled())
                .map(|split| MailTab {
                    source: MailListSource::Split(split.id().clone()),
                    label: split.name().to_string(),
                    count: String::new(),
                }),
        );
        special_tabs.push(MailTab {
            source: MailListSource::Other,
            label: "Other".to_string(),
            count: String::new(),
        });
    }
    special_tabs.push(MailTab {
        source: MailListSource::Starred,
        label: "Starred".to_string(),
        count: String::new(),
    });
    tabs.splice(insertion_index..insertion_index, special_tabs);
    tabs
}

pub(crate) fn mail_list_threads(workspace: &MailWorkspace) -> Vec<MailListThread> {
    mail_list_threads_for_messages(&workspace.messages, true)
}

pub(crate) fn mail_list_threads_for_messages(
    messages: &[MailMessage],
    include_sections: bool,
) -> Vec<MailListThread> {
    let mut previous_group = None;
    messages
        .iter()
        .map(|message| {
            let group = mail_section_group(&message.received_at);
            let section_label =
                (include_sections && previous_group != Some(group)).then_some(group.into());
            previous_group = Some(group);
            MailListThread {
                id: message.thread_id.clone(),
                message_id: message.id.clone(),
                section_label,
                sender: MailListText::plain(mail_primary_sender_label(message)),
                subject: MailListText::plain(mail_subject_label(message.subject.as_str())),
                preview: MailListText::plain(mail_preview_label(message)),
                date_label: SharedString::from(format_mail_received_at(&message.received_at)),
                tag_label: message.is_draft.then_some(SharedString::from("Draft")),
                tag_fill: MAIL_DRAFT_TAG_FILL,
                unread: message.is_unread,
                starred: message.is_starred,
                has_attachment: message.has_attachment || !message.attachments.is_empty(),
                attachments: mail_list_attachments(message),
            }
        })
        .collect()
}

pub(crate) fn mail_search_list_threads(
    workspace: &MailWorkspace,
    messages: &[MailMessage],
    query: &MailSearchQuery,
    snippets: &HashMap<String, MailSearchSnippet>,
    thread_message_counts: &HashMap<String, usize>,
) -> Vec<MailListThread> {
    let terms = MailSearchHighlightTerms::from_query(query);
    messages
        .iter()
        .map(|message| {
            mail_search_list_thread(
                workspace,
                message,
                &terms,
                snippets.get(message.id.as_str()),
                thread_message_counts,
            )
        })
        .collect()
}

fn mail_search_list_thread(
    workspace: &MailWorkspace,
    message: &MailMessage,
    terms: &MailSearchHighlightTerms,
    snippet: Option<&MailSearchSnippet>,
    thread_message_counts: &HashMap<String, usize>,
) -> MailListThread {
    let sender_terms = mail_search_sender_terms(workspace, message, terms);
    let mut sender = mail_search_sender_label(workspace, message);
    let thread_message_count = thread_message_counts
        .get(message.thread_id.as_str())
        .copied()
        .unwrap_or(1);
    if thread_message_count > 1 {
        sender.push_str(format!(" ({thread_message_count})").as_str());
    }
    MailListThread {
        id: message.thread_id.clone(),
        message_id: message.id.clone(),
        section_label: None,
        sender: mail_search_list_text(sender, sender_terms.as_slice()),
        subject: mail_search_subject_text(message, terms, snippet),
        preview: mail_search_preview_text(message, terms, snippet),
        date_label: SharedString::from(
            format_mail_received_at(&message.received_at).to_ascii_uppercase(),
        ),
        tag_label: mail_search_tag_label(workspace, message).map(SharedString::from),
        tag_fill: MAIL_SEARCH_TAG_FILL,
        unread: message.is_unread,
        starred: message.is_starred,
        has_attachment: message.has_attachment || !message.attachments.is_empty(),
        attachments: mail_list_attachments(message),
    }
}

fn mail_search_sender_terms(
    workspace: &MailWorkspace,
    message: &MailMessage,
    terms: &MailSearchHighlightTerms,
) -> Vec<String> {
    let directional_terms = if mail_message_is_outgoing(workspace, message) {
        &terms.to
    } else {
        &terms.from
    };
    terms
        .text
        .iter()
        .chain(directional_terms)
        .cloned()
        .collect()
}

fn mail_search_subject_text(
    message: &MailMessage,
    terms: &MailSearchHighlightTerms,
    snippet: Option<&MailSearchSnippet>,
) -> MailListText {
    let subject_terms = terms
        .text
        .iter()
        .chain(terms.subject.iter())
        .cloned()
        .collect::<Vec<_>>();
    snippet
        .and_then(MailSearchSnippet::subject)
        .map(mail_list_text_from_snippet)
        .unwrap_or_else(|| {
            mail_search_list_text(
                mail_subject_label(message.subject.as_str()),
                subject_terms.as_slice(),
            )
        })
}

fn mail_search_preview_text(
    message: &MailMessage,
    terms: &MailSearchHighlightTerms,
    snippet: Option<&MailSearchSnippet>,
) -> MailListText {
    snippet
        .and_then(MailSearchSnippet::preview)
        .map(mail_list_text_from_snippet)
        .unwrap_or_else(|| {
            mail_search_list_text(mail_preview_label(message), terms.text.as_slice())
        })
}

pub(crate) fn mail_search_list_text(value: impl Into<String>, terms: &[String]) -> MailListText {
    let value = value.into();
    let highlight_ranges = mail_search_highlight_ranges(value.as_str(), terms);
    MailListText::highlighted(value, highlight_ranges)
}

fn mail_list_text_from_snippet(snippet: &MailSearchSnippetText) -> MailListText {
    let mut text = String::new();
    let mut highlight_ranges = Vec::new();
    for segment in snippet.segments() {
        let start = text.len();
        text.push_str(segment.text());
        if segment.is_highlighted() && start < text.len() {
            highlight_ranges.push(start..text.len());
        }
    }
    MailListText::highlighted(text, highlight_ranges)
}

fn mail_search_sender_label(workspace: &MailWorkspace, message: &MailMessage) -> String {
    if mail_message_is_outgoing(workspace, message) && !message.to.is_empty() {
        return message
            .to
            .iter()
            .map(|address| mail_address_display(Some(address)))
            .collect::<Vec<_>>()
            .join(", ");
    }
    mail_primary_sender_label(message)
}

fn mail_message_is_outgoing(workspace: &MailWorkspace, message: &MailMessage) -> bool {
    workspace.mailbox_email.as_deref().is_some_and(|owner| {
        message
            .sender
            .iter()
            .chain(message.from.iter())
            .any(|address| address.email.eq_ignore_ascii_case(owner))
    })
}

fn mail_search_tag_label(workspace: &MailWorkspace, message: &MailMessage) -> Option<String> {
    workspace
        .mailboxes
        .iter()
        .find(|mailbox| {
            mailbox.role.is_none()
                && message
                    .mailbox_ids
                    .iter()
                    .any(|mailbox_id| mailbox_id == &mailbox.id)
        })
        .map(mailbox_display_label)
        .or_else(|| message.is_draft.then(|| "Draft".to_string()))
}

fn mail_list_attachments(message: &MailMessage) -> Arc<[MailListAttachment]> {
    Arc::from(
        message
            .attachments
            .iter()
            .map(mail_list_attachment)
            .collect::<Vec<_>>(),
    )
}

fn mail_list_attachment(attachment: &MailAttachment) -> MailListAttachment {
    let path = Path::new(attachment.name.as_str());
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let type_label = if extension.is_empty() {
        attachment_type_fallback_label(attachment)
    } else {
        extension.to_ascii_uppercase()
    };
    let display_name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Attachment")
        .to_string();
    MailListAttachment {
        type_label: SharedString::from(type_label),
        type_fill: attachment_type_fill(attachment, extension.as_str()),
        display_name: SharedString::from(display_name),
    }
}

fn attachment_type_fallback_label(attachment: &MailAttachment) -> String {
    let content_type = attachment.content_type.to_ascii_lowercase();
    if content_type == "application/pdf" {
        "PDF"
    } else if attachment_is_spreadsheet(content_type.as_str(), "") {
        "XLS"
    } else if attachment_is_document(content_type.as_str(), "") {
        "DOC"
    } else {
        "FILE"
    }
    .to_string()
}

fn attachment_type_fill(attachment: &MailAttachment, extension: &str) -> u32 {
    let content_type = attachment.content_type.to_ascii_lowercase();
    if content_type == "application/pdf" || extension == "pdf" {
        0xf77b7b
    } else if attachment_is_spreadsheet(content_type.as_str(), extension) {
        0x8ecc8b
    } else if attachment_is_document(content_type.as_str(), extension) {
        0x88afe6
    } else {
        0x8a9ab0
    }
}

fn attachment_is_spreadsheet(content_type: &str, extension: &str) -> bool {
    matches!(extension, "csv" | "xls" | "xlsx" | "ods" | "numbers")
        || content_type.contains("spreadsheet")
        || content_type.contains("excel")
        || content_type == "text/csv"
}

fn attachment_is_document(content_type: &str, extension: &str) -> bool {
    matches!(extension, "doc" | "docx" | "odt" | "pages" | "rtf" | "txt")
        || content_type.contains("word")
        || content_type.contains("document")
        || content_type.starts_with("text/")
}

pub(crate) fn mail_list_thread_by_id(
    workspace: &MailWorkspace,
    thread_id: &str,
) -> Option<MailListThread> {
    mail_list_threads(workspace)
        .into_iter()
        .find(|thread| thread.id == thread_id)
}
