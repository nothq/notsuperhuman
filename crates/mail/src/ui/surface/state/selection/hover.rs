use super::super::{Context, MailMessageAction, SurfaceState};

impl SurfaceState {
    pub(crate) fn set_mail_thread_hover(
        &mut self,
        thread_id: impl Into<String>,
        is_hovered: bool,
        cx: &mut Context<Self>,
    ) {
        let thread_id = thread_id.into();
        let was_currently_hovered =
            self.mail_hovered_thread_id.as_deref() == Some(thread_id.as_str());
        let next = if is_hovered {
            Some(thread_id.clone())
        } else if was_currently_hovered {
            None
        } else {
            self.mail_hovered_thread_id.clone()
        };
        if self.mail_hovered_thread_id == next {
            return;
        }
        self.mail_hovered_thread_id = next;
        cx.notify();
    }

    pub(crate) fn set_mail_message_card_hover(
        &mut self,
        message_card_id: impl Into<String>,
        is_hovered: bool,
        cx: &mut Context<Self>,
    ) {
        let message_card_id = message_card_id.into();
        let next = if is_hovered {
            Some(message_card_id.clone())
        } else if self.mail_hovered_message_card_id.as_deref() == Some(message_card_id.as_str()) {
            None
        } else {
            self.mail_hovered_message_card_id.clone()
        };
        if self.mail_hovered_message_card_id == next {
            return;
        }
        self.mail_hovered_message_card_id = next;
        cx.notify();
    }

    pub(crate) fn set_mail_message_action_hover(
        &mut self,
        message_card_id: impl Into<String>,
        action: MailMessageAction,
        is_hovered: bool,
        cx: &mut Context<Self>,
    ) {
        let message_card_id = message_card_id.into();
        let next = if is_hovered {
            Some((message_card_id.clone(), action))
        } else if self.mail_hovered_message_action.as_ref().is_some_and(
            |(current_id, current_action)| {
                current_id == &message_card_id && *current_action == action
            },
        ) {
            None
        } else {
            self.mail_hovered_message_action.clone()
        };
        if self.mail_hovered_message_action == next {
            return;
        }
        self.mail_hovered_message_action = next;
        cx.notify();
    }

    pub(crate) fn set_mail_history_message_hover(
        &mut self,
        message_id: impl Into<String>,
        is_hovered: bool,
        cx: &mut Context<Self>,
    ) {
        let message_id = message_id.into();
        let next = if is_hovered {
            Some(message_id.clone())
        } else if self.mail_hovered_history_message_id.as_deref() == Some(message_id.as_str()) {
            None
        } else {
            self.mail_hovered_history_message_id.clone()
        };
        if self.mail_hovered_history_message_id == next {
            return;
        }
        self.mail_hovered_history_message_id = next;
        cx.notify();
    }

    pub(crate) fn expand_mail_history_message(
        &mut self,
        message_id: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let message_id = message_id.into();
        if !self
            .mail_expanded_history_message_ids
            .insert(message_id.clone())
        {
            return;
        }
        if self.mail_hovered_history_message_id.as_deref() == Some(message_id.as_str()) {
            self.mail_hovered_history_message_id = None;
        }
        self.sync_mail_open_thread_body_list_state();
        cx.notify();
    }

    pub(crate) fn toggle_mail_message_header(
        &mut self,
        message_id: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let message_id = message_id.into();
        if !self
            .mail_expanded_message_header_ids
            .remove(message_id.as_str())
        {
            self.mail_expanded_message_header_ids.insert(message_id);
        }
        self.remeasure_mail_open_thread_body_list_state();
        cx.notify();
    }
}
