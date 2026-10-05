use crate::model::{MailIdentity, MailMessage, MailWorkspace};
use serde_json::json;

use super::{mail_capabilities, mail_submission_capabilities, MailLiveClient};
use crate::live::{
    helpers::{
        decode_account_jmap_response, find_initial_mailbox, into_mail_identity, into_mailbox,
        sort_jmap_mailboxes,
    },
    types::{JmapIdentityGetResponse, MailWorkspaceSummaryLoad},
};

mod messages;

impl MailLiveClient {
    pub fn load_workspace(&self) -> Result<MailWorkspace, String> {
        self.refresh_workspace_summary(None)
    }

    pub fn load_mailbox_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.refresh_workspace_summary(Some(mailbox_id))
    }

    pub fn refresh_mailbox_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.refresh_workspace_summary(Some(mailbox_id))
    }

    pub fn refresh_workspace_summary(
        &self,
        selected_mailbox_id: Option<&str>,
    ) -> Result<MailWorkspace, String> {
        self.load_workspace_summary(selected_mailbox_id, None, None)
            .map(|load| load.workspace)
    }

    pub fn load_workspace_summary(
        &self,
        selected_mailbox_id: Option<&str>,
        cached_query_state: Option<&str>,
        cached_messages: Option<&[MailMessage]>,
    ) -> Result<MailWorkspaceSummaryLoad, String> {
        let context = self.context();
        let mailboxes_response =
            self.load_mailboxes(&context.api_url, context.account.id.as_str())?;
        let mailbox_state = mailboxes_response.state;
        let mut mailboxes = mailboxes_response.list;
        sort_jmap_mailboxes(&mut mailboxes);
        let selected_mailbox = selected_mailbox_id
            .and_then(|mailbox_id| mailboxes.iter().find(|mailbox| mailbox.id == mailbox_id))
            .map_or_else(|| find_initial_mailbox(&mailboxes), Ok)?;
        let cache_matches_selected_mailbox =
            selected_mailbox_id == Some(selected_mailbox.id.as_str());
        let (cached_query_state, cached_messages) = if cache_matches_selected_mailbox {
            (cached_query_state, cached_messages)
        } else {
            (None, None)
        };
        let selected_mailbox_id = selected_mailbox.id.clone();
        let loaded_messages =
            self.load_mailbox_message_summaries(super::MailboxMessagesRequest {
                api_url: &context.api_url,
                account_id: context.account.id.as_str(),
                mailbox_id: selected_mailbox.id.as_str(),
                position: 0,
                cached_query_state,
                cached_messages,
            })?;
        Ok(MailWorkspaceSummaryLoad {
            workspace: MailWorkspace {
                account_id: context.account.id.clone(),
                mailbox_email: context.account.address.clone(),
                display_name: context.account.name.clone(),
                owner_username: context.owner_username.clone(),
                selected_mailbox_id,
                mailboxes: mailboxes.into_iter().map(into_mailbox).collect(),
                messages: loaded_messages.messages,
                message_next_position: loaded_messages.next_position,
            },
            mailbox_state,
            query_state: loaded_messages.query_state,
        })
    }

    pub fn load_mail_identity(&self) -> Result<MailIdentity, String> {
        let context = self.context();
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_submission_capabilities(),
                "methodCalls": [[
                    "Identity/get",
                    { "accountId": context.account.id },
                    "identity"
                ]]
            }),
        )?;
        let identities = decode_account_jmap_response::<JmapIdentityGetResponse>(
            &response,
            "identity",
            "Identity/get",
            context.account.id.as_str(),
        )?;
        identities
            .list
            .into_iter()
            .map(into_mail_identity)
            .next()
            .ok_or_else(|| "JMAP account does not expose a send identity".to_string())
    }
}

fn mail_message_detail_get_payload(account_id: &str, ids: Vec<String>) -> serde_json::Value {
    json!({
        "using": mail_capabilities(),
        "methodCalls": [[
            "Email/get",
            {
                "accountId": account_id,
                "ids": ids,
                "properties": [
                    "id", "threadId", "messageId", "inReplyTo", "references",
                    "keywords", "mailboxIds", "receivedAt", "sentAt", "sender",
                    "replyTo", "from", "to", "cc", "bcc", "subject", "preview",
                    "textBody", "htmlBody", "bodyValues", "hasAttachment", "attachments"
                ],
                "fetchTextBodyValues": true,
                "fetchAllBodyValues": true,
                "maxBodyValueBytes": super::MAIL_BODY_MAX_BYTES
            },
            "emails"
        ]]
    })
}

#[cfg(test)]
mod tests {
    use super::mail_message_detail_get_payload;

    #[test]
    fn detail_request_allows_large_rich_html_bodies() {
        let payload = mail_message_detail_get_payload("account", vec!["email".to_string()]);
        let max_body_value_bytes = payload
            .pointer("/methodCalls/0/1/maxBodyValueBytes")
            .and_then(serde_json::Value::as_u64)
            .expect("max body value bytes");

        assert!(
            max_body_value_bytes >= 512 * 1024,
            "rich marketing HTML mail bodies must not be truncated to plain text"
        );
    }
}
