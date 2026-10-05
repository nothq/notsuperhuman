use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use reqwest::Url;
use serde_json::json;
use time::OffsetDateTime;

use super::{
    api::{
        GmailApi, GmailAttachmentData, GmailDraft, GmailDraftList, GmailLabel, GmailLabelList,
        GmailMessage, GmailMessagePage, GmailSendAsList, GmailThread, GmailThreadPage,
    },
    convert::{
        decode_base64url, decode_text, gmail_email, gmail_mailboxes, inline_bodies,
        is_system_mailbox, parse_address_list, split_attachment_blob_id, ARCHIVE, DRAFT, INBOX,
        SENT, SNOOZED, SPAM, STARRED, TRASH, UNREAD,
    },
    mime::{raw_draft, MimeAttachment},
    search::{gmail_query, mailbox_query},
    snooze::SnoozeStore,
};
use crate::live::{
    client::download::{receive_download, validate_download_destination, MAIL_DOWNLOAD_MAX_BYTES},
    helpers::{into_mail_message, into_mail_message_summary},
    MailLiveConfig, MailSplitPreferencesStore, MailWorkspaceCache,
};
use crate::model::{
    MailAccountInfo, MailAddress, MailAttachment, MailAttachmentDownloadRequest,
    MailAttachmentDownloadResult, MailBlobId, MailDraftRequest, MailIdentity, MailMessage,
    MailMessagePage, MailReadState, MailSearchPage, MailSearchQuery, MailSendResult,
    MailSplitDefinition, MailSplitMutation, MailStarState, MailThread, MailThreadTriageChange,
    MailThreadTriageRequest, MailThreadTriageResult, MailUploadFile, MailWorkspace, Mailbox,
};

const PAGE_SIZE: usize = 25;
const GMAIL_SERVER: &str = "https://gmail.googleapis.com";
const GMAIL_API_ENDPOINT: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
/// The label counts worth a request each: unread badges and the drafts count.
const COUNTED_LABELS: &[&str] = &[INBOX, DRAFT, SPAM];
const SUMMARY_HEADERS: &[&str] = &[
    "From",
    "To",
    "Cc",
    "Bcc",
    "Reply-To",
    "Sender",
    "Subject",
    "Date",
    "Message-ID",
    "In-Reply-To",
    "References",
];

/// One Gmail account, served through the same workspace interface as JMAP.
#[derive(Clone)]
pub(crate) struct GmailWorkspace {
    api: GmailApi,
    account: MailAccountInfo,
    cache: Option<MailWorkspaceCache>,
    split_preferences: MailSplitPreferencesStore,
    snoozes: SnoozeStore,
    state: Arc<Mutex<GmailState>>,
}

#[derive(Default)]
struct GmailState {
    mailboxes: Vec<Mailbox>,
    /// Gmail pages with opaque tokens; the app pages by position. Keyed by
    /// listing (folder ID or search query) and the position the token opens.
    page_tokens: HashMap<(String, usize), String>,
    uploads: HashMap<String, MailUploadFile>,
    next_upload: u64,
}

/// Which threads a page lists: a folder, or a search.
enum Listing<'a> {
    Mailbox(&'a str),
    Search(String),
}

struct ThreadPage {
    threads: Vec<GmailThread>,
    next_position: Option<usize>,
}

impl GmailWorkspace {
    pub(crate) fn open(
        account: crate::live::accounts::SavedAccount,
        is_primary: bool,
    ) -> Result<Self, String> {
        let email = account.label().to_string();
        let email = email.as_str();
        let config = MailLiveConfig {
            server: GMAIL_SERVER.to_string(),
        };
        let api_endpoint = Url::parse(GMAIL_API_ENDPOINT).map_err(|error| error.to_string())?;
        let split_preferences =
            MailSplitPreferencesStore::open(&config, &api_endpoint, email, email)?;
        let cache = match MailWorkspaceCache::from_config(&config, email, email) {
            Ok(cache) => Some(cache),
            Err(error) => {
                eprintln!("mail cache unavailable: {error}");
                None
            }
        };
        Ok(Self {
            api: GmailApi::new(account)?,
            account: gmail_account_info(email, is_primary),
            cache,
            split_preferences,
            snoozes: SnoozeStore::for_account(email)?,
            state: Arc::new(Mutex::new(GmailState::default())),
        })
    }

    pub(crate) fn account(&self) -> &MailAccountInfo {
        &self.account
    }

    pub(crate) fn load_startup_workspace(&self) -> Result<(MailWorkspace, bool), String> {
        if let Some(workspace) = self.cached_workspace(None)? {
            self.lock()?.mailboxes = workspace.mailboxes.clone();
            return Ok((workspace, true));
        }
        self.load_workspace(None)
            .map(|workspace| (workspace, false))
    }

    fn cached_workspace(&self, mailbox_id: Option<&str>) -> Result<Option<MailWorkspace>, String> {
        match self.cache.as_ref() {
            Some(cache) => cache.load_cached_workspace(mailbox_id),
            None => Ok(None),
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, GmailState>, String> {
        self.state
            .lock()
            .map_err(|error| format!("Gmail state lock poisoned: {error}"))
    }

    fn load_workspace(&self, mailbox_id: Option<&str>) -> Result<MailWorkspace, String> {
        self.wake_due_snoozes()?;
        let mailboxes = self.load_mailboxes()?;
        let selected = mailbox_id
            .filter(|id| mailboxes.iter().any(|mailbox| mailbox.id == *id))
            .unwrap_or(INBOX)
            .to_string();
        let page = self.thread_page(&Listing::Mailbox(selected.as_str()), 0)?;
        let workspace = MailWorkspace {
            account_id: self.account.id.clone(),
            mailbox_email: self.account.address.clone(),
            display_name: self.account.name.clone(),
            owner_username: self.api.email.clone(),
            messages: thread_summaries(&page.threads, Some(selected.as_str())),
            message_next_position: page.next_position,
            selected_mailbox_id: selected,
            mailboxes,
        };
        if let Some(cache) = self.cache.as_ref() {
            cache.persist_workspace(&workspace, String::new(), String::new());
        }
        Ok(workspace)
    }

    fn load_mailboxes(&self) -> Result<Vec<Mailbox>, String> {
        let labels = self.api.get::<GmailLabelList>("/labels", &[])?.labels;
        let counted = self.api.parallel(COUNTED_LABELS, |api, id| {
            api.get::<GmailLabel>(&format!("/labels/{id}"), &[])
        })?;
        let counts = counted
            .into_iter()
            .map(|label| (label.id.clone(), label))
            .collect::<HashMap<_, _>>();
        let mut mailboxes = gmail_mailboxes(&labels, &counts);
        let snoozed = self.snoozes.all().len() as u64;
        if let Some(mailbox) = mailboxes.iter_mut().find(|mailbox| mailbox.id == SNOOZED) {
            mailbox.total_emails = snoozed;
        }
        self.lock()?.mailboxes = mailboxes.clone();
        Ok(mailboxes)
    }

    fn known_mailboxes(&self) -> Result<Vec<Mailbox>, String> {
        let mailboxes = self.lock()?.mailboxes.clone();
        if mailboxes.is_empty() {
            return self.load_mailboxes();
        }
        Ok(mailboxes)
    }

    fn thread_page(&self, listing: &Listing<'_>, position: usize) -> Result<ThreadPage, String> {
        if let Listing::Mailbox(SNOOZED) = listing {
            return self.snoozed_page(position);
        }
        let key = match listing {
            Listing::Mailbox(id) => format!("mailbox:{id}"),
            Listing::Search(query) => format!("search:{query}"),
        };
        let token = if position == 0 {
            None
        } else {
            match self
                .lock()?
                .page_tokens
                .get(&(key.clone(), position))
                .cloned()
            {
                Some(token) => Some(token),
                None => {
                    return Ok(ThreadPage {
                        threads: Vec::new(),
                        next_position: None,
                    })
                }
            }
        };
        let page_size = PAGE_SIZE.to_string();
        let mut query = vec![("maxResults", page_size.as_str())];
        let label_query;
        match listing {
            Listing::Mailbox(id) if *id == ARCHIVE => {
                label_query = mailbox_query(id, &[]);
                query.push(("q", label_query.as_str()));
            }
            Listing::Mailbox(id) => query.push(("labelIds", id)),
            Listing::Search(search) => query.push(("q", search.as_str())),
        }
        if let Some(token) = token.as_deref() {
            query.push(("pageToken", token));
        }
        let page = self.api.get::<GmailThreadPage>("/threads", &query)?;
        let ids = page
            .threads
            .iter()
            .map(|thread| thread.id.clone())
            .collect::<Vec<_>>();
        let next_position = page.next_page_token.map(|token| {
            let next = position + ids.len();
            if let Ok(mut state) = self.lock() {
                state.page_tokens.insert((key, next), token);
            }
            next
        });
        Ok(ThreadPage {
            threads: self.thread_metadata(&ids)?,
            next_position,
        })
    }

    fn snoozed_page(&self, position: usize) -> Result<ThreadPage, String> {
        let mut snoozes = self.snoozes.all();
        snoozes.sort_by(|left, right| left.remind_at.cmp(&right.remind_at));
        let ids = snoozes
            .iter()
            .skip(position)
            .take(PAGE_SIZE)
            .map(|snooze| snooze.thread_id.clone())
            .collect::<Vec<_>>();
        let next_position = (snoozes.len() > position + ids.len()).then_some(position + ids.len());
        Ok(ThreadPage {
            threads: self.thread_metadata(&ids)?,
            next_position,
        })
    }

    fn thread_metadata(&self, ids: &[String]) -> Result<Vec<GmailThread>, String> {
        self.api.parallel(ids, |api, id| {
            let mut query = vec![("format", "metadata")];
            query.extend(
                SUMMARY_HEADERS
                    .iter()
                    .map(|header| ("metadataHeaders", *header)),
            );
            api.get::<GmailThread>(&format!("/threads/{id}"), &query)
        })
    }

    fn full_thread(&self, thread_id: &str) -> Result<MailThread, String> {
        let thread = self
            .api
            .get::<GmailThread>(&format!("/threads/{thread_id}"), &[("format", "full")])?;
        let messages = thread
            .messages
            .iter()
            .map(|message| self.full_message(message))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(MailThread {
            id: thread.id,
            messages,
        })
    }

    fn full_message(&self, message: &GmailMessage) -> Result<MailMessage, String> {
        let (mut bodies, pending) = inline_bodies(message);
        let fetched = self.api.parallel(&pending, |api, body| {
            api.get::<GmailAttachmentData>(
                &format!(
                    "/messages/{}/attachments/{}",
                    message.id, body.attachment_id
                ),
                &[],
            )
            .map(|data| decode_text(data.data.as_str(), body.charset.as_deref()))
        })?;
        for (body, text) in pending.iter().zip(fetched) {
            bodies.insert(body.part_id.clone(), text);
        }
        Ok(into_mail_message(gmail_email(message, &bodies)))
    }

    fn modify_thread(&self, thread_id: &str, add: &[&str], remove: &[&str]) -> Result<(), String> {
        if add.is_empty() && remove.is_empty() {
            return Ok(());
        }
        self.api.post::<serde_json::Value>(
            &format!("/threads/{thread_id}/modify"),
            json!({ "addLabelIds": add, "removeLabelIds": remove }),
        )?;
        Ok(())
    }

    /// Takes a thread out of the folder it is listed in and out of the inbox.
    fn archive_thread(&self, mailbox_id: &str, thread_id: &str) -> Result<(), String> {
        let mut remove = vec![INBOX];
        if removable_label(mailbox_id) && mailbox_id != INBOX {
            remove.push(mailbox_id);
        }
        self.modify_thread(thread_id, &[], &remove)
    }

    fn wake_due_snoozes(&self) -> Result<(), String> {
        let due = self.snoozes.due(OffsetDateTime::now_utc());
        if due.is_empty() {
            return Ok(());
        }
        for snooze in &due {
            self.modify_thread(snooze.thread_id.as_str(), &[INBOX, UNREAD], &[])?;
        }
        let ids = due
            .into_iter()
            .map(|snooze| snooze.thread_id)
            .collect::<Vec<_>>();
        self.snoozes.remove(&ids)
    }

    /// Gmail drafts have their own IDs, which start with `r`; the message
    /// behind a draft changes every time the draft is saved.
    fn draft_id_for(&self, id: &str) -> Result<String, String> {
        if id.starts_with('r') {
            return Ok(id.to_string());
        }
        self.api
            .get::<GmailDraftList>("/drafts", &[("maxResults", "500")])?
            .drafts
            .into_iter()
            .find(|draft| draft.message.id == id)
            .map(|draft| draft.id)
            .ok_or_else(|| format!("Gmail draft {id} no longer exists"))
    }

    fn draft_message(&self, draft_id: &str) -> Result<MailMessage, String> {
        let draft = self
            .api
            .get::<GmailDraft>(&format!("/drafts/{draft_id}"), &[("format", "full")])?;
        let mut message = self.full_message(&draft.message)?;
        message.id = draft.id;
        Ok(message)
    }

    fn draft_body(&self, request: &MailDraftRequest) -> Result<serde_json::Value, String> {
        let attachments = request
            .attachments
            .iter()
            .map(|attachment| self.attachment_bytes(attachment))
            .collect::<Result<Vec<_>, _>>()?;
        let mut message = json!({ "raw": raw_draft(request, &attachments) });
        if let Some(thread_id) = self.reply_thread_id(request)? {
            message["threadId"] = json!(thread_id);
        }
        Ok(json!({ "message": message }))
    }

    /// Replies stay in their Gmail thread only when the draft names it.
    fn reply_thread_id(&self, request: &MailDraftRequest) -> Result<Option<String>, String> {
        let Some(parent) = request.in_reply_to.last() else {
            return Ok(None);
        };
        let query = format!("rfc822msgid:{parent}");
        Ok(self
            .api
            .get::<GmailMessagePage>(
                "/messages",
                &[
                    ("q", query.as_str()),
                    ("maxResults", "1"),
                    ("includeSpamTrash", "true"),
                ],
            )?
            .messages
            .into_iter()
            .next()
            .map(|message| message.thread_id))
    }

    fn attachment_bytes(&self, attachment: &MailAttachment) -> Result<MimeAttachment, String> {
        let blob_id = attachment
            .blob_id
            .as_ref()
            .ok_or_else(|| format!("attachment {:?} has no content", attachment.name))?;
        let bytes = match self.lock()?.uploads.get(blob_id.as_str()) {
            Some(upload) => Some(upload.bytes.clone()),
            None => None,
        };
        let bytes = match bytes {
            Some(bytes) => bytes,
            None => self.download_blob(blob_id)?,
        };
        Ok(MimeAttachment {
            name: attachment.name.clone(),
            content_type: attachment.content_type.clone(),
            bytes,
        })
    }

    fn download_blob(&self, blob_id: &MailBlobId) -> Result<Vec<u8>, String> {
        let (message_id, attachment_id) = split_attachment_blob_id(blob_id.as_str())
            .ok_or_else(|| format!("unknown Gmail attachment {}", blob_id.as_str()))?;
        let data = self.api.get::<GmailAttachmentData>(
            &format!("/messages/{message_id}/attachments/{attachment_id}"),
            &[],
        )?;
        Ok(decode_base64url(data.data.as_str()))
    }

    fn triage_labels(change: MailThreadTriageChange) -> (Vec<&'static str>, Vec<&'static str>) {
        match change {
            MailThreadTriageChange::Read(MailReadState::Read) => (vec![], vec![UNREAD]),
            MailThreadTriageChange::Read(MailReadState::Unread) => (vec![UNREAD], vec![]),
            MailThreadTriageChange::Star(MailStarState::Starred) => (vec![STARRED], vec![]),
            MailThreadTriageChange::Star(MailStarState::Unstarred) => (vec![], vec![STARRED]),
        }
    }
}

impl crate::model::MailWorkspaceApi for GmailWorkspace {
    fn load_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.load_workspace(Some(mailbox_id))
    }

    fn load_cached_mail_workspace(
        &self,
        mailbox_id: &str,
    ) -> Result<Option<MailWorkspace>, String> {
        self.cached_workspace(Some(mailbox_id))
    }

    fn refresh_mail_workspace(&self, mailbox_id: &str) -> Result<MailWorkspace, String> {
        self.load_workspace(Some(mailbox_id))
    }

    fn supports_mail_split_preferences(&self) -> bool {
        true
    }

    fn load_mail_split_definitions(&self) -> Result<Vec<MailSplitDefinition>, String> {
        self.split_preferences.snapshot()
    }

    fn mutate_mail_split_definitions(
        &self,
        mutation: MailSplitMutation,
    ) -> Result<Vec<MailSplitDefinition>, String> {
        self.split_preferences.mutate(mutation)
    }

    fn archive_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
    ) -> Result<MailWorkspace, String> {
        self.archive_thread(mailbox_id, thread_id)?;
        self.snoozes.remove(&[thread_id.to_string()])?;
        self.load_workspace(Some(mailbox_id))
    }

    fn move_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        destination_mailbox_id: &str,
    ) -> Result<MailWorkspace, String> {
        if mailbox_id == TRASH {
            self.api
                .post::<serde_json::Value>(&format!("/threads/{thread_id}/untrash"), json!({}))?;
        }
        match destination_mailbox_id {
            TRASH => {
                self.api
                    .post::<serde_json::Value>(&format!("/threads/{thread_id}/trash"), json!({}))?;
            }
            ARCHIVE => self.archive_thread(mailbox_id, thread_id)?,
            destination => {
                let remove = if removable_label(mailbox_id) {
                    vec![mailbox_id]
                } else {
                    Vec::new()
                };
                self.modify_thread(thread_id, &[destination], &remove)?;
            }
        }
        self.snoozes.remove(&[thread_id.to_string()])?;
        self.load_workspace(Some(mailbox_id))
    }

    fn snooze_mail_thread(
        &self,
        mailbox_id: &str,
        thread_id: &str,
        remind_at: &str,
    ) -> Result<MailWorkspace, String> {
        self.archive_thread(mailbox_id, thread_id)?;
        self.snoozes.add(thread_id, remind_at)?;
        self.load_workspace(Some(mailbox_id))
    }

    fn supports_mail_thread_triage(&self) -> bool {
        true
    }

    fn set_mail_thread_triage(
        &self,
        request: MailThreadTriageRequest,
    ) -> Result<MailThreadTriageResult, String> {
        let (add, remove) = Self::triage_labels(request.change);
        self.modify_thread(request.thread_id.as_str(), &add, &remove)?;
        Ok(MailThreadTriageResult {
            workspace: self.load_workspace(Some(request.mailbox_id.as_str()))?,
            thread: self.full_thread(request.thread_id.as_str())?,
        })
    }

    fn load_mail_message_detail(
        &self,
        _mailbox_id: &str,
        message_id: &str,
    ) -> Result<MailMessage, String> {
        if message_id.starts_with('r') {
            return self.draft_message(message_id);
        }
        let message = self
            .api
            .get::<GmailMessage>(&format!("/messages/{message_id}"), &[("format", "full")])?;
        self.full_message(&message)
    }

    fn load_mail_message_page(
        &self,
        mailbox_id: &str,
        position: usize,
    ) -> Result<MailMessagePage, String> {
        let page = self.thread_page(&Listing::Mailbox(mailbox_id), position)?;
        Ok(MailMessagePage {
            mailbox_id: mailbox_id.to_string(),
            position,
            messages: thread_summaries(&page.threads, Some(mailbox_id)),
            next_position: page.next_position,
        })
    }

    fn search_mail_messages(
        &self,
        query: &MailSearchQuery,
        position: usize,
    ) -> Result<MailSearchPage, String> {
        let mailboxes = self.known_mailboxes()?;
        let Some(search) = gmail_query(query, &mailboxes) else {
            return Ok(MailSearchPage {
                position,
                ..MailSearchPage::default()
            });
        };
        let page = self.thread_page(&Listing::Search(search), position)?;
        let thread_message_counts = page
            .threads
            .iter()
            .map(|thread| (thread.id.clone(), thread.messages.len()))
            .collect();
        Ok(MailSearchPage {
            position,
            messages: thread_summaries(&page.threads, None),
            next_position: page.next_position,
            snippets: HashMap::new(),
            thread_message_counts,
        })
    }

    fn load_mail_thread(&self, thread_id: &str) -> Result<MailThread, String> {
        self.full_thread(thread_id)
    }

    fn load_mail_identity(&self) -> Result<MailIdentity, String> {
        let send_as = self
            .api
            .get::<GmailSendAsList>("/settings/sendAs", &[])?
            .send_as
            .into_iter()
            .find(|send_as| send_as.is_primary)
            .ok_or_else(|| "Gmail did not return a primary send-as address".to_string())?;
        Ok(MailIdentity {
            id: send_as.send_as_email.clone(),
            name: send_as.display_name,
            email: send_as.send_as_email,
            reply_to: parse_address_list(send_as.reply_to_address.as_str())
                .into_iter()
                .map(|address| MailAddress {
                    name: address.name.unwrap_or_default(),
                    email: address.email,
                })
                .collect(),
            bcc: Vec::new(),
            text_signature: html_signature_text(send_as.signature.as_str()),
            html_signature: send_as.signature,
        })
    }

    fn create_mail_draft(&self, request: MailDraftRequest) -> Result<MailMessage, String> {
        let body = self.draft_body(&request)?;
        let draft = self.api.post::<GmailDraft>("/drafts", body)?;
        self.draft_message(draft.id.as_str())
    }

    fn update_mail_draft(
        &self,
        draft_id: &str,
        request: MailDraftRequest,
    ) -> Result<MailMessage, String> {
        let draft_id = self.draft_id_for(draft_id)?;
        let mut body = self.draft_body(&request)?;
        body["id"] = json!(draft_id);
        let draft = self
            .api
            .put::<GmailDraft>(&format!("/drafts/{draft_id}"), body)?;
        self.draft_message(draft.id.as_str())
    }

    fn delete_mail_draft(&self, draft_id: &str) -> Result<(), String> {
        let draft_id = self.draft_id_for(draft_id)?;
        self.api.delete(&format!("/drafts/{draft_id}"))
    }

    fn upload_mail_files(&self, files: Vec<MailUploadFile>) -> Result<Vec<MailAttachment>, String> {
        let mut state = self.lock()?;
        files
            .into_iter()
            .map(|file| {
                state.next_upload += 1;
                let blob_id = format!("upload:{}", state.next_upload);
                let attachment = MailAttachment {
                    blob_id: Some(MailBlobId::parse(blob_id.clone())?),
                    name: file.name.clone(),
                    content_type: file.content_type.clone(),
                    size: file.bytes.len() as u64,
                    disposition: Some("attachment".to_string()),
                };
                state.uploads.insert(blob_id, file);
                Ok(attachment)
            })
            .collect()
    }

    fn download_mail_attachment(
        &self,
        mut request: MailAttachmentDownloadRequest,
    ) -> Result<MailAttachmentDownloadResult, String> {
        if request.declared_size > MAIL_DOWNLOAD_MAX_BYTES {
            return Err(format!(
                "attachment {:?} is larger than {MAIL_DOWNLOAD_MAX_BYTES} bytes",
                request.file_name.as_str()
            ));
        }
        validate_download_destination(&request.destination)?;
        let bytes = self.download_blob(&request.blob_id)?;
        request.declared_size = bytes.len() as u64;
        receive_download(bytes.as_slice(), request)
    }

    fn send_mail_draft(
        &self,
        draft_id: &str,
        _identity_id: &str,
        _to: Vec<MailAddress>,
        _cc: Vec<MailAddress>,
    ) -> Result<MailSendResult, String> {
        let draft_id = self.draft_id_for(draft_id)?;
        let sent = self
            .api
            .post::<GmailMessage>("/drafts/send", json!({ "id": draft_id }))?;
        Ok(MailSendResult {
            draft_id,
            thread_id: sent.thread_id,
            sent_mailbox_id: SENT.to_string(),
        })
    }

    fn load_remote_image(
        &self,
        url: &str,
    ) -> Result<Option<remote_image_model::RemoteImageData>, String> {
        remote_image::load_public_remote_image_data(url).map(Some)
    }
}

fn gmail_account_info(email: &str, is_primary: bool) -> MailAccountInfo {
    MailAccountInfo {
        id: email.to_ascii_lowercase(),
        name: email.to_string(),
        address: Some(email.to_string()),
        is_primary,
        is_personal: true,
        is_read_only: false,
        can_submit: true,
    }
}

/// Labels a thread can be taken out of by moving it elsewhere. Synthetic
/// folders and filters such as Starred are not memberships.
fn removable_label(mailbox_id: &str) -> bool {
    matches!(mailbox_id, INBOX | SPAM) || !is_system_mailbox(mailbox_id)
}

/// One list row per thread: its newest message in the listed folder, read
/// only once every message is read, starred if any message is.
fn thread_summaries(threads: &[GmailThread], mailbox_id: Option<&str>) -> Vec<MailMessage> {
    threads
        .iter()
        .filter_map(|thread| {
            let in_folder = |message: &&GmailMessage| match mailbox_id {
                Some(id) if !matches!(id, ARCHIVE | SNOOZED) => {
                    message.label_ids.iter().any(|label| label == id)
                }
                _ => true,
            };
            let newest = thread
                .messages
                .iter()
                .filter(in_folder)
                .max_by_key(|message| message.internal_date.parse::<u64>().unwrap_or_default())
                .or_else(|| thread.messages.last())?;
            let mut summary = GmailMessage {
                label_ids: thread
                    .messages
                    .iter()
                    .flat_map(|message| message.label_ids.iter().cloned())
                    .collect(),
                ..newest.clone()
            };
            summary.label_ids.sort();
            summary.label_ids.dedup();
            Some(into_mail_message_summary(gmail_email(
                &summary,
                &HashMap::new(),
            )))
        })
        .collect()
}

/// Gmail signatures are HTML; compose writes plain text.
fn html_signature_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag = String::new();
    for character in html.chars() {
        match character {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let name = tag
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if matches!(name.as_str(), "br" | "br/" | "div" | "p" | "li" | "tr") {
                    text.push('\n');
                }
            }
            _ if in_tag => tag.push(character),
            _ => text.push(character),
        }
    }
    let text = super::convert::unescape_html(&text);
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{html_signature_text, removable_label, thread_summaries};
    use crate::live::gmail::api::GmailThread;

    #[test]
    fn signatures_become_plain_lines() {
        assert_eq!(
            html_signature_text("<div dir=\"ltr\">Ada Lee<br>Founder &amp; CEO</div>"),
            "Ada Lee\nFounder & CEO"
        );
    }

    #[test]
    fn only_real_memberships_are_removed_on_move() {
        assert!(removable_label("INBOX"));
        assert!(removable_label("Label_12"));
        assert!(!removable_label("STARRED"));
        assert!(!removable_label("ARCHIVE"));
    }

    #[test]
    fn thread_rows_show_the_newest_message_in_the_folder() {
        let thread: GmailThread = serde_json::from_value(serde_json::json!({
            "id": "t1",
            "messages": [
                {"id": "m1", "threadId": "t1", "labelIds": ["INBOX", "UNREAD"], "internalDate": "1000",
                 "payload": {"headers": [{"name": "Subject", "value": "First"}]}},
                {"id": "m2", "threadId": "t1", "labelIds": ["SENT"], "internalDate": "2000",
                 "payload": {"headers": [{"name": "Subject", "value": "Re: First"}]}}
            ]
        }))
        .expect("thread");
        let inbox = thread_summaries(std::slice::from_ref(&thread), Some("INBOX"));
        assert_eq!(inbox[0].id, "m1");
        assert!(inbox[0].is_unread);
        assert!(inbox[0].mailbox_ids.contains(&"SENT".to_string()));
        let all = thread_summaries(std::slice::from_ref(&thread), None);
        assert_eq!(all[0].id, "m2");
    }
}
