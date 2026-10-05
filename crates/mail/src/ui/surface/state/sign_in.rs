use std::rc::Rc;

use super::super::{MailSignInField, MailSignInPending, MailSignInState};
use crate::model::MailSignInRequest;
use crate::ui::surface::{MailPalette, MAIL_FONT_FAMILY};
use crate::ui::{
    alpha, mail_palette, px, rgb, AppContext, Context, Entity, SurfaceState, TextInput,
    TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

#[derive(Clone, Copy)]
enum MailSignInInputAction {
    Submit,
    Next,
    Cancel,
    Focus(MailSignInField),
}

impl SurfaceState {
    pub(crate) fn open_mail_sign_in(&mut self, dismissible: bool, cx: &mut Context<Self>) {
        self.mail_account_palette = None;
        self.mail_sign_in_inputs.borrow_mut().clear();
        self.mail_sign_in = Some(MailSignInState {
            dismissible,
            ..MailSignInState::default()
        });
        cx.notify();
    }

    pub(crate) fn close_mail_sign_in(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(state) = self.mail_sign_in.as_ref() else {
            return false;
        };
        if !state.dismissible {
            return false;
        }
        self.mail_sign_in = None;
        self.mail_sign_in_generation += 1;
        self.mail_sign_in_inputs.borrow_mut().clear();
        cx.notify();
        true
    }

    pub(crate) fn start_mail_superhuman_sign_in(&mut self, cx: &mut Context<Self>) {
        if self.mail_sign_in.is_none() {
            let dismissible = self.mail_surface_visible();
            self.open_mail_sign_in(dismissible, cx);
        }
        self.run_mail_sign_in(
            MailSignInPending::Superhuman,
            MailSignInRequest::Superhuman,
            cx,
        );
    }

    pub(crate) fn submit_mail_jmap_sign_in(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.mail_sign_in.as_ref() else {
            return;
        };
        let request = MailSignInRequest::Jmap {
            server: state.server.clone(),
            token: state.token.clone(),
        };
        self.run_mail_sign_in(MailSignInPending::Jmap, request, cx);
    }

    fn run_mail_sign_in(
        &mut self,
        pending: MailSignInPending,
        request: MailSignInRequest,
        cx: &mut Context<Self>,
    ) {
        let Some(bootstrap_api) = self.mail_bootstrap_api.clone() else {
            return;
        };
        let Some(state) = self.mail_sign_in.as_mut() else {
            return;
        };
        if state.pending.is_some() {
            return;
        }
        state.pending = Some(pending);
        state.error = None;
        self.mail_sign_in_generation += 1;
        let generation = self.mail_sign_in_generation;
        cx.notify();
        self.spawn_background_task(
            (bootstrap_api, request),
            cx,
            |(bootstrap_api, request)| bootstrap_api.sign_in(request),
            move |this, result, cx| {
                if this.mail_sign_in_generation != generation {
                    return;
                }
                match result {
                    Ok(()) => this.restart_mail_production_startup(cx),
                    Err(error) => {
                        if let Some(state) = this.mail_sign_in.as_mut() {
                            state.pending = None;
                            state.error = Some(error);
                        }
                        cx.notify();
                    }
                }
            },
        );
    }

    fn set_mail_sign_in_value(&mut self, field: MailSignInField, value: String) {
        let Some(state) = self.mail_sign_in.as_mut() else {
            return;
        };
        match field {
            MailSignInField::Server => state.server = value,
            MailSignInField::Token => state.token = value,
        }
        state.error = None;
    }

    fn focus_mail_sign_in_field(&mut self, field: MailSignInField, cx: &mut Context<Self>) {
        if let Some(state) = self.mail_sign_in.as_mut() {
            if state.focused_field != field {
                state.focused_field = field;
                cx.notify();
            }
        }
    }

    pub(crate) fn mail_sign_in_input_entity(
        &self,
        field: MailSignInField,
        cx: &mut Context<Self>,
    ) -> Entity<TextInput> {
        let props = self.mail_sign_in_input_props(field, cx);
        if let Some(input) = self.mail_sign_in_inputs.borrow().get(&field).cloned() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.mail_sign_in_inputs
            .borrow_mut()
            .insert(field, input.clone());
        input
    }

    fn mail_sign_in_input_props(
        &self,
        field: MailSignInField,
        cx: &mut Context<Self>,
    ) -> TextInputProps {
        let palette = mail_palette(self.appearance_mode);
        let state = self
            .mail_sign_in
            .as_ref()
            .expect("sign-in inputs render only while signing in");
        let (value, placeholder, next) = match field {
            MailSignInField::Server => (
                state.server.clone(),
                "Server, such as api.fastmail.com",
                MailSignInInputAction::Next,
            ),
            MailSignInField::Token => (
                state.token.clone(),
                "API token",
                MailSignInInputAction::Submit,
            ),
        };
        TextInputProps::single_line(value)
            .placeholder(placeholder)
            .obscured(field == MailSignInField::Token)
            .request_focus(state.focused_field == field && state.pending.is_none())
            .style(mail_sign_in_input_style(&palette))
            .on_change(self.mail_sign_in_input_change(field, cx))
            .on_submit(self.mail_sign_in_input_action(next, cx))
            .on_tab(self.mail_sign_in_input_action(MailSignInInputAction::Next, cx))
            .on_escape(self.mail_sign_in_input_action(MailSignInInputAction::Cancel, cx))
            .on_focus(self.mail_sign_in_input_action(MailSignInInputAction::Focus(field), cx))
    }

    fn mail_sign_in_input_change(
        &self,
        field: MailSignInField,
        cx: &mut Context<Self>,
    ) -> TextInputChange {
        let surface = cx.entity();
        Rc::new(move |value, _window, cx| {
            surface.update(cx, |surface, _cx| {
                surface.set_mail_sign_in_value(field, value);
            });
        })
    }

    fn mail_sign_in_input_action(
        &self,
        action: MailSignInInputAction,
        cx: &mut Context<Self>,
    ) -> TextInputAction {
        let surface = cx.entity();
        Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| match action {
                MailSignInInputAction::Submit => surface.submit_mail_jmap_sign_in(cx),
                MailSignInInputAction::Next => {
                    let next = match surface.mail_sign_in.as_ref().map(|s| s.focused_field) {
                        Some(MailSignInField::Server) => MailSignInField::Token,
                        _ => MailSignInField::Server,
                    };
                    surface.focus_mail_sign_in_field(next, cx);
                }
                MailSignInInputAction::Cancel => {
                    surface.close_mail_sign_in(cx);
                }
                MailSignInInputAction::Focus(field) => surface.focus_mail_sign_in_field(field, cx),
            });
        })
    }
}

fn mail_sign_in_input_style(palette: &MailPalette) -> TextInputStyle {
    TextInputStyle {
        height: px(38.0),
        min_height: px(38.0),
        padding_x: px(12.0),
        padding_y: px(10.0),
        radius: px(4.0),
        background: alpha(palette.text_rgb, 0.04),
        border: alpha(palette.text_rgb, 0.14),
        focused_border: alpha(palette.compose_caret, 0.8),
        text: alpha(palette.text_rgb, 0.88),
        placeholder: alpha(palette.text_rgb, 0.36),
        selection: alpha(palette.compose_caret, 0.28),
        caret: rgb(palette.compose_caret).into(),
        font_size: px(13.0),
        line_height: px(18.0),
        font_family: Some(MAIL_FONT_FAMILY.into()),
    }
}
