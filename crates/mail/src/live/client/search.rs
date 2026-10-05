use std::collections::HashMap;

use crate::model::{MailSearchPage, MailSearchQuery, MailSearchSnippet};
use serde_json::{json, Value};
use time::OffsetDateTime;

use super::{mail_capabilities, MailLiveClient, MailLiveContext};
use crate::live::{
    helpers::{decode_account_jmap_response, into_mail_message_summary},
    types::{
        JmapEmail, JmapEmailGetResponse, JmapEmailQueryResponse, JmapResponse,
        JmapSearchSnippetGetResponse, JmapThreadGetResponse,
    },
};

mod filter;
mod request;
mod snippet;

use filter::{
    constant_search_truth, expression_uses_mailbox, resolve_search_filter, ResolvedSearchFilter,
};
use request::{
    email_get_call_with_ids, email_query_call, search_mail_messages_payload,
    search_snippet_get_call_with_ids, thread_get_call_with_ids,
};
use snippet::parse_snippet_text;

impl MailLiveClient {
    pub fn search_mail_messages(
        &self,
        query: &MailSearchQuery,
        position: usize,
    ) -> Result<MailSearchPage, String> {
        let context = self.context();
        let constant_truth = constant_search_truth(query.expression());
        if constant_truth == Some(false) {
            return Ok(empty_search_page(position));
        }
        let mailboxes =
            if constant_truth != Some(true) && expression_uses_mailbox(query.expression()) {
                self.load_mailboxes(&context.api_url, context.account.id.as_str())?
                    .list
            } else {
                Vec::new()
            };
        let filter = resolve_search_filter(
            query.expression(),
            mailboxes.as_slice(),
            OffsetDateTime::now_utc(),
        )?;
        if filter == ResolvedSearchFilter::Never {
            return Ok(empty_search_page(position));
        }
        let filter = filter.into_jmap_filter();
        let page_limit = super::MAIL_PAGE_SIZE.min(context.limits.max_objects_in_get.get());
        if context.limits.max_calls_in_request.get() < 4 {
            return self.search_mail_messages_sequentially(context, position, page_limit, filter);
        }
        let response = self.jmap_request(
            &context.api_url,
            search_mail_messages_payload(
                context.account.id.as_str(),
                position,
                page_limit,
                filter.clone(),
            ),
        )?;
        decode_search_page(&response, context.account.id.as_str(), position, page_limit)
    }

    fn search_mail_messages_sequentially(
        &self,
        context: &MailLiveContext,
        position: usize,
        page_limit: usize,
        filter: Value,
    ) -> Result<MailSearchPage, String> {
        let account_id = context.account.id.as_str();
        let query_response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [email_query_call(account_id, position, page_limit, &filter)]
            }),
        )?;
        let search = decode_search_query(&query_response, account_id, position)?;
        let next_position = (search.ids.len() == page_limit).then_some(position + search.ids.len());
        if search.ids.is_empty() {
            return Ok(MailSearchPage {
                position,
                messages: Vec::new(),
                next_position,
                snippets: HashMap::new(),
                thread_message_counts: HashMap::new(),
            });
        }
        let emails = self.fetch_search_emails(context, search.ids.as_slice())?;
        let messages = order_search_messages(search.ids.as_slice(), emails)?;
        let snippets = self.fetch_search_snippets(context, search.ids.as_slice(), &filter)?;
        let thread_ids = messages
            .iter()
            .map(|message| message.thread_id.clone())
            .collect::<Vec<_>>();
        let thread_message_counts = self.fetch_search_thread_counts(context, &thread_ids)?;
        Ok(MailSearchPage {
            position,
            messages,
            next_position,
            snippets,
            thread_message_counts,
        })
    }

    fn fetch_search_emails(
        &self,
        context: &MailLiveContext,
        email_ids: &[String],
    ) -> Result<Vec<JmapEmail>, String> {
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [email_get_call_with_ids(
                    context.account.id.as_str(),
                    email_ids,
                )]
            }),
        )?;
        decode_account_jmap_response::<JmapEmailGetResponse>(
            &response,
            "emails",
            "Email/get",
            context.account.id.as_str(),
        )
        .map(|payload| payload.list)
    }

    fn fetch_search_snippets(
        &self,
        context: &MailLiveContext,
        email_ids: &[String],
        filter: &Value,
    ) -> Result<HashMap<String, MailSearchSnippet>, String> {
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [search_snippet_get_call_with_ids(
                    context.account.id.as_str(),
                    email_ids,
                    filter,
                )]
            }),
        )?;
        decode_search_snippets(&response, context.account.id.as_str())
    }

    fn fetch_search_thread_counts(
        &self,
        context: &MailLiveContext,
        thread_ids: &[String],
    ) -> Result<HashMap<String, usize>, String> {
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [thread_get_call_with_ids(
                    context.account.id.as_str(),
                    thread_ids,
                )]
            }),
        )?;
        decode_thread_message_counts(&response, context.account.id.as_str())
    }
}

fn decode_search_page(
    response: &JmapResponse,
    account_id: &str,
    position: usize,
    page_limit: usize,
) -> Result<MailSearchPage, String> {
    let search = decode_search_query(response, account_id, position)?;
    let next_position = (search.ids.len() == page_limit).then_some(position + search.ids.len());
    let emails = decode_account_jmap_response::<JmapEmailGetResponse>(
        response,
        "emails",
        "Email/get",
        account_id,
    )?;
    let messages = order_search_messages(search.ids.as_slice(), emails.list)?;
    let thread_message_counts = decode_thread_message_counts(response, account_id)?;
    Ok(MailSearchPage {
        position,
        messages,
        next_position,
        snippets: decode_search_snippets(response, account_id)?,
        thread_message_counts,
    })
}

fn decode_search_query(
    response: &JmapResponse,
    account_id: &str,
    position: usize,
) -> Result<JmapEmailQueryResponse, String> {
    let search = decode_account_jmap_response::<JmapEmailQueryResponse>(
        response,
        "search",
        "Email/query",
        account_id,
    )?;
    if search.position != position {
        return Err(format!(
            "JMAP returned search page position {} for requested position {position}",
            search.position
        ));
    }
    Ok(search)
}

fn decode_thread_message_counts(
    response: &JmapResponse,
    account_id: &str,
) -> Result<HashMap<String, usize>, String> {
    decode_account_jmap_response::<JmapThreadGetResponse>(
        response,
        "threads",
        "Thread/get",
        account_id,
    )
    .map(|payload| {
        payload
            .list
            .into_iter()
            .map(|thread| (thread.id, thread.email_ids.len()))
            .collect()
    })
}

fn decode_search_snippets(
    response: &JmapResponse,
    account_id: &str,
) -> Result<HashMap<String, MailSearchSnippet>, String> {
    decode_account_jmap_response::<JmapSearchSnippetGetResponse>(
        response,
        "snippets",
        "SearchSnippet/get",
        account_id,
    )?
    .list
    .into_iter()
    .map(|snippet| {
        let subject = snippet
            .subject
            .as_deref()
            .map(parse_snippet_text)
            .transpose()?;
        let preview = snippet
            .preview
            .as_deref()
            .map(parse_snippet_text)
            .transpose()?;
        Ok((snippet.email_id, MailSearchSnippet::new(subject, preview)))
    })
    .collect()
}

fn empty_search_page(position: usize) -> MailSearchPage {
    MailSearchPage {
        position,
        messages: Vec::new(),
        next_position: None,
        snippets: HashMap::new(),
        thread_message_counts: HashMap::new(),
    }
}

fn order_search_messages(
    search_ids: &[String],
    emails: Vec<JmapEmail>,
) -> Result<Vec<crate::model::MailMessage>, String> {
    let mut emails_by_id = emails
        .into_iter()
        .map(|email| (email.id.clone(), email))
        .collect::<HashMap<_, _>>();
    search_ids
        .iter()
        .map(|email_id| {
            emails_by_id
                .remove(email_id)
                .map(into_mail_message_summary)
                .ok_or_else(|| format!("Email/get omitted search result {email_id}"))
        })
        .collect()
}
