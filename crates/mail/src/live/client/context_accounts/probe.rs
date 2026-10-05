use reqwest::{blocking::Client, Url};
use serde_json::{json, Value};

use crate::live::{
    helpers::{decode_jmap_response, validate_jmap_account_response},
    types::{
        JmapCoreLimits, JmapIdentity, JmapIdentityGetResponse, JmapMailbox, JmapMailboxGetResponse,
        JmapResponse,
    },
};
use crate::model::MailAccountInfo;

use super::super::{
    jmap_request, mail_capabilities, mail_submission_capabilities, JmapCredentials,
};

pub(super) fn accessible_accounts(
    client: &Client,
    api_session: &JmapCredentials,
    api_url: &Url,
    candidates: Vec<MailAccountInfo>,
    limits: JmapCoreLimits,
) -> Result<Vec<MailAccountInfo>, String> {
    let max_calls_in_request = limits.max_calls_in_request.get();
    let mut accounts = Vec::with_capacity(candidates.len());
    for chunk in candidates.chunks(max_calls_in_request) {
        let response = mailbox_probe_response(client, api_session, api_url, chunk)?;
        collect_mailbox_probe_results(&response, chunk, &mut accounts)?;
    }
    enrich_submission_identities(client, api_session, api_url, accounts, max_calls_in_request)
}

fn mailbox_probe_response(
    client: &Client,
    api_session: &JmapCredentials,
    api_url: &Url,
    accounts: &[MailAccountInfo],
) -> Result<JmapResponse, String> {
    let method_calls = accounts
        .iter()
        .enumerate()
        .map(|(index, account)| {
            json!([
                "Mailbox/get",
                {
                    "accountId": account.id,
                    "ids": null,
                    "properties": ["id", "name", "role", "totalEmails", "unreadEmails", "myRights"]
                },
                format!("mailbox-probe-{index}")
            ])
        })
        .collect::<Vec<_>>();
    jmap_request(
        client,
        api_session,
        api_url,
        json!({ "using": mail_capabilities(), "methodCalls": method_calls }),
    )
}

fn collect_mailbox_probe_results(
    response: &JmapResponse,
    candidates: &[MailAccountInfo],
    accounts: &mut Vec<MailAccountInfo>,
) -> Result<(), String> {
    for (index, candidate) in candidates.iter().enumerate() {
        let call_id = format!("mailbox-probe-{index}");
        let Some(mailboxes) = decode_mailbox_probe(response, candidate, &call_id)? else {
            continue;
        };
        if let Some(account) = account_from_mailboxes(candidate, &mailboxes)? {
            accounts.push(account);
        }
    }
    Ok(())
}

fn decode_mailbox_probe(
    response: &JmapResponse,
    candidate: &MailAccountInfo,
    call_id: &str,
) -> Result<Option<JmapMailboxGetResponse>, String> {
    let mailboxes =
        match decode_jmap_response::<JmapMailboxGetResponse>(response, call_id, "Mailbox/get") {
            Ok(mailboxes) => mailboxes,
            Err(_)
                if !candidate.is_primary
                    && jmap_call_is_expected_access_error(response, call_id) =>
            {
                return Ok(None);
            }
            Err(error) => {
                return Err(format!(
                    "mail account {} mailbox discovery failed: {error}",
                    candidate.id
                ));
            }
        };
    validate_jmap_account_response(
        "Mailbox/get",
        candidate.id.as_str(),
        mailboxes.account_id.as_str(),
    )?;
    Ok(Some(mailboxes))
}

fn account_from_mailboxes(
    candidate: &MailAccountInfo,
    mailboxes: &JmapMailboxGetResponse,
) -> Result<Option<MailAccountInfo>, String> {
    let readable_mailboxes = mailboxes
        .list
        .iter()
        .filter(|mailbox| mailbox.my_rights.may_read_items);
    if readable_mailboxes.clone().next().is_none() {
        if candidate.is_primary {
            return Err(format!(
                "primary mail account {} has no readable mailboxes",
                candidate.id
            ));
        }
        return Ok(None);
    }
    let mut account = candidate.clone();
    account.is_read_only =
        account.is_read_only || !readable_mailboxes.clone().any(mailbox_is_writable);
    account.can_submit =
        account.can_submit && !account.is_read_only && mailboxes_allow_submission(mailboxes);
    Ok(Some(account))
}

fn mailboxes_allow_submission(mailboxes: &JmapMailboxGetResponse) -> bool {
    let drafts = mailboxes
        .list
        .iter()
        .find(|mailbox| mailbox.role.as_deref() == Some("drafts"));
    let sent = mailboxes
        .list
        .iter()
        .find(|mailbox| mailbox.role.as_deref() == Some("sent"));
    drafts.is_some_and(|mailbox| {
        mailbox.my_rights.may_read_items
            && mailbox.my_rights.may_add_items
            && mailbox.my_rights.may_remove_items
            && mailbox.my_rights.may_set_keywords
    }) && sent
        .is_some_and(|mailbox| mailbox.my_rights.may_read_items && mailbox.my_rights.may_add_items)
}

fn enrich_submission_identities(
    client: &Client,
    api_session: &JmapCredentials,
    api_url: &Url,
    mut accounts: Vec<MailAccountInfo>,
    max_calls_in_request: usize,
) -> Result<Vec<MailAccountInfo>, String> {
    let submitter_indices = accounts
        .iter()
        .enumerate()
        .filter_map(|(index, account)| account.can_submit.then_some(index))
        .collect::<Vec<_>>();
    for chunk in submitter_indices.chunks(max_calls_in_request) {
        let response = identity_probe_response(client, api_session, api_url, &accounts, chunk)?;
        collect_identity_probe_results(&response, &mut accounts, chunk)?;
    }
    Ok(accounts)
}

fn identity_probe_response(
    client: &Client,
    api_session: &JmapCredentials,
    api_url: &Url,
    accounts: &[MailAccountInfo],
    account_indices: &[usize],
) -> Result<JmapResponse, String> {
    let method_calls = account_indices
        .iter()
        .enumerate()
        .map(|(call_index, account_index)| {
            json!([
                "Identity/get",
                { "accountId": accounts[*account_index].id, "ids": null },
                format!("identity-probe-{call_index}")
            ])
        })
        .collect::<Vec<_>>();
    jmap_request(
        client,
        api_session,
        api_url,
        json!({ "using": mail_submission_capabilities(), "methodCalls": method_calls }),
    )
}

fn collect_identity_probe_results(
    response: &JmapResponse,
    accounts: &mut [MailAccountInfo],
    account_indices: &[usize],
) -> Result<(), String> {
    for (call_index, account_index) in account_indices.iter().enumerate() {
        let call_id = format!("identity-probe-{call_index}");
        let account = &mut accounts[*account_index];
        let identity = decode_submission_identity(response, account, &call_id)?;
        account.can_submit = !account.is_read_only && identity.is_some();
        if let Some(identity) = identity.filter(|_| !account.is_primary) {
            account.address = Some(identity.email.trim().to_string());
            let identity_name = identity.name.trim();
            if !identity_name.is_empty() {
                account.name = identity_name.to_string();
            }
        }
    }
    Ok(())
}

fn decode_submission_identity(
    response: &JmapResponse,
    account: &MailAccountInfo,
    call_id: &str,
) -> Result<Option<JmapIdentity>, String> {
    let identities =
        match decode_jmap_response::<JmapIdentityGetResponse>(response, call_id, "Identity/get") {
            Ok(identities) => identities,
            Err(_) if jmap_call_is_expected_access_error(response, call_id) => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "mail account {} identity discovery failed: {error}",
                    account.id
                ));
            }
        };
    validate_jmap_account_response(
        "Identity/get",
        account.id.as_str(),
        identities.account_id.as_str(),
    )?;
    Ok(identities
        .list
        .into_iter()
        .find(|identity| !identity.email.trim().is_empty()))
}

fn mailbox_is_writable(mailbox: &JmapMailbox) -> bool {
    let rights = &mailbox.my_rights;
    rights.may_add_items
        || rights.may_remove_items
        || rights.may_set_seen
        || rights.may_set_keywords
        || rights.may_create_child
        || rights.may_rename
        || rights.may_delete
        || rights.may_submit
}

fn jmap_call_is_expected_access_error(response: &JmapResponse, call_id: &str) -> bool {
    response
        .method_responses
        .iter()
        .any(|(method, payload, id)| {
            id == call_id
                && method == "error"
                && matches!(
                    payload.get("type").and_then(Value::as_str),
                    Some("forbidden" | "accountNotFound")
                )
        })
}
