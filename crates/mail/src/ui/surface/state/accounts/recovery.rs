use super::super::{Context, SurfaceState};
use crate::model::{MailAccountLoadError, MailAccountWorkspace};

impl SurfaceState {
    pub(crate) fn handle_mail_workspace_error(
        &mut self,
        message: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.report_mail_error(message);
        if self.mail_account_recovery_in_flight || self.mail_switching_account_id.is_some() {
            cx.notify();
            return;
        }
        let Some(workspace_api) = self.mail_workspace_api() else {
            cx.notify();
            return;
        };
        let Some(revoked_account_id) = self
            .mail_workspace()
            .map(|workspace| workspace.account_id.clone())
        else {
            cx.notify();
            return;
        };
        self.mail_account_recovery_in_flight = true;
        cx.notify();
        self.spawn_background_task(
            workspace_api,
            cx,
            move |workspace_api| workspace_api.recover_mail_account_access(),
            move |this, result, cx| {
                this.apply_mail_account_recovery(revoked_account_id, result, cx);
            },
        );
    }

    fn apply_mail_account_recovery(
        &mut self,
        revoked_account_id: String,
        result: Result<Option<MailAccountWorkspace>, MailAccountLoadError>,
        cx: &mut Context<Self>,
    ) {
        self.mail_account_recovery_in_flight = false;
        let Ok(Some(account_workspace)) = result else {
            cx.notify();
            return;
        };
        if self.mail_account_switch_blocked() {
            self.mail_error = Some(
                "Access removed. Finish copying or discard the current draft before switching accounts"
                    .to_string(),
            );
            cx.notify();
            return;
        }
        let fallback_account_id = account_workspace.workspace.account_id.clone();
        self.mail_account_generation = self.mail_account_generation.wrapping_add(1);
        self.mail_account_view_states
            .remove(revoked_account_id.as_str());
        self.mail_switching_account_id = Some(fallback_account_id.clone());
        self.apply_loaded_mail_account(fallback_account_id, account_workspace, cx);
        self.mail_error = Some("Access removed. Switched back to an available account".to_string());
        cx.notify();
    }
}
