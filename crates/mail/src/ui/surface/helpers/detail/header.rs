use super::super::super::{MailMessageHeaderAddress, MailMessageHeaderDetails};
use super::super::{MailMessage, MailWorkspace};
use super::{mail_address_summary, mail_primary_sender_label};
use crate::model::MailAddress;
use crate::ui::mail_local_timestamp;
use time::macros::format_description;

pub(super) fn mail_message_is_received(workspace: &MailWorkspace, message: &MailMessage) -> bool {
    if message.is_draft
        || mail_message_has_mailbox_role(workspace, message, "drafts")
        || mail_message_has_mailbox_role(workspace, message, "sent")
    {
        return false;
    }
    if mail_message_has_mailbox_role(workspace, message, "inbox") {
        return true;
    }
    let Some(owner) = workspace.mailbox_email.as_deref() else {
        return false;
    };
    !message
        .sender
        .iter()
        .chain(message.from.iter())
        .any(|address| address.email.eq_ignore_ascii_case(owner))
}

fn mail_message_has_mailbox_role(
    workspace: &MailWorkspace,
    message: &MailMessage,
    role: &str,
) -> bool {
    message.mailbox_ids.iter().any(|mailbox_id| {
        workspace
            .mailboxes
            .iter()
            .find(|mailbox| mailbox.id == *mailbox_id)
            .is_some_and(|mailbox| mailbox.role.as_deref() == Some(role))
    })
}

pub(super) fn mail_message_header_details(
    workspace: &MailWorkspace,
    message: &MailMessage,
) -> MailMessageHeaderDetails {
    MailMessageHeaderDetails {
        compact_label: mail_message_compact_header_label(workspace, message).into(),
        from: mail_message_header_addresses(&message.from),
        to: mail_message_header_addresses(&message.to),
        cc: mail_message_header_addresses(&message.cc),
        bcc: mail_message_header_addresses(&message.bcc),
        full_timestamp: mail_message_full_timestamp(message).into(),
    }
}

fn mail_message_header_addresses(
    addresses: &[MailAddress],
) -> std::sync::Arc<[MailMessageHeaderAddress]> {
    addresses
        .iter()
        .map(|address| {
            let has_name = !address.name.trim().is_empty();
            MailMessageHeaderAddress {
                name: if has_name {
                    address.name.clone().into()
                } else {
                    address.email.clone().into()
                },
                email: has_name.then(|| format!(" <{}>", address.email).into()),
            }
        })
        .collect::<Vec<_>>()
        .into()
}

fn mail_message_compact_header_label(workspace: &MailWorkspace, message: &MailMessage) -> String {
    let recipients = message
        .to
        .iter()
        .chain(message.cc.iter())
        .map(|address| mail_compact_recipient_label(workspace, address))
        .collect::<Vec<_>>();
    let recipients = if recipients.is_empty() {
        "Me".to_string()
    } else {
        natural_language_list(&recipients)
    };
    format!("{} to {recipients}", mail_primary_sender_label(message))
}

fn mail_compact_recipient_label(workspace: &MailWorkspace, address: &MailAddress) -> String {
    if workspace
        .mailbox_email
        .as_deref()
        .is_some_and(|mailbox| mailbox.eq_ignore_ascii_case(&address.email))
    {
        "Me".to_string()
    } else {
        mail_address_summary(address)
    }
}

fn natural_language_list(labels: &[String]) -> String {
    match labels {
        [] => String::new(),
        [label] => label.clone(),
        [first, second] => format!("{first} & {second}"),
        _ => {
            let (last, leading) = labels.split_last().expect("non-empty recipient labels");
            format!("{} & {last}", leading.join(", "))
        }
    }
}

fn mail_message_full_timestamp(message: &MailMessage) -> String {
    let Some(timestamp) = mail_local_timestamp(&message.received_at) else {
        return message.received_at.clone();
    };
    timestamp
        .format(&format_description!(
            "[weekday repr:long], [month repr:long] [day padding:none] [year] at [hour repr:12]:[minute] [period case:upper]"
        ))
        .unwrap_or_else(|_| message.received_at.clone())
}
