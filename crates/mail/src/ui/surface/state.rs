use super::{
    div, keystroke_input_text, mail_address_summary, mail_correspondent_address,
    mail_format_addresses, mail_forward_attachments, mail_forward_subject,
    mail_latest_thread_message, mail_list_thread_by_id, mail_list_threads,
    mail_list_threads_for_messages, mail_mailbox_id_for_role, mail_message_card_row_count,
    mail_palette, mail_parse_addresses, mail_replace_active_compose_address, mail_reply_recipients,
    mail_reply_subject, mail_search_list_threads, mail_search_thread_detail_from_summary,
    mail_split_compose_addresses, mail_tabs_with_splits, mail_thread_detail,
    mail_thread_detail_from_summary, mail_thread_quote, mail_thread_references,
    parse_data_image_url, px, render_svg_image, rgb, svg_from_body, AnyElement, Arc, Context,
    Duration, FluentBuilder, HashSet, Image, InteractiveElement, IntoElement, KeyDownEvent,
    ListState, MailAccountPaletteState, MailAccountViewState, MailActionPaletteKind,
    MailActionPaletteOption, MailActionPaletteOptionAction, MailActionPaletteState,
    MailAttachmentDownloadState, MailComposeAutocompleteItem, MailComposeDraftSave,
    MailComposeDraftSaveSnapshot, MailComposeField, MailComposeMode, MailComposeSendIntent,
    MailContactAddress, MailContactHistoryPage, MailContactHistoryRequest, MailContactHistoryRow,
    MailContactHistoryState, MailContactHistoryStatus, MailDraftRequest, MailListSource,
    MailListThread, MailMessage, MailMessageAction, MailMessagePage, MailRowAction,
    MailSearchOrigin, MailSearchState, MailSendRecipients, MailSendResult, MailSnoozePreset,
    MailStartup, MailThread, MailThreadDetail, MailThreadMessageDetail,
    MailTriageControl, MailUploadFile, MailWorkspace, ParentElement, RenderImage, ScrollHandle,
    Styled, SurfaceState, Window, MAIL_ACTIVITY_MAX_WIDTH, MAIL_ACTIVITY_MIN_WIDTH,
    MAIL_FONT_FAMILY, MAIL_PREVIEW_MIN_WIDTH, MAIL_SEARCH_TOP_INSET,
    MAIL_WINDOW_CHROME_CONTENT_ADJUSTMENT,
};
use gpui::font;

mod accounts;
mod actions;
mod attachment_downloads;
mod compose;
mod contact_history;
mod drawer;
mod keyboard;
mod list_rows;
mod pagination;
mod remote_images;
mod search;
mod selection;
mod sign_in;
mod splits;
mod startup;

impl SurfaceState {
    pub(crate) fn mail_content_top_inset(&self) -> f32 {
        if self.window_controls_visible {
            (self.chrome_top_inset - MAIL_WINDOW_CHROME_CONTENT_ADJUSTMENT).max(0.0)
        } else {
            0.0
        }
    }

    pub(crate) fn mail_surface_top_inset(&self) -> f32 {
        if self.window_controls_visible {
            self.mail_content_top_inset()
        } else if self.mail_search_open() {
            MAIL_SEARCH_TOP_INSET
        } else {
            0.0
        }
    }

    pub(crate) fn mail_surface_visible(&self) -> bool {
        self.mail_workspace().is_some()
    }

    pub(crate) fn mail_workspace(&self) -> Option<&MailWorkspace> {
        match &self.mail_startup {
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => self.fixture_mail_workspace.as_ref(),
            MailStartup::Ready(bootstrap) => Some(&bootstrap.workspace),
            MailStartup::Loading { .. }
            | MailStartup::Error { .. }
            | MailStartup::SignedOut => None,
        }
    }

    pub(crate) fn mail_workspace_api(&self) -> Option<Arc<dyn crate::model::MailWorkspaceApi>> {
        match &self.mail_startup {
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => self.fixture_mail_workspace_api.clone(),
            MailStartup::Ready(bootstrap) => Some(bootstrap.workspace_api.clone()),
            MailStartup::Loading { .. }
            | MailStartup::Error { .. }
            | MailStartup::SignedOut => None,
        }
    }

    pub(crate) fn mail_workspace_mut(&mut self) -> Option<&mut MailWorkspace> {
        match &mut self.mail_startup {
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => self.fixture_mail_workspace.as_mut(),
            MailStartup::Ready(bootstrap) => Some(&mut bootstrap.workspace),
            MailStartup::Loading { .. }
            | MailStartup::Error { .. }
            | MailStartup::SignedOut => None,
        }
    }

    pub(crate) fn replace_mail_workspace(&mut self, workspace: MailWorkspace) {
        match &mut self.mail_startup {
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => self.fixture_mail_workspace = Some(workspace),
            MailStartup::Ready(bootstrap) => bootstrap.workspace = workspace,
            MailStartup::Loading { .. }
            | MailStartup::Error { .. }
            | MailStartup::SignedOut => {}
        }
    }

    pub(crate) fn ensure_mail_active_thread_detail_cached(&mut self) {
        let Some(thread_id) = self.mail_open_thread_id.clone() else {
            return;
        };
        if self.mail_thread_detail_cache.contains_key(&thread_id) {
            return;
        }
        if !self.mail_thread_cache.contains_key(thread_id.as_str()) {
            return;
        }
        let Some(detail) = self.mail_uncached_thread_detail(&thread_id) else {
            return;
        };
        self.mail_thread_detail_cache
            .insert(thread_id, Arc::new(detail));
    }

    pub(crate) fn mail_uncached_thread_detail(&self, thread_id: &str) -> Option<MailThreadDetail> {
        let workspace = self.mail_workspace()?;
        self.mail_thread_cache
            .get(thread_id)
            .map(|thread| mail_thread_detail(workspace, thread))
            .or_else(|| {
                self.mail_visible_messages()
                    .iter()
                    .find(|message| message.thread_id == thread_id)
                    .map(|summary| mail_thread_detail_from_summary(workspace, summary))
            })
    }

    pub(crate) fn sync_mail_open_thread_body_list_state(&mut self) {
        let thread_id = self.mail_open_thread_id.clone();
        let row_count = thread_id
            .as_deref()
            .and_then(|thread_id| self.mail_thread_detail_cache.get(thread_id))
            .map(|thread| self.open_thread_body_row_count(thread))
            .unwrap_or(0);
        let width = self.preview_width;
        if self.mail_open_thread_body_list_thread_id != thread_id
            || self.mail_open_thread_body_list_state.item_count() != row_count
        {
            self.mail_open_thread_body_list_state.reset(row_count);
            self.mail_open_thread_body_list_thread_id = thread_id;
            self.mail_open_thread_body_list_width = width;
        } else if row_count > 0 && (self.mail_open_thread_body_list_width - width).abs() > 0.5 {
            self.mail_open_thread_body_list_state.remeasure();
            self.mail_open_thread_body_list_width = width;
        }
    }

    pub(crate) fn remeasure_mail_open_thread_body_list_state(&mut self) {
        self.mail_open_thread_body_list_state.remeasure();
    }

    pub(crate) fn open_thread_inline_compose_visible(&self, thread_id: &str) -> bool {
        matches!(
            self.mail_compose_mode,
            MailComposeMode::Reply | MailComposeMode::ReplyAll | MailComposeMode::Forward
        ) && self.mail_compose_bound_thread_id.as_deref() == Some(thread_id)
    }

    pub(crate) fn open_message_card_row_count(
        &self,
        thread_id: &str,
        message: &MailThreadMessageDetail,
        include_composer: bool,
    ) -> usize {
        let base_row_count = mail_message_card_row_count(message);
        if include_composer && self.open_thread_inline_compose_visible(thread_id) {
            base_row_count + 1
        } else {
            base_row_count
        }
    }

    pub(crate) fn open_thread_body_row_count(&self, thread: &MailThreadDetail) -> usize {
        thread
            .message_details
            .iter()
            .enumerate()
            .map(|(index, message)| self.open_thread_message_row_count(thread, index, message))
            .sum()
    }

    fn open_thread_message_row_count(
        &self,
        thread: &MailThreadDetail,
        index: usize,
        message: &MailThreadMessageDetail,
    ) -> usize {
        let latest = index + 1 == thread.message_details.len();
        if latest
            || self
                .mail_expanded_history_message_ids
                .contains(message.id.as_str())
        {
            return self.open_message_card_row_count(thread.id.as_str(), message, latest);
        }
        1
    }

    pub(crate) fn ensure_mail_surface_state(&mut self, cx: &mut Context<Self>) {
        if self.mail_surface_visible() {
            self.ensure_mail_identity_loaded(cx);
            if self.mail_selected_tab_id.is_empty() {
                if let Some(mailbox_id) = self
                    .mail_workspace()
                    .map(|workspace| workspace.selected_mailbox_id.clone())
                {
                    self.mail_selected_tab_id = mailbox_id.clone();
                    self.mail_list_source = MailListSource::Mailbox(mailbox_id);
                }
            }
            if self.mail_selected_thread_id.is_none() {
                self.mail_selected_thread_id = self
                    .mail_visible_messages()
                    .first()
                    .map(|message| message.thread_id.clone());
            }
            self.ensure_mail_active_thread_detail_cached();
            self.sync_mail_open_thread_body_list_state();
            self.ensure_mail_contact_history(cx);
            self.ensure_mail_remote_image_loads(cx);
            return;
        }
        self.ensure_mail_production_startup(cx);
    }

    pub(crate) fn render_mail(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.mail_surface_visible() {
            return self.render_mail_surface(cx);
        }
        self.render_mail_unavailable(cx)
    }

    pub(crate) fn render_mail_svg_icon(
        &self,
        view_box: &str,
        body: impl Into<String>,
        cx: &mut Context<Self>,
    ) -> Arc<RenderImage> {
        let body = body.into();
        let cache_key = format!("{}:{view_box}{body}", view_box.len());
        if let Some(image) = self.mail_svg_icon_cache.borrow().get(&cache_key).cloned() {
            return image;
        }
        let image = render_svg_image(svg_from_body(view_box, body), cx);
        self.mail_svg_icon_cache
            .borrow_mut()
            .insert(cache_key, image.clone());
        image
    }

    fn render_mail_surface(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let preview_width = self.preview_width.max(MAIL_PREVIEW_MIN_WIDTH);
        let activity_width_ratio = 0.2583;
        let activity_width = (preview_width * activity_width_ratio)
            .clamp(MAIL_ACTIVITY_MIN_WIDTH, MAIL_ACTIVITY_MAX_WIDTH);
        let top_inset = self.mail_surface_top_inset();
        let content = self.render_mail_surface_content(activity_width, cx);
        (div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .overflow_hidden()
            .flex()
            .relative()
            .font(font(MAIL_FONT_FAMILY))
            .bg(rgb(palette.surface_bg))
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event, window, cx| {
                if this.handle_mail_key_down(event, cx) {
                    if this.mail_account_palette.is_some() {
                        cx.focus_self(window);
                    }
                    cx.stop_propagation();
                }
            }))
            .pt(px(top_inset))
            .when(top_inset > 0.0, |this| {
                this.child(self.render_mail_activity_backdrop(activity_width))
            })
            .child(content)
            .when(self.mail_folder_drawer_open, |this| {
                this.child(self.render_mail_folder_drawer(cx))
            })
            .when_some(self.mail_action_palette.as_ref(), |this, state| {
                this.child(self.render_mail_action_palette(state, cx))
            })
            .when_some(self.mail_account_palette.as_ref(), |this, state| {
                this.child(self.render_mail_account_palette(state, cx))
            })
            .when_some(self.mail_sign_in.as_ref(), |this, state| {
                this.child(self.render_mail_sign_in_overlay(state, cx))
            })
            .when(self.mail_split_settings.is_some(), |this| {
                this.child(self.render_mail_split_settings(cx))
            })
            .when_some(self.mail_error.as_deref(), |this, error| {
                this.child(self.render_mail_error_toast(error))
            }))
        .into_any_element()
    }

    fn render_mail_surface_content(
        &self,
        activity_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.mail_inbox_zero_visible() {
            return self.render_mail_inbox_zero_surface(activity_width, cx);
        }
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .child(self.render_mail_shell(cx))
            .child(self.render_mail_surface_sidebar(activity_width, cx))
            .into_any_element()
    }

    fn render_mail_surface_sidebar(
        &self,
        activity_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let search_context = self.mail_search_context_thread_detail();
        if self.mail_switching_account_id.is_some() {
            self.render_mail_account_switching_sidebar(activity_width)
                .into_any_element()
        } else if self.mail_compose_mode != MailComposeMode::Closed {
            self.render_mail_compose_sidebar(activity_width, cx)
                .into_any_element()
        } else if self.mail_search_input_visible() && search_context.is_none() {
            self.render_mail_search_tips_sidebar(activity_width, cx)
                .into_any_element()
        } else if let Some(thread) = search_context {
            self.render_mail_contact_sidebar(activity_width, &thread, cx)
                .into_any_element()
        } else if self.mail_search_open() && self.mail_search_input_visible() {
            self.render_mail_search_blank_sidebar(activity_width)
                .into_any_element()
        } else {
            self.render_mail_activity_sidebar(activity_width, cx)
                .into_any_element()
        }
    }
}
