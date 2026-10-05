use super::{Context, SurfaceState};

impl SurfaceState {
    pub(crate) fn toggle_mail_folder_drawer(&mut self, cx: &mut Context<Self>) -> bool {
        self.mail_folder_drawer_open = !self.mail_folder_drawer_open;
        self.mail_action_palette = None;
        self.mail_account_palette = None;
        self.mail_hovered_thread_id = None;
        self.mail_shortcuts_focused = true;
        cx.notify();
        true
    }

    pub(crate) fn close_mail_folder_drawer(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.mail_folder_drawer_open {
            return false;
        }
        self.mail_folder_drawer_open = false;
        cx.notify();
        true
    }
}
