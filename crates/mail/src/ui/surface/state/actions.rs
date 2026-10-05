use super::{
    Context, MailActionPaletteKind, MailActionPaletteOption, MailActionPaletteOptionAction,
    MailActionPaletteState, MailRowAction, MailSnoozePreset, SurfaceState,
};

mod thread;

struct MailThreadActionContext {
    source_mailbox_id: String,
    refresh_mailbox_id: String,
}

impl SurfaceState {
    pub(crate) fn handle_mail_thread_action(
        &mut self,
        thread_id: String,
        action: MailRowAction,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_thread_action_allowed(thread_id.as_str(), action) {
            return true;
        }
        match action {
            MailRowAction::MarkDone => self.archive_mail_thread(thread_id, cx),
            MailRowAction::RemindMe => {
                self.open_mail_action_palette(thread_id, MailActionPaletteKind::RemindMe, cx)
            }
            MailRowAction::Move => {
                self.open_mail_action_palette(thread_id, MailActionPaletteKind::Move, cx)
            }
        }
    }

    pub(crate) fn handle_active_mail_thread_action(
        &mut self,
        action: MailRowAction,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(thread_id) = self
            .mail_open_thread_id
            .clone()
            .or_else(|| self.mail_selected_thread_id.clone())
        else {
            return false;
        };
        self.handle_mail_thread_action(thread_id, action, cx)
    }

    pub(crate) fn archive_selected_mail_thread(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(thread_id) = self
            .mail_open_thread_id
            .clone()
            .or_else(|| self.mail_selected_thread_id.clone())
        else {
            return false;
        };
        self.archive_mail_thread(thread_id, cx)
    }

    pub(crate) fn open_mail_action_palette(
        &mut self,
        thread_id: String,
        kind: MailActionPaletteKind,
        cx: &mut Context<Self>,
    ) -> bool {
        self.mail_action_palette = Some(MailActionPaletteState {
            kind,
            thread_id,
            selected_index: 0,
        });
        self.mail_hovered_thread_id = None;
        self.mail_hovered_message_card_id = None;
        self.mail_hovered_message_action = None;
        self.mail_hovered_history_message_id = None;
        let had_expanded_history = !self.mail_expanded_history_message_ids.is_empty();
        self.mail_expanded_history_message_ids.clear();
        if had_expanded_history {
            self.sync_mail_open_thread_body_list_state();
        }
        self.mail_error = None;
        cx.notify();
        true
    }

    pub(crate) fn close_mail_action_palette(&mut self, cx: &mut Context<Self>) -> bool {
        if self.mail_action_palette.take().is_none() {
            return false;
        }
        cx.notify();
        true
    }

    pub(crate) fn move_mail_action_palette_selection(
        &mut self,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = self.mail_action_palette.as_ref() else {
            return false;
        };
        let options = self.mail_action_palette_options(state);
        if options.is_empty() {
            return true;
        }
        let selected_index = crate::ui::offset_index(
            state.selected_index.min(options.len() - 1),
            delta,
            options.len(),
        );
        if let Some(state) = self.mail_action_palette.as_mut() {
            state.selected_index = selected_index;
        }
        cx.notify();
        true
    }

    pub(crate) fn activate_mail_action_palette(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(state) = self.mail_action_palette.clone() else {
            return false;
        };
        let options = self.mail_action_palette_options(&state);
        let Some(option) = options
            .get(state.selected_index.min(options.len().saturating_sub(1)))
            .cloned()
        else {
            return true;
        };
        match option.action {
            MailActionPaletteOptionAction::Snooze { preset } => {
                self.snooze_mail_thread(state.thread_id, preset, cx)
            }
            MailActionPaletteOptionAction::Move { mailbox_id } => {
                self.move_mail_thread(state.thread_id, mailbox_id, cx)
            }
        }
    }

    pub(crate) fn mail_action_palette_options(
        &self,
        state: &MailActionPaletteState,
    ) -> Vec<MailActionPaletteOption> {
        match state.kind {
            MailActionPaletteKind::RemindMe => [
                MailSnoozePreset::LaterToday,
                MailSnoozePreset::Tomorrow,
                MailSnoozePreset::NextWeek,
            ]
            .into_iter()
            .map(|preset| MailActionPaletteOption {
                label: preset.label().to_string(),
                detail: preset.detail().to_string(),
                action: MailActionPaletteOptionAction::Snooze { preset },
            })
            .collect(),
            MailActionPaletteKind::Move => {
                self.mail_move_destination_options(state.thread_id.as_str())
            }
        }
    }

    fn mail_move_destination_options(&self, thread_id: &str) -> Vec<MailActionPaletteOption> {
        let Some(workspace) = self.mail_workspace() else {
            return Vec::new();
        };
        let Ok(action_context) = self.mail_thread_action_context(thread_id) else {
            return Vec::new();
        };
        workspace
            .mailboxes
            .iter()
            .filter(|mailbox| mailbox.id != action_context.source_mailbox_id)
            .filter(|mailbox| mailbox.rights.may_add_items)
            .filter(|mailbox| {
                !matches!(
                    mailbox.role.as_deref(),
                    Some("drafts" | "sent" | "all" | "snoozed")
                )
            })
            .map(|mailbox| MailActionPaletteOption {
                label: crate::ui::mailbox_display_label(mailbox),
                detail: mail_move_destination_detail(mailbox.total_emails),
                action: MailActionPaletteOptionAction::Move {
                    mailbox_id: mailbox.id.clone(),
                },
            })
            .collect()
    }

    pub(crate) fn mail_thread_action_allowed(
        &self,
        thread_id: &str,
        action: MailRowAction,
    ) -> bool {
        if self.mail_workspace_actions_in_flight != 0 || self.mail_active_account_is_read_only() {
            return false;
        }
        let Some(workspace) = self.mail_workspace() else {
            return false;
        };
        let Ok(context) = self.mail_thread_action_context(thread_id) else {
            return false;
        };
        let Some(source) = workspace
            .mailboxes
            .iter()
            .find(|mailbox| mailbox.id == context.source_mailbox_id)
        else {
            return false;
        };
        if !source.rights.may_remove_items {
            return false;
        }
        match action {
            MailRowAction::MarkDone => mailbox_role_accepts_items(workspace, "archive"),
            MailRowAction::RemindMe => mailbox_role_accepts_items(workspace, "snoozed"),
            MailRowAction::Move => !self.mail_move_destination_options(thread_id).is_empty(),
        }
    }

    pub(super) fn mail_thread_move_allowed(
        &self,
        thread_id: &str,
        destination_mailbox_id: &str,
    ) -> bool {
        if !self.mail_thread_action_allowed(thread_id, MailRowAction::Move) {
            return false;
        }
        let Ok(context) = self.mail_thread_action_context(thread_id) else {
            return false;
        };
        self.mail_workspace().is_some_and(|workspace| {
            workspace.mailboxes.iter().any(|mailbox| {
                mailbox.id == destination_mailbox_id
                    && mailbox.id != context.source_mailbox_id
                    && mailbox.rights.may_add_items
                    && !matches!(
                        mailbox.role.as_deref(),
                        Some("drafts" | "sent" | "all" | "snoozed")
                    )
            })
        })
    }

    fn mail_thread_action_context(
        &self,
        thread_id: &str,
    ) -> Result<MailThreadActionContext, String> {
        let refresh_mailbox_id = self.mail_selected_tab_id.clone();
        if refresh_mailbox_id.is_empty() {
            return Err("missing selected Mail mailbox".to_string());
        }
        if self.mail_search_session().is_none() {
            return Ok(MailThreadActionContext {
                source_mailbox_id: refresh_mailbox_id.clone(),
                refresh_mailbox_id,
            });
        }
        let message = self
            .mail_search_session()
            .and_then(|session| {
                session
                    .messages
                    .iter()
                    .find(|message| message.thread_id == thread_id)
            })
            .ok_or_else(|| format!("missing search result thread {thread_id}"))?;
        let workspace = self
            .mail_workspace()
            .ok_or_else(|| "missing Mail workspace".to_string())?;
        let source_mailbox = workspace
            .mailboxes
            .iter()
            .filter(|mailbox| mailbox.role.as_deref() != Some("all"))
            .filter(|mailbox| {
                message
                    .mailbox_ids
                    .iter()
                    .any(|mailbox_id| mailbox_id == &mailbox.id)
            })
            .min_by_key(|mailbox| mail_thread_action_mailbox_rank(mailbox.role.as_deref()))
            .ok_or_else(|| {
                format!(
                    "search result {} has no actionable Mail mailbox membership",
                    message.id
                )
            })?;
        Ok(MailThreadActionContext {
            source_mailbox_id: source_mailbox.id.clone(),
            refresh_mailbox_id,
        })
    }
}

fn mailbox_role_accepts_items(workspace: &crate::ui::MailWorkspace, role: &str) -> bool {
    workspace
        .mailboxes
        .iter()
        .any(|mailbox| mailbox.role.as_deref() == Some(role) && mailbox.rights.may_add_items)
}

fn mail_move_destination_detail(total_emails: u64) -> String {
    match total_emails {
        1 => "1 email".to_string(),
        count => format!("{count} emails"),
    }
}

fn mail_thread_action_mailbox_rank(role: Option<&str>) -> usize {
    match role.unwrap_or_default() {
        "inbox" => 0,
        "drafts" => 1,
        "sent" => 2,
        "archive" => 3,
        "snoozed" => 4,
        "junk" => 5,
        "trash" => 6,
        _ => usize::MAX,
    }
}
