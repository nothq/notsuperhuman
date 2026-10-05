use super::{
    keystroke_input_text, Context, KeyDownEvent, MailComposeField, MailComposeMode, MailRowAction,
    MailTriageControl, SurfaceState,
};
use crate::ui::offset_index;

impl SurfaceState {
    pub(crate) fn handle_mail_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.mail_surface_visible() {
            return self.handle_mail_unavailable_key_down(event, cx);
        }
        if self.mail_sign_in.is_some() {
            return event.keystroke.key == "escape" && self.close_mail_sign_in(cx);
        }
        if self.mail_split_settings.is_some() {
            if event.keystroke.key == "escape" {
                if !self.cancel_mail_split_editor(cx) {
                    self.close_mail_split_settings(cx);
                }
                return true;
            }
            return false;
        }
        if self.handle_mail_account_shortcut(event, cx) {
            return true;
        }
        if self.mail_account_palette.is_some() {
            return self.handle_mail_account_palette_key_down(event, cx);
        }
        if self.mail_folder_drawer_open {
            return self.handle_mail_folder_drawer_key_down(event, cx);
        }
        if self.mail_compose_mode != MailComposeMode::Closed {
            if let Some(handled) = self.handle_mail_compose_key_down(event, cx) {
                return handled;
            }
            if !self.mail_shortcuts_focused {
                return false;
            }
        }
        if self.mail_action_palette.is_some() {
            return self.handle_mail_action_palette_key_down(event, cx);
        }
        if self.mail_search_input_focused() {
            return false;
        }
        self.handle_mail_navigation_key_down(event, cx)
    }

    fn handle_mail_account_shortcut(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = &event.keystroke.modifiers;
        if !modifiers.control
            || modifiers.alt
            || modifiers.shift
            || modifiers.platform
            || modifiers.function
        {
            return false;
        }
        match event.keystroke.key.as_str() {
            "0" => self.open_mail_account_palette(cx),
            key if matches!(key, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9") => {
                let slot = key.parse::<usize>().expect("digit key must parse");
                self.switch_mail_account_slot(slot, cx)
            }
            _ => false,
        }
    }

    fn handle_mail_account_palette_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        match event.keystroke.key.as_str() {
            "escape" => self.close_mail_account_palette(cx),
            "up" => self.move_mail_account_palette_selection(-1, cx),
            "down" => self.move_mail_account_palette_selection(1, cx),
            "k" if mail_account_palette_control_navigation(event) => {
                self.move_mail_account_palette_selection(-1, cx)
            }
            "j" if mail_account_palette_control_navigation(event) => {
                self.move_mail_account_palette_selection(1, cx)
            }
            "enter" => self.activate_mail_account_palette(cx),
            "backspace" => self.backspace_mail_account_palette_query(cx),
            _ => keystroke_input_text(event)
                .map(|value| self.append_mail_account_palette_query(value, cx))
                .unwrap_or(false),
        }
    }

    fn handle_mail_folder_drawer_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if event.keystroke.key == "escape" {
            self.close_mail_folder_drawer(cx);
        }
        true
    }

    fn handle_mail_unavailable_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if event.keystroke.key.eq_ignore_ascii_case("r")
            && matches!(
                self.mail_startup,
                crate::ui::surface::MailStartup::Error { .. }
            )
        {
            self.retry_mail_production_startup(cx);
            return true;
        }
        false
    }

    fn handle_mail_navigation_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        match event.keystroke.key.as_str() {
            "escape" => self.handle_mail_escape(cx),
            "enter" => {
                let Some(thread_id) = self.mail_selected_thread_id.clone() else {
                    return false;
                };
                self.open_mail_thread(thread_id, cx);
                true
            }
            "up" | "k" => self.select_adjacent_mail_thread(-1, cx),
            "down" | "j" => self.select_adjacent_mail_thread(1, cx),
            "left" if !self.mail_search_open() => self.select_adjacent_mail_tab(-1, cx),
            "right" if !self.mail_search_open() => self.select_adjacent_mail_tab(1, cx),
            "left" | "right" => false,
            "c" => {
                self.open_new_mail_composer(cx);
                true
            }
            "r" => self.reply_mail_thread(cx),
            "a" => self.reply_all_mail_thread(cx),
            "f" => self.forward_mail_thread(cx),
            "e" if self
                .mail_active_message()
                .is_some_and(|message| message.is_draft) =>
            {
                self.edit_mail_draft(cx)
            }
            "e" => self.archive_selected_mail_thread(cx),
            "h" => self.handle_active_mail_thread_action(MailRowAction::RemindMe, cx),
            "v" => self.handle_active_mail_thread_action(MailRowAction::Move, cx),
            "s" => self.toggle_active_mail_thread_triage(MailTriageControl::Star, cx),
            "u" => self.toggle_active_mail_thread_triage(MailTriageControl::Read, cx),
            "/" | "slash" => self.open_mail_search(cx),
            _ => false,
        }
    }

    fn handle_mail_action_palette_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        match event.keystroke.key.as_str() {
            "escape" => self.close_mail_action_palette(cx),
            "up" | "k" => self.move_mail_action_palette_selection(-1, cx),
            "down" | "j" => self.move_mail_action_palette_selection(1, cx),
            "enter" => self.activate_mail_action_palette(cx),
            _ => keystroke_input_text(event).is_some(),
        }
    }

    fn handle_mail_compose_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> Option<bool> {
        let autocomplete_visible = self.mail_compose_recipient_field_focused()
            && !self.mail_compose_autocomplete_items().is_empty();
        match event.keystroke.key.as_str() {
            "escape" => Some(self.close_mail_compose_view(cx)),
            "up" if autocomplete_visible => Some(self.move_mail_compose_autocomplete(-1, cx)),
            "down" if autocomplete_visible => Some(self.move_mail_compose_autocomplete(1, cx)),
            "tab" | "enter" if autocomplete_visible => {
                Some(self.accept_mail_compose_autocomplete(cx))
            }
            _ => None,
        }
    }

    fn mail_compose_recipient_field_focused(&self) -> bool {
        matches!(
            self.mail_compose_focused_field,
            MailComposeField::To | MailComposeField::Cc
        )
    }

    fn handle_mail_escape(&mut self, cx: &mut Context<Self>) -> bool {
        if self.close_mail_folder_drawer(cx) {
            return true;
        }
        if self.mail_open_thread_id.take().is_some() {
            self.reset_mail_open_thread_read_session();
            self.mail_remote_image_discovery_thread_id = None;
            self.mail_hovered_thread_id = None;
            self.mail_hovered_message_card_id = None;
            self.mail_hovered_message_action = None;
            self.mail_hovered_history_message_id = None;
            self.mail_expanded_history_message_ids.clear();
            self.mail_expanded_message_header_ids.clear();
            cx.notify();
            return true;
        }
        if self.mail_search_open() {
            return self.close_mail_search(cx);
        }
        if self.mail_error.take().is_some() {
            cx.notify();
            return true;
        }
        false
    }

    pub(crate) fn select_adjacent_mail_thread(
        &mut self,
        delta: isize,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.mail_list_threads.is_empty() {
            return false;
        }
        let current_index = self
            .mail_selected_thread_id
            .as_deref()
            .and_then(|thread_id| {
                self.mail_list_threads
                    .iter()
                    .position(|thread| thread.id == thread_id)
            })
            .unwrap_or(0);
        let next_index = offset_index(current_index, delta, self.mail_list_threads.len());
        let next_thread = self
            .mail_list_threads
            .get(next_index)
            .map(|thread| thread.id.clone())
            .unwrap_or_else(|| self.mail_list_threads[0].id.clone());
        self.mail_selected_thread_id = Some(next_thread.clone());
        self.mail_hovered_thread_id = None;
        self.mail_hovered_message_card_id = None;
        self.mail_hovered_message_action = None;
        self.mail_hovered_history_message_id = None;
        self.mail_expanded_history_message_ids.clear();
        self.mail_expanded_message_header_ids.clear();
        if self.mail_open_thread_id.is_some() {
            self.reset_mail_open_thread_read_session();
            self.mail_open_thread_id = Some(next_thread.clone());
            self.mail_remote_image_discovery_thread_id = None;
            self.mark_mail_thread_read_on_open(next_thread.as_str(), cx);
            self.load_mail_thread_in_background(next_thread, cx);
        }
        cx.notify();
        true
    }

    fn select_adjacent_mail_tab(&mut self, delta: isize, cx: &mut Context<Self>) -> bool {
        if self.mail_workspace().is_none() {
            return false;
        }
        let tabs = self.mail_tabs();
        if tabs.is_empty() {
            return false;
        }
        let current_index = tabs
            .iter()
            .position(|tab| tab.source == self.mail_list_source)
            .unwrap_or(0);
        let next_index = offset_index(current_index, delta, tabs.len());
        let next_source = tabs
            .get(next_index)
            .map(|tab| tab.source.clone())
            .unwrap_or_else(|| tabs[0].source.clone());
        self.set_mail_list_source(next_source, cx);
        true
    }
}

fn mail_account_palette_control_navigation(event: &KeyDownEvent) -> bool {
    let modifiers = &event.keystroke.modifiers;
    modifiers.control
        && !modifiers.alt
        && !modifiers.shift
        && !modifiers.platform
        && !modifiers.function
}
