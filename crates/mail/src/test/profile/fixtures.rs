use crate::model::{MailMessage, MailWorkspace, Mailbox, MailboxRights};

pub(super) fn test_workspace(mailbox_id: &str) -> MailWorkspace {
    MailWorkspace {
        account_id: "account".to_string(),
        mailbox_email: Some("notsuperhuman@example.test".to_string()),
        display_name: "notsuperhuman".to_string(),
        owner_username: "notsuperhuman".to_string(),
        selected_mailbox_id: mailbox_id.to_string(),
        mailboxes: vec![
            test_mailbox("inbox", "Inbox"),
            test_mailbox("drafts", "Drafts"),
            test_mailbox("sent", "Sent"),
        ],
        messages: vec![test_message(mailbox_id)],
        message_next_position: None,
    }
}

fn test_mailbox(id: &str, name: &str) -> Mailbox {
    Mailbox {
        id: id.to_string(),
        name: name.to_string(),
        role: Some(id.to_string()),
        unread_emails: 0,
        total_emails: 1,
        rights: MailboxRights {
            may_read_items: true,
            may_add_items: true,
            may_remove_items: true,
            may_set_seen: true,
            may_set_keywords: true,
            may_create_child: true,
            may_rename: true,
            may_delete: true,
            may_submit: true,
        },
    }
}

fn test_message(mailbox_id: &str) -> MailMessage {
    MailMessage {
        id: format!("{mailbox_id}-message"),
        thread_id: format!("{mailbox_id}-thread"),
        subject: format!("{mailbox_id} subject"),
        preview: String::new(),
        received_at: "2026-05-19T00:00:00Z".to_string(),
        from: Vec::new(),
        to: Vec::new(),
        mailbox_ids: vec![mailbox_id.to_string()],
        has_attachment: false,
        is_unread: false,
        is_starred: false,
        body_text: String::new(),
        attachments: Vec::new(),
        ..Default::default()
    }
}
