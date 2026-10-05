use crate::model::{MailAddress, MailContactAddress, MailMessage, MailWorkspace};

#[cfg(any(test, feature = "test-support"))]
use super::{mail_preview_label, mail_subject_label, MailContactHistoryRow};
#[cfg(any(test, feature = "test-support"))]
use crate::ui::format_mail_received_at;
#[cfg(any(test, feature = "test-support"))]
use std::collections::HashSet;

#[cfg(any(test, feature = "test-support"))]
const MAIL_CONTACT_HISTORY_LIMIT: usize = 6;

#[cfg(any(test, feature = "test-support"))]
pub(crate) fn mail_contact_history_rows(
    workspace: &MailWorkspace,
    contact_email: &str,
    selected_thread_id: &str,
) -> Vec<MailContactHistoryRow> {
    let contact_email = contact_email.trim().to_ascii_lowercase();
    if contact_email.is_empty() {
        return Vec::new();
    }
    let mut seen_thread_ids = HashSet::new();
    workspace
        .messages
        .iter()
        .filter(|message| mail_message_involves_contact(message, contact_email.as_str()))
        .filter(|message| seen_thread_ids.insert(message.thread_id.clone()))
        .map(|message| MailContactHistoryRow {
            thread_id: message.thread_id.clone(),
            subject: mail_subject_label(message.subject.as_str()),
            preview: mail_preview_label(message),
            timestamp: format_mail_received_at(&message.received_at),
            selected: message.thread_id == selected_thread_id,
        })
        .take(MAIL_CONTACT_HISTORY_LIMIT)
        .collect()
}

pub(crate) fn mail_message_involves_contact(message: &MailMessage, contact_email: &str) -> bool {
    message
        .sender
        .iter()
        .chain(message.from.iter())
        .chain(message.reply_to.iter())
        .chain(message.to.iter())
        .chain(message.cc.iter())
        .chain(message.bcc.iter())
        .any(|address| mail_address_matches_contact(address, contact_email))
}

pub(crate) fn mail_correspondent_address<'a>(
    workspace: &MailWorkspace,
    message: &'a MailMessage,
) -> Option<(&'a MailAddress, MailContactAddress)> {
    let owner = workspace
        .mailbox_email
        .as_deref()
        .and_then(|email| MailContactAddress::parse(email).ok());
    let from_owner = owner.as_ref().is_some_and(|owner| {
        message
            .from
            .iter()
            .any(|address| MailContactAddress::parse(address.email.as_str()).as_ref() == Ok(owner))
    });
    let outgoing = message.is_draft
        || from_owner
        || message.mailbox_ids.iter().any(|mailbox_id| {
            workspace
                .mailboxes
                .iter()
                .find(|mailbox| mailbox.id == *mailbox_id)
                .is_some_and(|mailbox| matches!(mailbox.role.as_deref(), Some("drafts" | "sent")))
        });
    if outgoing {
        first_external_mail_contact(
            message
                .to
                .iter()
                .chain(message.cc.iter())
                .chain(message.bcc.iter()),
            owner.as_ref(),
        )
    } else {
        first_external_mail_contact(message.from.iter(), owner.as_ref())
    }
}

fn first_external_mail_contact<'a>(
    mut addresses: impl Iterator<Item = &'a MailAddress>,
    owner: Option<&MailContactAddress>,
) -> Option<(&'a MailAddress, MailContactAddress)> {
    addresses.find_map(|address| {
        let contact = MailContactAddress::parse(address.email.as_str()).ok()?;
        (owner != Some(&contact)).then_some((address, contact))
    })
}

fn mail_address_matches_contact(address: &MailAddress, contact_email: &str) -> bool {
    address.email.eq_ignore_ascii_case(contact_email)
}
