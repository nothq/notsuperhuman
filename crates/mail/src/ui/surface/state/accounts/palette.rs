use super::super::{Context, MailAccountPaletteState, ScrollHandle, SurfaceState};
use super::catalog::sorted_mail_accounts;
use crate::model::MailAccountInfo;

impl SurfaceState {
    pub(crate) fn open_mail_account_palette(&mut self, cx: &mut Context<Self>) -> bool {
        let options = sorted_mail_accounts(self.mail_account_catalog());
        if options.is_empty() {
            return false;
        }
        let active_account_id = self
            .mail_workspace()
            .map(|workspace| workspace.account_id.as_str());
        let selected_index = options
            .iter()
            .position(|account| Some(account.id.as_str()) == active_account_id)
            .unwrap_or(0);
        self.mail_account_palette = Some(MailAccountPaletteState {
            query: String::new(),
            selected_index,
            error: None,
        });
        self.mail_folder_drawer_open = false;
        self.mail_hovered_thread_id = None;
        self.mail_account_palette_scroll = ScrollHandle::new();
        self.scroll_mail_account_palette_selection(&options, selected_index);
        self.mail_action_palette = None;
        self.mail_shortcuts_focused = true;
        cx.notify();
        true
    }

    pub(crate) fn close_mail_account_palette(&mut self, cx: &mut Context<Self>) -> bool {
        if self.mail_account_palette.take().is_none() {
            return false;
        }
        cx.notify();
        true
    }

    pub(crate) fn move_mail_account_palette_selection(
        &mut self,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        let options = self.mail_account_palette_options();
        if options.is_empty() {
            return true;
        }
        let current_index = self
            .mail_account_palette
            .as_ref()
            .map(|state| state.selected_index.min(options.len() - 1))
            .unwrap_or(0);
        let selected_index = crate::ui::offset_index(current_index, delta, options.len());
        if let Some(state) = self.mail_account_palette.as_mut() {
            state.selected_index = selected_index;
            state.error = None;
        }
        let accounts = options
            .iter()
            .map(|(_, account)| account.clone())
            .collect::<Vec<_>>();
        self.scroll_mail_account_palette_selection(&accounts, selected_index);
        cx.notify();
        true
    }

    pub(crate) fn append_mail_account_palette_query(
        &mut self,
        value: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = self.mail_account_palette.as_mut() else {
            return false;
        };
        state.query.push_str(value);
        state.selected_index = 0;
        state.error = None;
        let options = self
            .mail_account_palette_options()
            .into_iter()
            .map(|(_, account)| account)
            .collect::<Vec<_>>();
        self.scroll_mail_account_palette_selection(&options, 0);
        cx.notify();
        true
    }

    pub(crate) fn backspace_mail_account_palette_query(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(state) = self.mail_account_palette.as_mut() else {
            return false;
        };
        state.query.pop();
        state.selected_index = 0;
        state.error = None;
        let options = self
            .mail_account_palette_options()
            .into_iter()
            .map(|(_, account)| account)
            .collect::<Vec<_>>();
        self.scroll_mail_account_palette_selection(&options, 0);
        cx.notify();
        true
    }

    pub(crate) fn activate_mail_account_palette(&mut self, cx: &mut Context<Self>) -> bool {
        let options = self.mail_account_palette_options();
        let selected_index = self
            .mail_account_palette
            .as_ref()
            .map(|state| state.selected_index.min(options.len().saturating_sub(1)))
            .unwrap_or(0);
        let Some((_, account)) = options.get(selected_index) else {
            return true;
        };
        self.switch_mail_account(account.id.clone(), cx)
    }

    pub(crate) fn switch_mail_account_slot(&mut self, slot: usize, cx: &mut Context<Self>) -> bool {
        let Some(account) = sorted_mail_accounts(self.mail_account_catalog())
            .get(slot.saturating_sub(1))
            .cloned()
        else {
            return true;
        };
        self.switch_mail_account(account.id, cx)
    }

    pub(super) fn set_mail_account_palette_error(
        &mut self,
        error: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        if self.mail_account_palette.is_none() {
            self.open_mail_account_palette(cx);
        }
        if let Some(state) = self.mail_account_palette.as_mut() {
            state.error = Some(error.into());
        }
        cx.notify();
    }

    fn scroll_mail_account_palette_selection(
        &self,
        options: &[MailAccountInfo],
        selected_index: usize,
    ) {
        let Some(selected) = options.get(selected_index) else {
            return;
        };
        let selected_is_shared = !selected.is_primary && !selected.is_personal;
        let personal_section_precedes = selected_is_shared
            && options
                .iter()
                .any(|account| account.is_primary || account.is_personal);
        let section_titles_before_or_at_selection = 1 + usize::from(personal_section_precedes);
        self.mail_account_palette_scroll
            .scroll_to_item(selected_index + section_titles_before_or_at_selection);
    }
}
