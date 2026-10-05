use super::{Context, SurfaceState};
use crate::ui::surface::MailStartup;

impl SurfaceState {
    pub(crate) fn ensure_mail_production_startup(&mut self, cx: &mut Context<Self>) {
        let bootstrap_api = match &mut self.mail_startup {
            MailStartup::Loading {
                bootstrap_api,
                request_started,
            } if !*request_started => {
                *request_started = true;
                Some(bootstrap_api.clone())
            }
            #[cfg(any(test, feature = "test-support"))]
            MailStartup::Fixture => None,
            MailStartup::Loading { .. }
            | MailStartup::Ready(_)
            | MailStartup::Error { .. }
            | MailStartup::SignedOut => None,
        };
        let Some(bootstrap_api) = bootstrap_api else {
            return;
        };
        self.mail_bootstrap_api = Some(bootstrap_api.clone());
        cx.notify();
        self.spawn_background_task(
            bootstrap_api,
            cx,
            move |bootstrap_api| {
                if !bootstrap_api.has_mail_accounts() {
                    return Ok(None);
                }
                bootstrap_api.bootstrap_workspace().map(Some)
            },
            |this, result, cx| match result {
                Ok(Some(bootstrap)) => this.apply_mail_workspace_bootstrap(bootstrap, cx),
                Ok(None) => this.apply_mail_signed_out(cx),
                Err(error) => this.apply_mail_startup_error(error, cx),
            },
        );
    }

    /// Loads mail again from scratch, as after adding an account.
    pub(crate) fn restart_mail_production_startup(&mut self, cx: &mut Context<Self>) {
        let Some(bootstrap_api) = self.mail_bootstrap_api.clone() else {
            return;
        };
        self.mail_sign_in = None;
        self.mail_sign_in_inputs.borrow_mut().clear();
        self.mail_account_palette = None;
        self.mail_error = None;
        self.mail_startup = MailStartup::Loading {
            bootstrap_api,
            request_started: false,
        };
        self.ensure_mail_production_startup(cx);
    }

    fn apply_mail_signed_out(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.mail_startup, MailStartup::Loading { .. }) {
            return;
        }
        self.mail_startup = MailStartup::SignedOut;
        self.open_mail_sign_in(false, cx);
    }

    pub(crate) fn retry_mail_production_startup(&mut self, cx: &mut Context<Self>) {
        let MailStartup::Error { bootstrap_api, .. } = &self.mail_startup else {
            return;
        };
        let bootstrap_api = bootstrap_api.clone();
        self.mail_startup = MailStartup::Loading {
            bootstrap_api,
            request_started: false,
        };
        self.mail_error = None;
        self.ensure_mail_production_startup(cx);
    }

    pub(crate) fn apply_mail_workspace_bootstrap(
        &mut self,
        bootstrap: crate::model::MailWorkspaceBootstrap,
        cx: &mut Context<Self>,
    ) {
        let split_preferences_supported = bootstrap.workspace_api.supports_mail_split_preferences();
        let split_definitions = if split_preferences_supported {
            match bootstrap.workspace_api.load_mail_split_definitions() {
                Ok(definitions) => definitions,
                Err(error) => {
                    self.apply_mail_startup_error(error, cx);
                    return;
                }
            }
        } else {
            Vec::new()
        };
        let needs_initial_refresh = bootstrap.needs_initial_refresh;
        let workspace = bootstrap.workspace.clone();
        self.mail_startup = MailStartup::Ready(Box::new(bootstrap));
        #[cfg(any(test, feature = "test-support"))]
        {
            self.fixture_mail_workspace = None;
            self.fixture_mail_workspace_api = None;
        }
        self.mail_thread_cache.clear();
        self.mail_thread_detail_cache.clear();
        self.clear_mail_contact_histories();
        self.mail_open_thread_body_list_thread_id = None;
        self.mail_loading_thread_id = None;
        self.mail_loading_message_page = None;
        self.clear_mail_remote_images();
        self.mail_identity = None;
        self.mail_identity_loading = false;
        self.mail_action_palette = None;
        self.mail_account_palette = None;
        self.mail_switching_account_id = None;
        self.mail_account_view_states.clear();
        self.mail_split_preferences_supported = split_preferences_supported;
        self.mail_split_definitions = split_definitions;
        self.mail_split_settings = None;
        self.mail_split_text_inputs.borrow_mut().clear();
        self.mail_error = None;
        self.clear_mail_compose_state();
        self.apply_mail_workspace_refresh(workspace, cx);
        if needs_initial_refresh {
            let mailbox_id = self.mail_selected_tab_id.clone();
            if !mailbox_id.is_empty() {
                self.refresh_mail_mailbox_in_background(mailbox_id, cx);
            }
        }
        self.ensure_mail_identity_loaded(cx);
    }

    fn apply_mail_startup_error(&mut self, error: String, cx: &mut Context<Self>) {
        let MailStartup::Loading { bootstrap_api, .. } = &self.mail_startup else {
            return;
        };
        let bootstrap_api = bootstrap_api.clone();
        report_mail_startup_error(&error);
        self.mail_error = Some(error.clone());
        self.mail_startup = MailStartup::Error {
            bootstrap_api,
            error,
        };
        cx.notify();
    }
}

fn report_mail_startup_error(error: impl AsRef<str>) {
    eprintln!("mail startup: {}", error.as_ref());
}
