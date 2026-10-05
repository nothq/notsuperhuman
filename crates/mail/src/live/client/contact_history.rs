use std::collections::{HashMap, HashSet};

use serde_json::json;

use crate::{
    live::{
        helpers::decode_account_jmap_response,
        types::{JmapEmailQueryResponse, JmapThread, JmapThreadGetResponse},
    },
    model::{
        MailContactAddress, MailContactHistoryEntry, MailContactHistoryPage,
        MailContactHistoryRequest, MailMessage, MAIL_CONTACT_HISTORY_LIMIT,
    },
};

use super::{mail_capabilities, MailLiveClient, MailLiveContext};

const MAIL_CONTACT_CANDIDATE_PAGE_SIZE: usize = 24;

struct ContactHistoryCandidate {
    representative: MailMessage,
    thread: JmapThread,
}

impl MailLiveClient {
    pub fn load_mail_contact_history(
        &self,
        request: MailContactHistoryRequest,
    ) -> Result<MailContactHistoryPage, String> {
        let context = self.context();
        let contact = request.contact().clone();
        let page_size =
            MAIL_CONTACT_CANDIDATE_PAGE_SIZE.min(context.limits.max_objects_in_get.get());
        let mut position = 0usize;
        let mut query_state = None;
        let mut seen_email_ids = HashSet::new();
        let mut seen_thread_ids = HashSet::new();
        let mut entries = Vec::with_capacity(MAIL_CONTACT_HISTORY_LIMIT);

        loop {
            let query =
                self.query_mail_contact_candidates(context, &contact, position, page_size)?;
            validate_contact_query_page(
                &query,
                position,
                page_size,
                query_state.as_deref(),
                &mut seen_email_ids,
            )?;
            query_state = Some(query.query_state.clone());
            if query.ids.is_empty() {
                break;
            }

            let candidates = self.load_contact_history_candidate_page(
                context,
                query.ids.clone(),
                &mut seen_thread_ids,
            )?;
            let messages = self.load_contact_history_thread_messages(context, &candidates)?;

            for candidate in candidates {
                if let Some(entry) = contact_history_entry(candidate, &messages, &contact)? {
                    entries.push(entry);
                    if entries.len() == MAIL_CONTACT_HISTORY_LIMIT {
                        return MailContactHistoryPage::new(contact, entries);
                    }
                }
            }

            if query.ids.len() < page_size {
                break;
            }
            let next_position = position
                .checked_add(query.ids.len())
                .ok_or_else(|| "mail contact history position overflowed".to_string())?;
            if next_position <= position {
                return Err("mail contact history query did not advance".to_string());
            }
            position = next_position;
        }

        MailContactHistoryPage::new(contact, entries)
    }

    fn load_contact_history_candidate_page(
        &self,
        context: &MailLiveContext,
        email_ids: Vec<String>,
        seen_thread_ids: &mut HashSet<String>,
    ) -> Result<Vec<ContactHistoryCandidate>, String> {
        let representatives = self.fetch_mail_message_summaries(
            &context.api_url,
            context.account.id.as_str(),
            email_ids,
        )?;
        let representatives = representatives
            .into_iter()
            .filter(|message| seen_thread_ids.insert(message.thread_id.clone()))
            .collect::<Vec<_>>();
        self.load_contact_history_candidates(context, representatives)
    }

    fn query_mail_contact_candidates(
        &self,
        context: &MailLiveContext,
        contact: &MailContactAddress,
        position: usize,
        limit: usize,
    ) -> Result<JmapEmailQueryResponse, String> {
        let response = self.jmap_request(
            &context.api_url,
            json!({
                "using": mail_capabilities(),
                "methodCalls": [[
                    "Email/query",
                    {
                        "accountId": context.account.id.as_str(),
                        "filter": {
                            "operator": "OR",
                            "conditions": [
                                { "from": contact.as_str() },
                                { "to": contact.as_str() },
                                { "cc": contact.as_str() },
                                { "bcc": contact.as_str() }
                            ]
                        },
                        "sort": [{ "property": "receivedAt", "isAscending": false }],
                        "collapseThreads": true,
                        "position": position,
                        "limit": limit
                    },
                    "contact-history-query"
                ]]
            }),
        )?;
        decode_account_jmap_response::<JmapEmailQueryResponse>(
            &response,
            "contact-history-query",
            "Email/query",
            context.account.id.as_str(),
        )
    }

    fn load_contact_history_candidates(
        &self,
        context: &MailLiveContext,
        representatives: Vec<MailMessage>,
    ) -> Result<Vec<ContactHistoryCandidate>, String> {
        let thread_ids = representatives
            .iter()
            .map(|message| message.thread_id.clone())
            .collect::<Vec<_>>();
        let threads = self.fetch_contact_history_threads(context, thread_ids.as_slice())?;
        representatives
            .into_iter()
            .zip(threads)
            .map(|(representative, thread)| {
                if representative.thread_id != thread.id {
                    return Err(format!(
                        "Thread/get returned thread {} for contact history representative {} in {}",
                        thread.id, representative.id, representative.thread_id
                    ));
                }
                if !thread
                    .email_ids
                    .iter()
                    .any(|email_id| email_id == &representative.id)
                {
                    return Err(format!(
                        "Thread/get omitted representative {} from contact history thread {}",
                        representative.id, thread.id
                    ));
                }
                Ok(ContactHistoryCandidate {
                    representative,
                    thread,
                })
            })
            .collect()
    }

    fn fetch_contact_history_threads(
        &self,
        context: &MailLiveContext,
        thread_ids: &[String],
    ) -> Result<Vec<JmapThread>, String> {
        let mut threads_by_id = HashMap::with_capacity(thread_ids.len());
        for chunk in thread_ids.chunks(context.limits.max_objects_in_get.get()) {
            let response = self.jmap_request(
                &context.api_url,
                json!({
                    "using": mail_capabilities(),
                    "methodCalls": [[
                        "Thread/get",
                        {
                            "accountId": context.account.id.as_str(),
                            "ids": chunk,
                            "properties": ["id", "emailIds"]
                        },
                        "contact-history-threads"
                    ]]
                }),
            )?;
            for thread in decode_account_jmap_response::<JmapThreadGetResponse>(
                &response,
                "contact-history-threads",
                "Thread/get",
                context.account.id.as_str(),
            )?
            .list
            {
                if !thread_ids.iter().any(|thread_id| thread_id == &thread.id) {
                    return Err(format!(
                        "Thread/get returned unexpected contact history thread {}",
                        thread.id
                    ));
                }
                let thread_id = thread.id.clone();
                if threads_by_id.insert(thread_id.clone(), thread).is_some() {
                    return Err(format!(
                        "Thread/get returned duplicate contact history thread {thread_id}"
                    ));
                }
            }
        }
        thread_ids
            .iter()
            .map(|thread_id| {
                threads_by_id
                    .remove(thread_id)
                    .ok_or_else(|| format!("Thread/get omitted contact history thread {thread_id}"))
            })
            .collect()
    }

    fn load_contact_history_thread_messages(
        &self,
        context: &MailLiveContext,
        candidates: &[ContactHistoryCandidate],
    ) -> Result<HashMap<String, MailMessage>, String> {
        let mut seen_email_ids = HashSet::new();
        let email_ids = candidates
            .iter()
            .flat_map(|candidate| candidate.thread.email_ids.iter())
            .filter(|email_id| seen_email_ids.insert((*email_id).clone()))
            .cloned()
            .collect::<Vec<_>>();
        self.fetch_mail_message_summaries(&context.api_url, context.account.id.as_str(), email_ids)
            .map(|messages| {
                messages
                    .into_iter()
                    .map(|message| (message.id.clone(), message))
                    .collect()
            })
    }
}

fn contact_history_entry(
    candidate: ContactHistoryCandidate,
    messages: &HashMap<String, MailMessage>,
    contact: &MailContactAddress,
) -> Result<Option<MailContactHistoryEntry>, String> {
    let mut exact_match = false;
    for email_id in &candidate.thread.email_ids {
        let message = messages
            .get(email_id)
            .ok_or_else(|| format!("Email/get omitted contact history email {email_id}"))?;
        if message.thread_id != candidate.thread.id {
            return Err(format!(
                "Email/get returned contact history email {email_id} for thread {} instead of {}",
                message.thread_id, candidate.thread.id
            ));
        }
        if mail_message_exactly_matches_contact(message, contact) {
            exact_match = true;
        }
    }
    if !exact_match {
        return Ok(None);
    }
    Ok(Some(MailContactHistoryEntry {
        thread_id: candidate.thread.id,
        email_ids: candidate.thread.email_ids,
        subject: candidate.representative.subject,
        preview: candidate.representative.preview,
        received_at: candidate.representative.received_at,
    }))
}

fn validate_contact_query_page(
    query: &JmapEmailQueryResponse,
    requested_position: usize,
    requested_limit: usize,
    expected_query_state: Option<&str>,
    seen_email_ids: &mut HashSet<String>,
) -> Result<(), String> {
    if query.position != requested_position {
        return Err(format!(
            "JMAP returned contact history position {} for requested position {requested_position}",
            query.position
        ));
    }
    if query.query_state.is_empty() {
        return Err("JMAP returned an empty contact history query state".to_string());
    }
    if expected_query_state.is_some_and(|state| state != query.query_state) {
        return Err("mail contact history query changed while paging".to_string());
    }
    if query.ids.len() > requested_limit {
        return Err(format!(
            "JMAP returned {} contact history candidates for requested limit {requested_limit}",
            query.ids.len()
        ));
    }
    for email_id in &query.ids {
        if email_id.is_empty() {
            return Err("JMAP returned an empty contact history email ID".to_string());
        }
        if !seen_email_ids.insert(email_id.clone()) {
            return Err(format!(
                "JMAP repeated contact history email {email_id} across query positions"
            ));
        }
    }
    Ok(())
}

fn mail_message_exactly_matches_contact(
    message: &MailMessage,
    contact: &MailContactAddress,
) -> bool {
    message
        .from
        .iter()
        .chain(message.to.iter())
        .chain(message.cc.iter())
        .chain(message.bcc.iter())
        .filter_map(|address| MailContactAddress::parse(address.email.as_str()).ok())
        .any(|address| &address == contact)
}
