use std::collections::HashMap;

use reqwest::Url;
use serde_json::json;

use crate::{
    live::{
        helpers::{
            decode_account_jmap_response, into_mail_message, into_mail_message_summary,
            into_mail_thread, sanitize_cached_mail_summary_messages,
        },
        types::{
            JmapEmail, JmapEmailGetResponse, JmapEmailQueryResponse, JmapThread,
            JmapThreadGetResponse, MailboxMessagesLoad,
        },
    },
    model::{MailMessage, MailMessagePage, MailThread},
};

use super::super::{mail_capabilities, MailLiveClient, MailLiveContext, MailboxMessagesRequest};

impl MailLiveClient {
    pub(crate) fn load_mailbox_message_summaries(
        &self,
        request: MailboxMessagesRequest<'_>,
    ) -> Result<MailboxMessagesLoad, String> {
        let query = self.query_mailbox_messages(&request)?;
        if query.position != request.position {
            return Err(format!(
                "JMAP returned mailbox page position {} for requested position {}",
                query.position, request.position
            ));
        }
        let position = query.position;
        if query.ids.is_empty() {
            return Ok(MailboxMessagesLoad {
                mailbox_id: request.mailbox_id.to_string(),
                position,
                query_state: query.query_state,
                messages: Vec::new(),
                next_position: None,
            });
        }
        let next_position = next_mail_message_position(position, query.ids.len());
        if request.position == 0 && request.cached_query_state == Some(query.query_state.as_str()) {
            if let Some(cached_messages) = request.cached_messages {
                return Ok(MailboxMessagesLoad {
                    mailbox_id: request.mailbox_id.to_string(),
                    position,
                    query_state: query.query_state,
                    messages: sanitize_cached_mail_summary_messages(cached_messages.to_vec()),
                    next_position,
                });
            }
        }
        let messages =
            self.fetch_mail_message_summaries(request.api_url, request.account_id, query.ids)?;
        Ok(MailboxMessagesLoad {
            mailbox_id: request.mailbox_id.to_string(),
            position,
            query_state: query.query_state,
            messages,
            next_position,
        })
    }

    pub(crate) fn query_mailbox_messages(
        &self,
        request: &MailboxMessagesRequest<'_>,
    ) -> Result<JmapEmailQueryResponse, String> {
        let response = self.jmap_request(
            request.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [[
                    "Email/query",
                    {
                        "accountId": request.account_id,
                        "filter": { "inMailbox": request.mailbox_id },
                        "sort": [{ "property": "receivedAt", "isAscending": false }],
                        "collapseThreads": true,
                        "position": request.position,
                        "limit": super::super::MAIL_PAGE_SIZE
                    },
                    "query"
                ]]
            }),
        )?;
        decode_account_jmap_response::<JmapEmailQueryResponse>(
            &response,
            "query",
            "Email/query",
            request.account_id,
        )
    }

    pub fn load_mail_message_page(
        &self,
        mailbox_id: &str,
        position: usize,
    ) -> Result<MailMessagePage, String> {
        let context = self.context();
        let load = self.load_mailbox_message_summaries(MailboxMessagesRequest {
            api_url: &context.api_url,
            account_id: context.account.id.as_str(),
            mailbox_id,
            position,
            cached_query_state: None,
            cached_messages: None,
        })?;
        Ok(MailMessagePage {
            mailbox_id: load.mailbox_id,
            position: load.position,
            messages: load.messages,
            next_position: load.next_position,
        })
    }

    pub fn fetch_mail_message_summaries(
        &self,
        api_url: &Url,
        account_id: &str,
        ids: Vec<String>,
    ) -> Result<Vec<MailMessage>, String> {
        let mut emails = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(self.context().limits.max_objects_in_get.get()) {
            let response = self.jmap_request(
                api_url,
                json!({
                    "using": mail_capabilities(),
                    "methodCalls": [[
                        "Email/get",
                        {
                            "accountId": account_id,
                            "ids": chunk,
                            "properties": [
                                "id", "threadId", "messageId", "keywords", "mailboxIds",
                                "receivedAt", "sentAt", "sender", "replyTo", "from", "to",
                                "cc", "bcc", "subject", "preview", "hasAttachment"
                            ]
                        },
                        "emails"
                    ]]
                }),
            )?;
            emails.extend(
                decode_account_jmap_response::<JmapEmailGetResponse>(
                    &response,
                    "emails",
                    "Email/get",
                    account_id,
                )?
                .list,
            );
        }
        order_jmap_emails(ids.as_slice(), emails, "summary")
            .map(|emails| emails.into_iter().map(into_mail_message_summary).collect())
    }

    pub fn load_mail_message_detail(
        &self,
        _mailbox_id: &str,
        message_id: &str,
    ) -> Result<MailMessage, String> {
        let context = self.context();
        self.load_mail_message_detail_with_context(context, message_id)
    }

    pub(crate) fn load_mail_message_detail_with_context(
        &self,
        context: &MailLiveContext,
        message_id: &str,
    ) -> Result<MailMessage, String> {
        if let Some(message) = self.cached_message_detail(message_id)? {
            return Ok(message);
        }
        let message = self
            .fetch_mail_message_details(
                &context.api_url,
                context.account.id.as_str(),
                vec![message_id.to_string()],
            )?
            .into_iter()
            .next()
            .ok_or_else(|| format!("missing JMAP message detail {message_id}"))?;
        self.store_message_detail(message.clone())?;
        Ok(message)
    }

    pub fn fetch_mail_message_details(
        &self,
        api_url: &Url,
        account_id: &str,
        ids: Vec<String>,
    ) -> Result<Vec<MailMessage>, String> {
        let mut emails = Vec::with_capacity(ids.len());
        for chunk in ids.chunks(self.context().limits.max_objects_in_get.get()) {
            let response = self.jmap_request(
                api_url,
                super::mail_message_detail_get_payload(account_id, chunk.to_vec()),
            )?;
            emails.extend(
                decode_account_jmap_response::<JmapEmailGetResponse>(
                    &response,
                    "emails",
                    "Email/get",
                    account_id,
                )?
                .list,
            );
        }
        order_jmap_emails(ids.as_slice(), emails, "detail")
            .map(|emails| emails.into_iter().map(into_mail_message).collect())
    }

    pub fn load_mail_thread(&self, thread_id: &str) -> Result<MailThread, String> {
        let context = self.context();
        let thread = self.load_jmap_thread(context, thread_id)?;
        let messages = self.fetch_mail_message_details(
            &context.api_url,
            context.account.id.as_str(),
            thread.email_ids.clone(),
        )?;
        for message in &messages {
            self.store_message_detail(message.clone())?;
        }
        Ok(into_mail_thread(thread, messages))
    }

    pub(crate) fn load_jmap_thread(
        &self,
        context: &MailLiveContext,
        thread_id: &str,
    ) -> Result<JmapThread, String> {
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [[
                    "Thread/get",
                    { "accountId": context.account.id.as_str(), "ids": [thread_id] },
                    "thread"
                ]]
            }),
        )?;
        decode_account_jmap_response::<JmapThreadGetResponse>(
            &response,
            "thread",
            "Thread/get",
            context.account.id.as_str(),
        )?
        .list
        .into_iter()
        .next()
        .ok_or_else(|| format!("missing JMAP thread detail {thread_id}"))
    }
}

fn next_mail_message_position(position: usize, loaded_count: usize) -> Option<usize> {
    (loaded_count == super::super::MAIL_PAGE_SIZE).then_some(position + loaded_count)
}

fn order_jmap_emails(
    requested_ids: &[String],
    emails: Vec<JmapEmail>,
    description: &str,
) -> Result<Vec<JmapEmail>, String> {
    let mut emails_by_id = HashMap::with_capacity(emails.len());
    for email in emails {
        if !requested_ids
            .iter()
            .any(|requested_id| requested_id == &email.id)
        {
            return Err(format!(
                "Email/get returned unexpected {description} email {}",
                email.id
            ));
        }
        let email_id = email.id.clone();
        if emails_by_id.insert(email_id.clone(), email).is_some() {
            return Err(format!(
                "Email/get returned duplicate {description} email {email_id}"
            ));
        }
    }
    requested_ids
        .iter()
        .map(|email_id| {
            emails_by_id.remove(email_id).ok_or_else(|| {
                format!("Email/get omitted requested {description} email {email_id}")
            })
        })
        .collect()
}
