use std::rc::Rc;

use super::super::super::MAIL_SEARCH_FONT_FAMILY;
use crate::ui::{
    alpha, mail_palette, px, rgb, AppContext, Context, Entity, HashSet, MailAddress, SurfaceState,
    TextInput, TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

impl SurfaceState {
    pub(crate) fn mail_search_contacts(&self) -> Vec<MailAddress> {
        let Some(workspace) = self.mail_workspace() else {
            return Vec::new();
        };
        let owner = workspace
            .mailbox_email
            .as_deref()
            .map(str::to_ascii_lowercase);
        let filter = self
            .mail_search_session()
            .map(|session| session.raw_query.trim().to_lowercase())
            .unwrap_or_default();
        let result_limit = if filter.is_empty() { 3 } else { 5 };
        let mut seen = HashSet::new();
        let mut contacts = Vec::new();
        for message in &workspace.messages {
            for address in message
                .from
                .iter()
                .chain(message.to.iter())
                .chain(message.cc.iter())
                .chain(message.bcc.iter())
            {
                let email_key = address.email.to_ascii_lowercase();
                if email_key.is_empty() || owner.as_deref() == Some(email_key.as_str()) {
                    continue;
                }
                if !filter.is_empty()
                    && !address.name.to_lowercase().contains(filter.as_str())
                    && !address.email.to_lowercase().contains(filter.as_str())
                {
                    continue;
                }
                if !seen.insert(email_key) {
                    continue;
                }
                contacts.push(address.clone());
                if contacts.len() == result_limit {
                    return contacts;
                }
            }
        }
        contacts
    }

    pub(crate) fn mail_search_completion(&self) -> Option<(String, String)> {
        let session = self.mail_search_session()?;
        let query = session.raw_query.as_str();
        if session.submitted_query.is_some() || query.is_empty() {
            return None;
        }
        let contact = self.mail_search_contacts().into_iter().next()?;
        if !contact.name.is_empty() {
            if let Some(prefix_end) =
                mail_search_case_insensitive_prefix_end(contact.name.as_str(), query)
            {
                return Some((
                    query.to_string(),
                    format!("{} ⇥ {}", &contact.name[prefix_end..], contact.email),
                ));
            }
        }
        let prefix_end = mail_search_case_insensitive_prefix_end(&contact.email, query)?;
        (prefix_end < contact.email.len())
            .then(|| (query.to_string(), contact.email[prefix_end..].to_string()))
    }

    pub(crate) fn accept_mail_search_completion(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session) = self.mail_search_session() else {
            return false;
        };
        if session.submitted_query.is_some() || session.raw_query.trim().is_empty() {
            return false;
        }
        if self.mail_search_completion().is_none() {
            return false;
        }
        let Some(contact) = self.mail_search_contacts().into_iter().next() else {
            return false;
        };
        self.set_mail_search_input(format!("{} ", contact.email), cx);
        true
    }

    pub(crate) fn mail_search_input_entity(&self, cx: &mut Context<Self>) -> Entity<TextInput> {
        let props = self.mail_search_input_props(cx);
        if let Some(input) = self.mail_search_input.borrow().clone() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        *self.mail_search_input.borrow_mut() = Some(input.clone());
        input
    }

    fn mail_search_input_props(&self, cx: &mut Context<Self>) -> TextInputProps {
        let palette = mail_palette(self.appearance_mode);
        let value = self
            .mail_search_session()
            .map(|session| session.raw_query.clone())
            .unwrap_or_default();
        let request_focus = self
            .mail_search_session()
            .is_some_and(|session| session.input_focused);
        let text_color = if self.mail_search_completion().is_some() {
            alpha(palette.text_rgb, 0.0)
        } else {
            rgb(palette.text_rgb).into()
        };
        TextInputProps::single_line(value)
            .placeholder("Search")
            .request_focus(request_focus)
            .style(TextInputStyle {
                height: px(28.0),
                min_height: px(28.0),
                padding_x: px(0.0),
                padding_y: px(0.0),
                radius: px(0.0),
                background: alpha(palette.shell_bg, 0.0),
                border: alpha(palette.shell_bg, 0.0),
                focused_border: alpha(palette.shell_bg, 0.0),
                text: text_color,
                placeholder: alpha(palette.text_rgb, 0.7),
                selection: alpha(0x8e82ba, 0.35),
                caret: rgb(0xa69bcf).into(),
                font_size: px(16.0),
                line_height: px(28.0),
                font_family: Some(MAIL_SEARCH_FONT_FAMILY.into()),
            })
            .on_change(self.mail_search_on_change(cx))
            .on_submit(self.mail_search_on_submit(cx))
            .on_escape(self.mail_search_on_escape(cx))
            .on_tab(self.mail_search_on_tab(cx))
            .on_focus(self.mail_search_on_focus(cx))
    }

    fn mail_search_on_change(&self, cx: &mut Context<Self>) -> TextInputChange {
        let surface = cx.entity();
        Rc::new(move |value, _window, cx| {
            surface.update(cx, |surface, cx| {
                surface.set_mail_search_input(value, cx);
            });
        })
    }

    fn mail_search_on_submit(&self, cx: &mut Context<Self>) -> TextInputAction {
        let surface = cx.entity();
        Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                surface.submit_mail_search(cx);
            });
        })
    }

    fn mail_search_on_escape(&self, cx: &mut Context<Self>) -> TextInputAction {
        let surface = cx.entity();
        Rc::new(move |window, cx| {
            surface.update(cx, |surface, cx| {
                surface.close_mail_search(cx);
                cx.focus_self(window);
            });
        })
    }

    fn mail_search_on_focus(&self, cx: &mut Context<Self>) -> TextInputAction {
        let surface = cx.entity();
        Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                if let Some(session) = surface.mail_search_state.session_mut() {
                    session.input_focused = true;
                }
                surface.mail_shortcuts_focused = false;
                cx.notify();
            });
        })
    }

    fn mail_search_on_tab(&self, cx: &mut Context<Self>) -> TextInputAction {
        let surface = cx.entity();
        Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                surface.accept_mail_search_completion(cx);
            });
        })
    }
}

fn mail_search_case_insensitive_prefix_end(value: &str, prefix: &str) -> Option<usize> {
    let prefix_character_count = prefix.chars().count();
    let end = value
        .char_indices()
        .map(|(byte_offset, _)| byte_offset)
        .chain(std::iter::once(value.len()))
        .nth(prefix_character_count)?;
    (value[..end].to_lowercase() == prefix.to_lowercase()).then_some(end)
}
