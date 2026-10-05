use serde_json::{json, Value};

use super::super::mail_capabilities;

pub(super) fn search_mail_messages_payload(
    account_id: &str,
    position: usize,
    page_limit: usize,
    filter: Value,
) -> Value {
    json!({
        "using": mail_capabilities(),
        "methodCalls": [
            email_query_call(account_id, position, page_limit, &filter),
            email_get_call(account_id),
            search_snippet_get_call(account_id, &filter),
            thread_get_call(account_id)
        ]
    })
}

pub(super) fn email_query_call(
    account_id: &str,
    position: usize,
    page_limit: usize,
    filter: &Value,
) -> Value {
    json!([
        "Email/query",
        {
            "accountId": account_id,
            "filter": filter,
            "sort": [{ "property": "receivedAt", "isAscending": false }],
            "collapseThreads": true,
            "position": position,
            "limit": page_limit
        },
        "search"
    ])
}

pub(super) fn email_get_call_with_ids(account_id: &str, email_ids: &[String]) -> Value {
    json!([
        "Email/get",
        {
            "accountId": account_id,
            "ids": email_ids,
            "properties": [
                "id", "threadId", "messageId", "keywords", "mailboxIds",
                "receivedAt", "sentAt", "sender", "replyTo", "from", "to",
                "cc", "bcc", "subject", "preview", "hasAttachment", "attachments"
            ]
        },
        "emails"
    ])
}

fn email_get_call(account_id: &str) -> Value {
    json!([
        "Email/get",
        {
            "accountId": account_id,
            "#ids": {
                "resultOf": "search",
                "name": "Email/query",
                "path": "/ids"
            },
            "properties": [
                "id", "threadId", "messageId", "keywords", "mailboxIds",
                "receivedAt", "sentAt", "sender", "replyTo", "from", "to",
                "cc", "bcc", "subject", "preview", "hasAttachment", "attachments"
            ]
        },
        "emails"
    ])
}

fn search_snippet_get_call(account_id: &str, filter: &Value) -> Value {
    json!([
        "SearchSnippet/get",
        {
            "accountId": account_id,
            "filter": filter,
            "#emailIds": {
                "resultOf": "search",
                "name": "Email/query",
                "path": "/ids"
            }
        },
        "snippets"
    ])
}

pub(super) fn search_snippet_get_call_with_ids(
    account_id: &str,
    email_ids: &[String],
    filter: &Value,
) -> Value {
    json!([
        "SearchSnippet/get",
        {
            "accountId": account_id,
            "filter": filter,
            "emailIds": email_ids
        },
        "snippets"
    ])
}

fn thread_get_call(account_id: &str) -> Value {
    json!([
        "Thread/get",
        {
            "accountId": account_id,
            "#ids": {
                "resultOf": "emails",
                "name": "Email/get",
                "path": "/list/*/threadId"
            },
            "properties": ["id", "emailIds"]
        },
        "threads"
    ])
}

pub(super) fn thread_get_call_with_ids(account_id: &str, thread_ids: &[String]) -> Value {
    json!([
        "Thread/get",
        {
            "accountId": account_id,
            "ids": thread_ids,
            "properties": ["id", "emailIds"]
        },
        "threads"
    ])
}
