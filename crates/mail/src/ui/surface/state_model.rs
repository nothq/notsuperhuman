mod construction;
mod runtime;
mod search;
mod splits;

pub(crate) use search::*;
pub(crate) use splits::*;

use super::MailStartup;
use super::{
    MailContactHistoryRow, MailListSource, MailMessageAction, MailThreadDetail, MailTriageControl,
};
use crate::ui::*;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

use runtime::MailRuntimeState;

use crate::model::MailContactAddress;

type MailComposeTextInputs = RefCell<HashMap<MailComposeField, Entity<TextInput>>>;
type MailSvgIconCache = RefCell<HashMap<String, Arc<RenderImage>>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailComposeDraftSaveSnapshot {
    pub(crate) draft_id: String,
    pub(crate) request: MailDraftRequest,
}

#[derive(Clone, Debug)]
pub(crate) struct MailComposeDraftSave {
    pub(crate) generation: u64,
    pub(crate) draft_id: String,
    pub(crate) request: MailDraftRequest,
}

impl MailComposeDraftSave {
    pub(crate) fn snapshot(&self) -> MailComposeDraftSaveSnapshot {
        MailComposeDraftSaveSnapshot {
            draft_id: self.draft_id.clone(),
            request: self.request.clone(),
        }
    }
}

/// The sign-in screen: Superhuman, or a JMAP server and API token.
#[derive(Clone, Debug, Default)]
pub(crate) struct MailSignInState {
    pub(crate) server: String,
    pub(crate) token: String,
    pub(crate) focused_field: MailSignInField,
    pub(crate) pending: Option<MailSignInPending>,
    pub(crate) error: Option<String>,
    /// Opened over a signed-in workspace, so Escape can close it.
    pub(crate) dismissible: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) enum MailSignInField {
    #[default]
    Server,
    Token,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MailSignInPending {
    Superhuman,
    Jmap,
}

#[derive(Clone, Debug)]
pub(crate) struct MailSendRecipients {
    pub(crate) to: Vec<MailAddress>,
    pub(crate) cc: Vec<MailAddress>,
}

#[derive(Clone, Debug)]
pub(crate) struct MailComposeSendIntent {
    pub(crate) draft_id: String,
    pub(crate) identity_id: String,
    pub(crate) recipients: MailSendRecipients,
    pub(crate) selected_tab_id: String,
    pub(crate) drafts_mailbox_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct MailAccountPaletteState {
    pub(crate) query: String,
    pub(crate) selected_index: usize,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct MailAccountViewState {
    pub(crate) selected_mailbox_id: String,
    pub(crate) list_source: MailListSource,
    pub(crate) selected_thread_id: Option<String>,
    pub(crate) open_thread_id: Option<String>,
    pub(crate) list_scroll: ListOffset,
    pub(crate) search_list_scroll: ListOffset,
    pub(crate) search_state: MailSearchState,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MailTriageIntentKey {
    pub(crate) thread_id: String,
    pub(crate) control: MailTriageControl,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MailTriageIntent {
    pub(crate) generation: u64,
    pub(crate) change: MailThreadTriageChange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MailAttachmentDownloadState {
    pub(crate) generation: u64,
    pub(crate) account_generation: u64,
    pub(crate) thread_id: String,
    pub(crate) message_id: String,
}

/// The received message whose attachments a download saves.
#[derive(Clone, Debug)]
pub(crate) struct MailAttachmentMessageKey {
    pub(crate) thread_id: String,
    pub(crate) message_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct MailContactHistoryState {
    pub(crate) request_generation: u64,
    pub(crate) thread_id: String,
    pub(crate) status: MailContactHistoryStatus,
}

#[derive(Clone, Debug)]
pub(crate) enum MailContactHistoryStatus {
    Loading,
    Ready(Arc<[MailContactHistoryRow]>),
    Error(String),
}

pub struct SurfaceState {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) appearance_mode: AppearanceMode,
    /// Platform line measurement, captured from the window; the mail renderer
    /// needs real font metrics to resolve table column widths.
    pub(crate) text_measure: crate::ui::MailTextMeasure,
    pub(crate) theme: SurfaceTheme,
    pub(crate) preview_width: f32,
    pub(crate) viewport_height: f32,
    pub(crate) window_controls_visible: bool,
    pub(crate) chrome_top_inset: f32,
    pub(crate) mail_shortcuts_focused: bool,
    pub(crate) mail_startup: MailStartup,
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fixture_mail_workspace: Option<MailWorkspace>,
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fixture_mail_workspace_api: Option<Arc<dyn crate::model::MailWorkspaceApi>>,
    pub(crate) mail_local_file_api: Arc<dyn crate::model::MailLocalFileApi>,
    pub(crate) mail_selected_tab_id: String,
    pub(crate) mail_list_source: MailListSource,
    pub(crate) mail_account_palette_scroll: ScrollHandle,
    pub(crate) mail_list_threads: Arc<[MailListThread]>,
    pub(crate) mail_list_state: ListState,
    pub(crate) mail_search_list_state: ListState,
    pub(crate) mail_open_thread_body_list_state: ListState,
    pub(crate) mail_compose_focused_field: MailComposeField,
    runtime: MailRuntimeState,
}

impl Deref for SurfaceState {
    type Target = MailRuntimeState;

    fn deref(&self) -> &Self::Target {
        &self.runtime
    }
}

impl DerefMut for SurfaceState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.runtime
    }
}

pub(crate) struct MailSurfaceStateConfig {
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) input: SurfaceInput,
    pub(crate) local_file_api: Arc<dyn crate::model::MailLocalFileApi>,
    pub(crate) startup: MailStartup,
    pub(crate) theme: SurfaceTheme,
    pub(crate) preview_width: f32,
    pub(crate) viewport_height: f32,
    pub(crate) window_controls_visible: bool,
    pub(crate) chrome_top_inset: f32,
}

impl SurfaceState {
    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn apply_input(&mut self, input: SurfaceInput, startup: MailStartup) {
        let search_was_open = self.mail_search_state.session().is_some();
        let workspace = input.workspace;
        self.mail_account_generation = self.mail_account_generation.wrapping_add(1);
        self.mail_folder_drawer_open = false;
        self.mail_account_palette = None;
        self.mail_switching_account_id = None;
        self.mail_account_recovery_in_flight = false;
        self.mail_account_view_states.clear();
        self.mail_split_definitions.clear();
        self.mail_split_preferences_supported = false;
        self.mail_split_settings = None;
        self.mail_split_text_inputs.borrow_mut().clear();
        self.mail_local_file_api = input.local_file_api;
        self.invalidate_mail_search(true);
        self.mail_startup = startup;
        self.fixture_mail_workspace = None;
        self.fixture_mail_workspace_api = input.workspace_api;
        if let Some(workspace) = workspace {
            self.set_workspace(workspace);
        }
        self.mail_loading_message_page = None;
        self.mail_workspace_refresh_generation =
            self.mail_workspace_refresh_generation.wrapping_add(1);
        self.mail_workspace_actions_in_flight = 0;
        self.mail_mark_read_after_triage.clear();
        self.mail_mark_read_suppressed.clear();
        self.mail_triage_preflights.clear();
        self.mail_triage_intents.clear();
        self.mail_triage_summaries.clear();
        self.mail_triage_reconciled_generation = self.mail_triage_generation;
        self.mail_thread_detail_cache.clear();
        self.clear_mail_contact_histories();
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_open_thread_body_list_thread_id = None;
        if search_was_open || self.mail_workspace().is_some() {
            self.sync_mail_list_rows();
        }
    }

    #[cfg(any(test, feature = "test-support"))]
    pub(crate) fn set_workspace(&mut self, workspace: MailWorkspace) {
        let selected_thread_id = (!self.mail_search_open())
            .then(|| self.mail_selected_thread_id.clone())
            .flatten()
            .filter(|thread_id| {
                workspace
                    .messages
                    .iter()
                    .any(|message| &message.thread_id == thread_id)
            })
            .or_else(|| {
                workspace
                    .messages
                    .first()
                    .map(|message| message.thread_id.clone())
            });
        self.invalidate_mail_search(false);
        self.mail_selected_tab_id = workspace.selected_mailbox_id.clone();
        self.mail_list_source = MailListSource::Mailbox(self.mail_selected_tab_id.clone());
        self.mail_selected_thread_id = selected_thread_id;
        self.replace_mail_workspace(workspace);
        self.mail_loading_message_page = None;
        self.mail_thread_detail_cache.clear();
        self.mail_remote_image_discovery_thread_id = None;
        self.mail_open_thread_body_list_thread_id = None;
        self.sync_mail_list_rows();
    }

    pub(crate) fn report_mail_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        println!("[notsuperhuman mail] error: {message}");
        self.mail_error = Some(message);
    }

    pub(crate) fn build_mail_list_state(row_count: usize) -> ListState {
        ListState::new(row_count, ListAlignment::Top, px(72.0))
    }

    fn build_mail_list_state_for_surface(cx: &mut Context<Self>) -> ListState {
        let mail_list_state = Self::build_mail_list_state(0);
        mail_list_state.set_scroll_handler(cx.listener(|this, event: &ListScrollEvent, _, cx| {
            this.maybe_load_more_mail_messages(event.visible_range.end, event.count, cx);
        }));
        mail_list_state
    }

    fn build_initial_mail_search_list_state(cx: &mut Context<Self>) -> ListState {
        Self::build_mail_list_state_for_surface_with_estimated_height(36.0, cx)
    }

    pub(crate) fn build_mail_list_state_for_surface_with_estimated_height(
        estimated_height: f32,
        cx: &mut Context<Self>,
    ) -> ListState {
        let mail_list_state = ListState::new(0, ListAlignment::Top, px(estimated_height));
        mail_list_state.set_scroll_handler(cx.listener(|this, event: &ListScrollEvent, _, cx| {
            this.maybe_load_more_mail_messages(event.visible_range.end, event.count, cx);
        }));
        mail_list_state
    }

    pub(crate) fn build_mail_open_thread_body_list_state(row_count: usize) -> ListState {
        ListState::new(row_count, ListAlignment::Top, px(120.0))
    }

    pub(crate) fn spawn_background_task<Request, Result, Work, Apply>(
        &self,
        request: Request,
        cx: &mut Context<Self>,
        work: Work,
        apply: Apply,
    ) where
        Request: Send + 'static,
        Result: Send + 'static,
        Work: FnOnce(Request) -> Result + Send + 'static,
        Apply: FnOnce(&mut Self, Result, &mut Context<Self>) + 'static,
    {
        let account_generation = self.mail_account_generation;
        spawn_background_task_for_entity(request, cx, work, move |this, result, cx| {
            if this.mail_account_generation != account_generation {
                return;
            }
            apply(this, result, cx);
        });
    }

    pub(crate) fn spawn_timer_task<Token, Apply>(
        &self,
        token: Token,
        delay: Duration,
        cx: &mut Context<Self>,
        apply: Apply,
    ) where
        Token: Send + 'static,
        Apply: FnOnce(&mut Self, Token, &mut Context<Self>) + 'static,
    {
        spawn_timer_task_for_entity(token, delay, cx, apply);
    }
}

impl Render for SurfaceState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.appearance_mode = AppearanceMode::current(cx);
        self.text_measure =
            super::render_thread::card::mail_text_measure(window.text_system().clone());
        self.render_mail(cx)
    }
}

impl Focusable for SurfaceState {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
