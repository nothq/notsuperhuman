use super::{MailListSource, MailSurfaceStateConfig, SurfaceState};
use crate::ui::*;
impl SurfaceState {
    pub(crate) fn new(config: MailSurfaceStateConfig, cx: &mut Context<Self>) -> Self {
        Self::new_unsynced(config, cx).with_synced_mail_list_rows()
    }

    fn new_unsynced(config: MailSurfaceStateConfig, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            appearance_mode: AppearanceMode::current(cx),
            text_measure: std::sync::Arc::new(|_, _| 0.0),
            theme: config.theme,
            preview_width: config.preview_width,
            viewport_height: config.viewport_height,
            window_controls_visible: config.window_controls_visible,
            chrome_top_inset: config.chrome_top_inset,
            mail_shortcuts_focused: true,
            mail_startup: config.startup,
            #[cfg(any(test, feature = "test-support"))]
            fixture_mail_workspace: config.input.workspace,
            #[cfg(any(test, feature = "test-support"))]
            fixture_mail_workspace_api: config.input.workspace_api,
            mail_local_file_api: config.local_file_api,
            mail_selected_tab_id: default_mail_selected_tab_id(),
            mail_list_source: MailListSource::Mailbox(String::new()),
            mail_account_palette_scroll: ScrollHandle::new(),
            mail_list_threads: Arc::from(Vec::<MailListThread>::new()),
            mail_list_state: Self::build_mail_list_state_for_surface(cx),
            mail_search_list_state: Self::build_initial_mail_search_list_state(cx),
            mail_open_thread_body_list_state: Self::build_mail_open_thread_body_list_state(0),
            mail_compose_focused_field: MailComposeField::To,
            runtime: Default::default(),
        }
    }

    fn with_synced_mail_list_rows(mut self) -> Self {
        self.sync_mail_list_rows();
        self
    }
}
