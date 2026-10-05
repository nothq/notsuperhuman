use std::collections::HashMap;

use crate::model::{
    MailReadState, MailStarState, MailThreadTriageChange, MailThreadTriageRequest,
    MailThreadTriageResult, MailWorkspace,
};
use serde_json::{json, Value};

use super::{mail_capabilities, mail_snooze_capabilities, MailLiveClient, MailLiveContext};
use crate::live::{
    helpers::decode_account_jmap_response,
    types::{JmapEmailCreateResult, JmapSetResponse},
};

struct MoveJmapThreadRequest<'a> {
    context: &'a MailLiveContext,
    mailbox_id: &'a str,
    thread_id: &'a str,
    destination_mailbox_id: &'a str,
    call_id: &'a str,
}

struct JmapThreadEmailUpdates<'a> {
    context: &'a MailLiveContext,
    email_ids: &'a [String],
    update: serde_json::Map<String, Value>,
    capabilities: Vec<&'static str>,
    call_id: &'a str,
}

impl MailLiveClient {
    pub fn archive_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
    ) -> Result<MailWorkspace, String> {
        let context = self.context();
        let archive_mailbox_id = self.mailbox_id_for_role(context, "archive")?;
        self.move_jmap_thread(MoveJmapThreadRequest {
            context,
            mailbox_id,
            thread_id,
            destination_mailbox_id: archive_mailbox_id.as_str(),
            call_id: "archive",
        })
    }

    pub fn move_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        destination_mailbox_id: &str,
    ) -> Result<MailWorkspace, String> {
        let context = self.context();
        self.move_jmap_thread(MoveJmapThreadRequest {
            context,
            mailbox_id,
            thread_id,
            destination_mailbox_id,
            call_id: "move",
        })
    }

    pub fn snooze_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        remind_at: &str,
    ) -> Result<MailWorkspace, String> {
        let context = self.context();
        let snoozed_mailbox_id = self.mailbox_id_for_role(context, "snoozed")?;
        let thread = self.load_jmap_thread(context, thread_id)?;
        if thread.email_ids.is_empty() {
            return self.refresh_mailbox_workspace(mailbox_id);
        }
        let updates = thread
            .email_ids
            .iter()
            .map(|email_id| {
                (
                    email_id.clone(),
                    snooze_email_update(mailbox_id, snoozed_mailbox_id.as_str(), remind_at),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        self.set_jmap_thread_email_updates(JmapThreadEmailUpdates {
            context,
            email_ids: &thread.email_ids,
            update: updates,
            capabilities: mail_snooze_capabilities().to_vec(),
            call_id: "snooze",
        })?;
        self.remove_message_details(&thread.email_ids)?;
        self.refresh_mailbox_workspace(mailbox_id)
    }

    pub fn set_mail_thread_triage(
        &self,
        request: MailThreadTriageRequest,
    ) -> Result<MailThreadTriageResult, String> {
        let context = self.context();
        if context.account.is_read_only {
            return Err(format!("mail account {} is read-only", context.account.id));
        }
        let thread = self.load_jmap_thread(context, request.thread_id.as_str())?;
        self.ensure_mail_thread_triage_allowed(context, &thread.email_ids, request.change)?;
        if !thread.email_ids.is_empty() {
            let update = thread
                .email_ids
                .iter()
                .map(|email_id| (email_id.clone(), triage_email_update(request.change)))
                .collect();
            self.set_jmap_thread_email_updates(JmapThreadEmailUpdates {
                context,
                email_ids: &thread.email_ids,
                update,
                capabilities: mail_capabilities().to_vec(),
                call_id: "triage",
            })?;
            self.remove_message_details(&thread.email_ids)?;
        }
        let workspace = self.refresh_mailbox_workspace(request.mailbox_id.as_str())?;
        let thread = self.load_mail_thread(request.thread_id.as_str())?;
        Ok(MailThreadTriageResult { workspace, thread })
    }

    fn ensure_mail_thread_triage_allowed(
        &self,
        context: &MailLiveContext,
        email_ids: &[String],
        change: MailThreadTriageChange,
    ) -> Result<(), String> {
        if email_ids.is_empty() {
            return Ok(());
        }
        let messages = self.fetch_mail_message_summaries(
            &context.api_url,
            context.account.id.as_str(),
            email_ids.to_vec(),
        )?;
        let mailboxes = self
            .load_mailboxes(&context.api_url, context.account.id.as_str())?
            .list
            .into_iter()
            .map(|mailbox| (mailbox.id.clone(), mailbox))
            .collect::<HashMap<_, _>>();
        for email_id in email_ids {
            let message = messages
                .iter()
                .find(|message| message.id == *email_id)
                .ok_or_else(|| format!("Email/get omitted thread email {email_id}"))?;
            if message.mailbox_ids.is_empty() {
                return Err(format!(
                    "cannot change triage for email {email_id} without mailbox membership"
                ));
            }
            for mailbox_id in &message.mailbox_ids {
                let mailbox = mailboxes.get(mailbox_id).ok_or_else(|| {
                    format!(
                        "cannot change triage for email {email_id}: mailbox {mailbox_id} is not readable"
                    )
                })?;
                let allowed = match change {
                    MailThreadTriageChange::Read(_) => mailbox.my_rights.may_set_seen,
                    MailThreadTriageChange::Star(_) => mailbox.my_rights.may_set_keywords,
                };
                if !allowed {
                    return Err(format!(
                        "mailbox {mailbox_id} does not allow {} changes for email {email_id}",
                        triage_change_label(change)
                    ));
                }
            }
        }
        Ok(())
    }

    fn move_jmap_thread(
        &self,
        request: MoveJmapThreadRequest<'_>,
    ) -> Result<MailWorkspace, String> {
        let thread = self.load_jmap_thread(request.context, request.thread_id)?;
        if thread.email_ids.is_empty() {
            return self.refresh_mailbox_workspace(request.mailbox_id);
        }
        let updates = thread
            .email_ids
            .iter()
            .map(|email_id| {
                (
                    email_id.clone(),
                    move_email_update(request.mailbox_id, request.destination_mailbox_id),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        self.set_jmap_thread_email_updates(JmapThreadEmailUpdates {
            context: request.context,
            email_ids: &thread.email_ids,
            update: updates,
            capabilities: mail_capabilities().to_vec(),
            call_id: request.call_id,
        })?;
        self.remove_message_details(&thread.email_ids)?;
        self.refresh_mailbox_workspace(request.mailbox_id)
    }

    fn set_jmap_thread_email_updates(
        &self,
        request: JmapThreadEmailUpdates<'_>,
    ) -> Result<(), String> {
        for email_ids in request
            .email_ids
            .chunks(request.context.limits.max_objects_in_set.get())
        {
            let update = email_ids
                .iter()
                .map(|email_id| {
                    request
                        .update
                        .get(email_id)
                        .cloned()
                        .map(|update| (email_id.clone(), update))
                        .ok_or_else(|| {
                            format!(
                                "mail {} update is missing email {email_id}",
                                request.call_id
                            )
                        })
                })
                .collect::<Result<serde_json::Map<_, _>, _>>()?;
            let response = self.jmap_request(
                &request.context.api_url,
                json!({
                    "using": &request.capabilities,
                    "methodCalls": [[
                        "Email/set",
                        {
                            "accountId": request.context.account.id.as_str(),
                            "update": Value::Object(update)
                        },
                        request.call_id
                    ]]
                }),
            )?;
            let set_response =
                decode_account_jmap_response::<JmapSetResponse<JmapEmailCreateResult>>(
                    &response,
                    request.call_id,
                    "Email/set",
                    request.context.account.id.as_str(),
                )?;
            for email_id in email_ids {
                if let Some(error) = set_response.not_updated.get(email_id) {
                    return Err(jmap_set_error(request.call_id, email_id, error));
                }
                if !set_response.updated.contains_key(email_id) {
                    return Err(format!(
                        "mail {} Email/set omitted result for {email_id}",
                        request.call_id
                    ));
                }
            }
        }
        Ok(())
    }
}

fn triage_email_update(change: MailThreadTriageChange) -> Value {
    let (path, value) = match change {
        MailThreadTriageChange::Read(MailReadState::Read) => ("keywords/$seen", Value::Bool(true)),
        MailThreadTriageChange::Read(MailReadState::Unread) => ("keywords/$seen", Value::Null),
        MailThreadTriageChange::Star(MailStarState::Starred) => {
            ("keywords/$flagged", Value::Bool(true))
        }
        MailThreadTriageChange::Star(MailStarState::Unstarred) => {
            ("keywords/$flagged", Value::Null)
        }
    };
    serde_json::Map::from_iter([(path.to_string(), value)]).into()
}

fn triage_change_label(change: MailThreadTriageChange) -> &'static str {
    match change {
        MailThreadTriageChange::Read(_) => "read/unread",
        MailThreadTriageChange::Star(_) => "star",
    }
}

fn jmap_set_error(
    call_id: &str,
    email_id: &str,
    error: &crate::live::types::JmapSetError,
) -> String {
    if error.description.is_empty() {
        format!(
            "mail {call_id} Email/set failed for {email_id}: {}",
            error.kind
        )
    } else {
        format!(
            "mail {call_id} Email/set failed for {email_id}: {}: {}",
            error.kind, error.description
        )
    }
}

fn move_email_update(mailbox_id: &str, destination_mailbox_id: &str) -> Value {
    let mut patch = serde_json::Map::new();
    patch.insert(format!("mailboxIds/{mailbox_id}"), Value::Null);
    patch.insert(
        format!("mailboxIds/{destination_mailbox_id}"),
        Value::Bool(true),
    );
    Value::Object(patch)
}

fn snooze_email_update(mailbox_id: &str, snoozed_mailbox_id: &str, remind_at: &str) -> Value {
    let mut patch = serde_json::Map::new();
    patch.insert(format!("mailboxIds/{mailbox_id}"), Value::Null);
    patch.insert(
        format!("mailboxIds/{snoozed_mailbox_id}"),
        Value::Bool(true),
    );
    patch.insert(
        "snoozed".to_string(),
        json!({
            "until": remind_at,
            "moveToMailboxId": mailbox_id,
            "setKeywords": { "$seen": true }
        }),
    );
    Value::Object(patch)
}
