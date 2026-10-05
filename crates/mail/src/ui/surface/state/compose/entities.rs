use std::rc::Rc;

use super::{Context, MailComposeField, SurfaceState};
use crate::ui::{
    AppContext, Entity, LongFormEditor, LongFormEditorAction, LongFormEditorChange,
    LongFormEditorProps, LongFormEditorStyle, MailComposeTextInputRequest, TextInput,
    TextInputAction, TextInputChange, TextInputProps,
};

impl SurfaceState {
    pub(crate) fn mail_compose_text_input_entity(
        &self,
        request: MailComposeTextInputRequest,
        cx: &mut Context<Self>,
    ) -> Entity<TextInput> {
        let field = request.field;
        let props = self.mail_compose_text_input_props(request, cx);
        if let Some(input) = self.mail_compose_text_inputs.borrow().get(&field).cloned() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.mail_compose_text_inputs
            .borrow_mut()
            .insert(field, input.clone());
        input
    }

    pub(crate) fn mail_compose_body_editor_entity(
        &self,
        value: String,
        placeholder: impl Into<String>,
        style: LongFormEditorStyle,
        cx: &mut Context<Self>,
    ) -> Entity<LongFormEditor> {
        let props = self.mail_compose_body_editor_props(value, placeholder.into(), style, cx);
        if let Some(editor) = self.mail_compose_body_editor.borrow().as_ref().cloned() {
            editor.update(cx, |editor, cx| editor.apply_props(props, cx));
            return editor;
        }
        let editor = cx.new(|cx| LongFormEditor::new(props, cx));
        self.mail_compose_body_editor
            .borrow_mut()
            .replace(editor.clone());
        editor
    }

    fn mail_compose_body_editor_props(
        &self,
        value: String,
        placeholder: String,
        style: LongFormEditorStyle,
        cx: &mut Context<Self>,
    ) -> LongFormEditorProps {
        let surface = cx.entity();
        let on_change: LongFormEditorChange = Rc::new(move |value, _window, cx| {
            surface.update(cx, |surface, cx| {
                surface.set_mail_compose_field_value(MailComposeField::Body, value, cx);
            });
        });
        let surface = cx.entity();
        let on_submit: LongFormEditorAction = Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                surface.send_mail_draft(cx);
            });
        });
        let surface = cx.entity();
        let on_tab: LongFormEditorAction = Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                surface.cycle_mail_compose_field(false, cx);
            });
        });
        let surface = cx.entity();
        let on_focus: LongFormEditorAction = Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                surface.focus_mail_compose_field(MailComposeField::Body, cx);
            });
        });
        LongFormEditorProps::new(value)
            .placeholder(placeholder)
            .style(style)
            .request_focus(self.mail_compose_focused_field == MailComposeField::Body)
            .on_change(on_change)
            .on_submit(on_submit)
            .on_tab(on_tab)
            .on_focus(on_focus)
    }

    fn mail_compose_text_input_props(
        &self,
        request: MailComposeTextInputRequest,
        cx: &mut Context<Self>,
    ) -> TextInputProps {
        let MailComposeTextInputRequest {
            field,
            value,
            placeholder,
            mode,
            style,
        } = request;
        let surface = cx.entity();
        let on_change: TextInputChange = Rc::new(move |value, _window, cx| {
            surface.update(cx, |surface, cx| {
                surface.set_mail_compose_field_value(field, value, cx);
            });
        });
        let surface = cx.entity();
        let on_submit: TextInputAction = Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                if matches!(field, MailComposeField::To | MailComposeField::Cc)
                    && !surface.mail_compose_autocomplete_items().is_empty()
                {
                    surface.accept_mail_compose_autocomplete(cx);
                } else {
                    surface.cycle_mail_compose_field(false, cx);
                }
            });
        });
        let surface = cx.entity();
        let on_tab: TextInputAction = Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                if matches!(field, MailComposeField::To | MailComposeField::Cc)
                    && !surface.mail_compose_autocomplete_items().is_empty()
                {
                    surface.accept_mail_compose_autocomplete(cx);
                } else {
                    surface.cycle_mail_compose_field(false, cx);
                }
            });
        });
        let surface = cx.entity();
        let on_focus: TextInputAction = Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| {
                surface.focus_mail_compose_field(field, cx);
            });
        });
        TextInputProps::single_line(value)
            .placeholder(placeholder)
            .mode(mode)
            .style(style)
            .request_focus(self.mail_compose_focused_field == field)
            .on_change(on_change)
            .on_submit(on_submit)
            .on_tab(on_tab)
            .on_focus(on_focus)
    }
}
