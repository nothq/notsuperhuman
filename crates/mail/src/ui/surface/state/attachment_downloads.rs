use std::{fmt::Display, path::PathBuf, sync::Arc};

use crate::model::{
    MailAttachment, MailAttachmentDownloadRequest, MailDownloadFileName, MailWorkspaceApi,
};
use crate::ui::surface::MailAttachmentMessageKey;
use gpui::{AppContext, PathPromptOptions, TaskExt};

use super::{Context, MailAttachmentDownloadState, SurfaceState, Window};

#[derive(Clone)]
struct MailAttachmentDownloadGuard {
    account_generation: u64,
    thread_id: String,
    message_id: String,
    attachments: Vec<MailAttachment>,
    exact_downloadable_set: bool,
}

struct MailAttachmentDestinationPrompt {
    operation: MailAttachmentDownloadState,
    guard: MailAttachmentDownloadGuard,
    workspace_api: Arc<dyn MailWorkspaceApi>,
    attachment: MailAttachment,
    download_directory: PathBuf,
    file_name: MailDownloadFileName,
}

struct MailAttachmentDirectoryPrompt {
    operation: MailAttachmentDownloadState,
    guard: MailAttachmentDownloadGuard,
    workspace_api: Arc<dyn MailWorkspaceApi>,
    attachments: Vec<MailAttachment>,
}

impl MailAttachmentDownloadGuard {
    fn is_current(&self, state: &SurfaceState) -> bool {
        if state.mail_account_generation != self.account_generation
            || state.mail_open_thread_id.as_deref() != Some(self.thread_id.as_str())
        {
            return false;
        }
        let Some(message) = state.mail_open_thread_detail().and_then(|thread| {
            thread
                .message_details
                .iter()
                .find(|message| message.id == self.message_id)
                .cloned()
        }) else {
            return false;
        };
        if !message.received {
            return false;
        }
        if self.exact_downloadable_set {
            let downloadable = message
                .attachments
                .into_iter()
                .filter(|attachment| attachment.blob_id.is_some())
                .collect::<Vec<_>>();
            return downloadable == self.attachments;
        }
        self.attachments
            .iter()
            .all(|attachment| message.attachments.contains(attachment))
    }
}

impl SurfaceState {
    pub(crate) fn mail_attachment_download_pending(&self) -> bool {
        self.mail_attachment_download
            .as_ref()
            .is_some_and(|download| {
                download.account_generation == self.mail_account_generation
                    && self.mail_open_thread_id.as_deref() == Some(download.thread_id.as_str())
                    && self.mail_open_thread_detail().is_some_and(|thread| {
                        thread
                            .message_details
                            .iter()
                            .any(|message| message.id == download.message_id)
                    })
            })
    }

    pub(crate) fn prompt_download_received_mail_attachment(
        &mut self,
        message: MailAttachmentMessageKey,
        attachment: MailAttachment,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if attachment.blob_id.is_none() {
            return;
        }
        let guard = MailAttachmentDownloadGuard {
            account_generation: self.mail_account_generation,
            thread_id: message.thread_id,
            message_id: message.message_id,
            attachments: vec![attachment.clone()],
            exact_downloadable_set: false,
        };
        let Some(operation) = self.begin_mail_attachment_download(&guard, cx) else {
            return;
        };
        let workspace_api = match self.mail_workspace_api() {
            Some(workspace_api) => workspace_api,
            None => {
                self.finish_mail_attachment_download(
                    &operation,
                    &guard,
                    Some("missing Mail workspace api".to_string()),
                    cx,
                );
                return;
            }
        };
        let download_directory = match self.mail_local_file_api.default_download_directory() {
            Ok(download_directory) => download_directory,
            Err(error) => {
                self.finish_mail_attachment_download(&operation, &guard, Some(error), cx);
                return;
            }
        };
        let file_name = MailDownloadFileName::for_attachment(
            attachment.name.as_str(),
            attachment.content_type.as_str(),
        );
        self.prompt_mail_attachment_destination(
            MailAttachmentDestinationPrompt {
                operation,
                guard,
                workspace_api,
                attachment,
                download_directory,
                file_name,
            },
            window,
            cx,
        );
    }

    fn prompt_mail_attachment_destination(
        &mut self,
        prompt: MailAttachmentDestinationPrompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let MailAttachmentDestinationPrompt {
            operation,
            guard,
            workspace_api,
            attachment,
            download_directory,
            file_name,
        } = prompt;
        let destination_receiver =
            cx.prompt_for_new_path(&download_directory, Some(file_name.as_str()));
        let entity = cx.entity();
        window
            .spawn(cx, async move |cx| {
                let selection = destination_receiver.await.unwrap_or(Ok(None));
                let destination = match selected_mail_download_path(selection, "save dialog") {
                    Ok(Some(destination)) => destination,
                    Ok(None) => {
                        entity.update(cx, |this, cx| {
                            this.finish_mail_attachment_download(&operation, &guard, None, cx);
                        });
                        return Ok::<(), String>(());
                    }
                    Err(error) => {
                        entity.update(cx, |this, cx| {
                            this.finish_mail_attachment_download(
                                &operation,
                                &guard,
                                Some(error),
                                cx,
                            );
                        });
                        return Ok(());
                    }
                };
                let result = cx
                    .background_spawn(async move {
                        download_selected_mail_attachment(workspace_api, attachment, destination)
                    })
                    .await;
                entity.update(cx, |this, cx| {
                    this.finish_mail_attachment_download(&operation, &guard, result.err(), cx);
                });
                Ok(())
            })
            .detach_and_log_err(cx);
    }

    pub(crate) fn prompt_download_all_received_mail_attachments(
        &mut self,
        message: MailAttachmentMessageKey,
        attachments: Vec<MailAttachment>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let attachments = attachments
            .into_iter()
            .filter(|attachment| attachment.blob_id.is_some())
            .collect::<Vec<_>>();
        if attachments.is_empty() {
            return;
        }
        let guard = MailAttachmentDownloadGuard {
            account_generation: self.mail_account_generation,
            thread_id: message.thread_id,
            message_id: message.message_id,
            attachments: attachments.clone(),
            exact_downloadable_set: true,
        };
        let Some(operation) = self.begin_mail_attachment_download(&guard, cx) else {
            return;
        };
        let workspace_api = match self.mail_workspace_api() {
            Some(workspace_api) => workspace_api,
            None => {
                self.finish_mail_attachment_download(
                    &operation,
                    &guard,
                    Some("missing Mail workspace api".to_string()),
                    cx,
                );
                return;
            }
        };
        self.prompt_mail_attachment_directory(
            MailAttachmentDirectoryPrompt {
                operation,
                guard,
                workspace_api,
                attachments,
            },
            window,
            cx,
        );
    }

    fn prompt_mail_attachment_directory(
        &mut self,
        prompt: MailAttachmentDirectoryPrompt,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let MailAttachmentDirectoryPrompt {
            operation,
            guard,
            workspace_api,
            attachments,
        } = prompt;
        let directory_receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose download folder".into()),
        });
        let entity = cx.entity();
        window
            .spawn(cx, async move |cx| {
                let selection = directory_receiver.await.unwrap_or(Ok(None));
                let directory = match selected_mail_download_directory(selection) {
                    Ok(Some(directory)) => directory,
                    Ok(None) => {
                        entity.update(cx, |this, cx| {
                            this.finish_mail_attachment_download(&operation, &guard, None, cx);
                        });
                        return Ok::<(), String>(());
                    }
                    Err(error) => {
                        entity.update(cx, |this, cx| {
                            this.finish_mail_attachment_download(
                                &operation,
                                &guard,
                                Some(error),
                                cx,
                            );
                        });
                        return Ok(());
                    }
                };
                let result = cx
                    .background_spawn(async move {
                        download_mail_attachments_sequentially(
                            workspace_api,
                            attachments,
                            directory,
                        )
                    })
                    .await;
                entity.update(cx, |this, cx| {
                    this.finish_mail_attachment_download(&operation, &guard, result.err(), cx);
                });
                Ok(())
            })
            .detach_and_log_err(cx);
    }

    fn begin_mail_attachment_download(
        &mut self,
        guard: &MailAttachmentDownloadGuard,
        cx: &mut Context<Self>,
    ) -> Option<MailAttachmentDownloadState> {
        if !guard.is_current(self) || self.mail_attachment_download_pending() {
            return None;
        }
        self.mail_attachment_download_generation =
            self.mail_attachment_download_generation.wrapping_add(1);
        let operation = MailAttachmentDownloadState {
            generation: self.mail_attachment_download_generation,
            account_generation: guard.account_generation,
            thread_id: guard.thread_id.clone(),
            message_id: guard.message_id.clone(),
        };
        self.mail_attachment_download = Some(operation.clone());
        self.mail_error = None;
        cx.notify();
        Some(operation)
    }

    fn finish_mail_attachment_download(
        &mut self,
        operation: &MailAttachmentDownloadState,
        guard: &MailAttachmentDownloadGuard,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self.mail_attachment_download.as_ref() != Some(operation) {
            return;
        }
        self.mail_attachment_download = None;
        if guard.is_current(self) {
            if let Some(error) = error {
                self.report_mail_error(error);
            }
        }
        cx.notify();
    }
}

fn download_mail_attachments_sequentially(
    workspace_api: Arc<dyn MailWorkspaceApi>,
    attachments: Vec<MailAttachment>,
    directory: std::path::PathBuf,
) -> Result<(), String> {
    for attachment in attachments {
        let request = MailAttachmentDownloadRequest::for_directory(&attachment, directory.clone())
            .ok_or_else(|| {
                format!(
                    "mail attachment {:?} no longer has a downloadable blob ID",
                    attachment.name
                )
            })?;
        workspace_api.download_mail_attachment(request)?;
    }
    Ok(())
}

fn download_selected_mail_attachment(
    workspace_api: Arc<dyn MailWorkspaceApi>,
    attachment: MailAttachment,
    destination: PathBuf,
) -> Result<(), String> {
    let request = MailAttachmentDownloadRequest::for_selected_path(&attachment, destination)
        .ok_or_else(|| "mail attachment no longer has a downloadable blob ID".to_string())?;
    workspace_api.download_mail_attachment(request).map(|_| ())
}

fn selected_mail_download_path<Error: Display>(
    selection: Result<Option<PathBuf>, Error>,
    picker: &str,
) -> Result<Option<PathBuf>, String> {
    selection.map_err(|error| format!("failed to open mail attachment {picker}: {error}"))
}

fn selected_mail_download_directory<Error: Display>(
    selection: Result<Option<Vec<PathBuf>>, Error>,
) -> Result<Option<PathBuf>, String> {
    let Some(paths) = selection
        .map_err(|error| format!("failed to open mail attachment folder picker: {error}"))?
    else {
        return Ok(None);
    };
    if paths.len() != 1 {
        return Err(format!(
            "mail attachment folder picker returned {} paths; expected one",
            paths.len()
        ));
    }
    Ok(paths.into_iter().next())
}
