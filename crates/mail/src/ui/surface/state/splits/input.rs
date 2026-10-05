use std::rc::Rc;

use super::super::super::MailSplitInputField;
use crate::ui::surface::{MailPalette, MAIL_FONT_FAMILY};
use crate::ui::{
    alpha, mail_palette, px, rgb, AppContext, Context, Entity, SurfaceState, TextInput,
    TextInputAction, TextInputChange, TextInputProps, TextInputStyle,
};

#[derive(Clone, Copy)]
enum MailSplitInputAction {
    Advance,
    Cancel,
    Focus(MailSplitInputField),
}

impl SurfaceState {
    pub(crate) fn mail_split_text_input_entity(
        &self,
        field: MailSplitInputField,
        cx: &mut Context<Self>,
    ) -> Entity<TextInput> {
        let props = self.mail_split_text_input_props(field, cx);
        if let Some(input) = self.mail_split_text_inputs.borrow().get(&field).cloned() {
            input.update(cx, |input, cx| input.apply_props(props, cx));
            return input;
        }
        let input = cx.new(|cx| TextInput::new(props, cx));
        self.mail_split_text_inputs
            .borrow_mut()
            .insert(field, input.clone());
        input
    }

    fn mail_split_text_input_props(
        &self,
        field: MailSplitInputField,
        cx: &mut Context<Self>,
    ) -> TextInputProps {
        let palette = mail_palette(self.appearance_mode);
        let editor = self
            .mail_split_settings
            .as_ref()
            .and_then(|settings| settings.editor.as_ref())
            .expect("Mail split input requires an open editor");
        let (value, placeholder) = match field {
            MailSplitInputField::Name => (editor.name.clone(), "Split name"),
            MailSplitInputField::Query => (editor.query.clone(), "Search query"),
        };
        let on_change = self.mail_split_input_change(field, cx);
        let advance = self.mail_split_input_action(MailSplitInputAction::Advance, cx);
        TextInputProps::single_line(value)
            .placeholder(placeholder)
            .request_focus(editor.focused_field == field)
            .style(mail_split_input_style(&palette))
            .on_change(on_change)
            .on_submit(advance.clone())
            .on_tab(advance)
            .on_escape(self.mail_split_input_action(MailSplitInputAction::Cancel, cx))
            .on_focus(self.mail_split_input_action(MailSplitInputAction::Focus(field), cx))
    }

    fn mail_split_input_change(
        &self,
        field: MailSplitInputField,
        cx: &mut Context<Self>,
    ) -> TextInputChange {
        let surface = cx.entity();
        Rc::new(move |value, _window, cx| {
            surface.update(cx, |surface, cx| {
                surface.set_mail_split_editor_value(field, value, cx);
            });
        })
    }

    fn mail_split_input_action(
        &self,
        action: MailSplitInputAction,
        cx: &mut Context<Self>,
    ) -> TextInputAction {
        let surface = cx.entity();
        Rc::new(move |_window, cx| {
            surface.update(cx, |surface, cx| match action {
                MailSplitInputAction::Advance => surface.advance_mail_split_editor(cx),
                MailSplitInputAction::Cancel => {
                    surface.cancel_mail_split_editor(cx);
                }
                MailSplitInputAction::Focus(field) => {
                    surface.focus_mail_split_editor_field(field, cx);
                }
            });
        })
    }
}

fn mail_split_input_style(palette: &MailPalette) -> TextInputStyle {
    TextInputStyle {
        height: px(34.0),
        min_height: px(34.0),
        padding_x: px(10.0),
        padding_y: px(0.0),
        radius: px(3.0),
        background: alpha(palette.text_rgb, 0.04),
        border: alpha(palette.text_rgb, 0.12),
        focused_border: alpha(palette.compose_caret, 0.7),
        text: alpha(palette.text_rgb, 0.86),
        placeholder: alpha(palette.text_rgb, 0.34),
        selection: alpha(palette.compose_caret, 0.28),
        caret: rgb(palette.compose_caret).into(),
        font_size: px(12.0),
        line_height: px(18.0),
        font_family: Some(MAIL_FONT_FAMILY.into()),
    }
}
