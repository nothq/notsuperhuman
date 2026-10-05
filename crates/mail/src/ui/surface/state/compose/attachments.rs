use super::{
    mail_mailbox_id_for_role, mail_parse_addresses, Context, MailComposeSendIntent,
    MailSendRecipients, MailSendResult, MailUploadFile, SurfaceState, Window,
};
use gpui::{AppContext, TaskExt};

#[derive(Clone)]
struct MailAttachmentSelection {
    account_generation: u64,
    draft_id: String,
}

impl SurfaceState {
    pub(crate) fn prompt_for_mail_attachment_files(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(draft_id) = self.mail_compose_draft_id.clone() else {
            self.mail_compose_error = Some("open a draft before adding attachments".to_string());
            cx.notify();
            return;
        };
        let selection = MailAttachmentSelection {
            account_generation: self.mail_account_generation,
            draft_id,
        };
        let local_file_api = self.mail_local_file_api.clone();
        let entity = cx.entity();
        let paths_receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Select files".into()),
        });
        window
            .spawn(cx, async move |cx| {
                let paths = match paths_receiver.await {
                    Ok(Ok(Some(paths))) => paths,
                    Ok(Ok(None)) | Err(_) => return Ok::<(), String>(()),
                    Ok(Err(error)) => {
                        let selection = selection.clone();
                        entity.update(cx, |this, cx| {
                            if !this.mail_attachment_selection_is_current(&selection) {
                                return;
                            }
                            this.mail_compose_error =
                                Some(format!("failed to open mail attachment picker: {error}"));
                            cx.notify();
                        });
                        return Ok(());
                    }
                };
                let loaded = cx
                    .background_spawn(async move { local_file_api.load_upload_files(paths) })
                    .await;
                entity.update(cx, |this, cx| {
                    if !this.mail_attachment_selection_is_current(&selection) {
                        return;
                    }
                    match loaded {
                        Ok(files) => this.upload_mail_attachment_files(files, cx),
                        Err(message) => {
                            this.mail_compose_error = Some(message);
                            cx.notify();
                        }
                    }
                });
                Ok(())
            })
            .detach_and_log_err(cx);
    }

    fn mail_attachment_selection_is_current(&self, selection: &MailAttachmentSelection) -> bool {
        self.mail_account_generation == selection.account_generation
            && self.mail_compose_draft_id.as_deref() == Some(selection.draft_id.as_str())
    }

    pub(crate) fn upload_mail_attachment_files(
        &mut self,
        files: Vec<MailUploadFile>,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.mail_compose_error = Some("missing Mail workspace api".to_string());
            cx.notify();
            return;
        };
        self.mail_compose_pending = true;
        self.mail_compose_error = None;
        cx.notify();
        self.spawn_background_task(
            files,
            cx,
            move |files| workspace_api.upload_mail_files(files),
            |this, result, cx| {
                this.mail_compose_pending = false;
                match result {
                    Ok(mut attachments) => {
                        this.mail_compose_attachments.append(&mut attachments);
                        this.mail_compose_error = None;
                        cx.notify();
                        this.save_mail_draft_in_background(cx);
                    }
                    Err(message) => {
                        this.mail_compose_error = Some(message);
                        cx.notify();
                    }
                }
            },
        );
    }

    pub(crate) fn remove_mail_attachment(&mut self, attachment_name: &str, cx: &mut Context<Self>) {
        self.mail_compose_attachments
            .retain(|attachment| attachment.name != attachment_name);
        self.mail_compose_error = None;
        cx.notify();
        self.save_mail_draft_in_background(cx);
    }

    pub(crate) fn discard_mail_draft(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(draft_id) = self.mail_compose_draft_id.clone() else {
            self.clear_mail_compose_state();
            self.sync_mail_open_thread_body_list_state();
            cx.notify();
            return true;
        };
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.mail_compose_error = Some("missing Mail workspace api".to_string());
            cx.notify();
            return true;
        };
        let selected_tab_id = self.mail_selected_tab_id.clone();
        let drafts_mailbox_id = self
            .mail_workspace()
            .and_then(|workspace| mail_mailbox_id_for_role(workspace, "drafts"));
        self.cancel_mail_draft_autosave();
        self.mail_compose_pending = true;
        self.mail_compose_error = None;
        cx.notify();
        self.spawn_background_task(
            draft_id,
            cx,
            move |draft_id| workspace_api.delete_mail_draft(&draft_id),
            move |this, result, cx| match result {
                Ok(()) => {
                    this.clear_mail_compose_state();
                    if selected_tab_id == drafts_mailbox_id.clone().unwrap_or_default() {
                        this.reset_mail_open_thread_read_session();
                        this.mail_open_thread_id = None;
                        this.mail_remote_image_discovery_thread_id = None;
                    }
                    this.sync_mail_open_thread_body_list_state();
                    cx.notify();
                    if !selected_tab_id.is_empty() {
                        this.refresh_mail_mailbox_in_background(selected_tab_id.clone(), cx);
                    }
                    if let Some(drafts_mailbox_id) = drafts_mailbox_id.clone() {
                        if drafts_mailbox_id != selected_tab_id {
                            this.prefetch_mail_mailbox_in_background(drafts_mailbox_id, cx);
                        }
                    }
                }
                Err(message) => {
                    this.mail_compose_pending = false;
                    this.mail_compose_error = Some(message);
                    cx.notify();
                }
            },
        );
        true
    }

    pub(crate) fn send_mail_draft(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(draft_id) = self.mail_compose_draft_id.clone() else {
            self.mail_compose_error = Some("missing draft id".to_string());
            cx.notify();
            return true;
        };
        let Some(identity_id) = self.mail_send_identity_id(cx) else {
            return true;
        };
        let Some(recipients) = self.mail_send_recipients(cx) else {
            return true;
        };
        let selected_tab_id = self.mail_selected_tab_id.clone();
        let drafts_mailbox_id = self
            .mail_workspace()
            .and_then(|workspace| mail_mailbox_id_for_role(workspace, "drafts"));

        self.mail_compose_pending = true;
        self.mail_compose_error = None;
        self.mail_compose_send_after_autosave = Some(MailComposeSendIntent {
            draft_id,
            identity_id,
            recipients,
            selected_tab_id,
            drafts_mailbox_id,
        });
        cx.notify();
        self.flush_mail_draft_autosave_for_send(cx);
        true
    }

    pub(super) fn send_mail_draft_now(
        &mut self,
        intent: MailComposeSendIntent,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_api) = self.mail_workspace_api() else {
            self.mail_compose_pending = false;
            self.mail_compose_error = Some("missing Mail workspace api".to_string());
            cx.notify();
            return;
        };

        let selected_tab_id = intent.selected_tab_id.clone();
        let drafts_mailbox_id = intent.drafts_mailbox_id.clone();
        self.mail_compose_pending = true;
        self.mail_compose_error = None;
        cx.notify();
        self.spawn_background_task(
            intent,
            cx,
            move |intent| {
                workspace_api.send_mail_draft(
                    &intent.draft_id,
                    &intent.identity_id,
                    intent.recipients.to,
                    intent.recipients.cc,
                )
            },
            move |this, result, cx| match result {
                Ok(result) => {
                    this.apply_mail_send_result(
                        result,
                        selected_tab_id.clone(),
                        drafts_mailbox_id.clone(),
                        cx,
                    );
                }
                Err(message) => {
                    this.mail_compose_pending = false;
                    this.mail_compose_error = Some(message);
                    cx.notify();
                }
            },
        );
    }

    fn mail_send_identity_id(&mut self, cx: &mut Context<Self>) -> Option<String> {
        match self
            .mail_compose_identity_id
            .clone()
            .filter(|value| !value.is_empty())
        {
            Some(identity_id) => Some(identity_id),
            None => {
                self.mail_compose_error = Some("missing mail identity".to_string());
                cx.notify();
                None
            }
        }
    }

    fn mail_send_recipients(&mut self, cx: &mut Context<Self>) -> Option<MailSendRecipients> {
        let to = mail_parse_addresses(&self.mail_compose_to);
        let cc = mail_parse_addresses(&self.mail_compose_cc);
        if to.is_empty() && cc.is_empty() {
            self.mail_compose_error = Some("add at least one recipient before sending".to_string());
            cx.notify();
            None
        } else {
            Some(MailSendRecipients { to, cc })
        }
    }

    fn apply_mail_send_result(
        &mut self,
        result: MailSendResult,
        selected_tab_id: String,
        drafts_mailbox_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let thread_id = result.thread_id.clone();
        let sent_mailbox_id = result.sent_mailbox_id.clone();
        self.clear_mail_compose_state();
        self.reset_mail_open_thread_read_session();
        self.mail_selected_thread_id = Some(thread_id.clone());
        self.mail_open_thread_id = Some(thread_id.clone());
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_error = None;
        self.sync_mail_open_thread_body_list_state();
        cx.notify();
        self.load_mail_thread_in_background(thread_id, cx);
        if !selected_tab_id.is_empty() {
            self.refresh_mail_mailbox_in_background(selected_tab_id.clone(), cx);
        }
        if let Some(drafts_mailbox_id) = drafts_mailbox_id {
            if drafts_mailbox_id != selected_tab_id {
                self.prefetch_mail_mailbox_in_background(drafts_mailbox_id, cx);
            }
        }
        if sent_mailbox_id != selected_tab_id {
            self.prefetch_mail_mailbox_in_background(sent_mailbox_id, cx);
        }
    }
}
