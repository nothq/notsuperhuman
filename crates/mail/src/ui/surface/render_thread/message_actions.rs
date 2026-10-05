use super::{
    div, px, AnyElement, Context, Div, FluentBuilder, InteractiveElement, IntoElement,
    MailMessageAction, MouseButton, MouseDownEvent, ParentElement, Styled, SurfaceState,
};
use gpui::StatefulInteractiveElement;
use gpui_components::tooltip::{Tooltip, TooltipRow};

impl SurfaceState {
    pub(crate) fn render_mail_message_action_group(
        &self,
        thread_id: &str,
        message_id: &str,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .h(px(18.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(self.render_mail_message_action_button(
                thread_id,
                message_id,
                MailMessageAction::Reply,
                cx,
            ))
            .child(self.render_mail_message_action_button(
                thread_id,
                message_id,
                MailMessageAction::Forward,
                cx,
            ))
    }

    fn render_mail_message_action_button(
        &self,
        thread_id: &str,
        message_id: &str,
        action: MailMessageAction,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let disabled = !self.mail_active_account_can_submit();
        let message_card_id = format!("{thread_id}:{message_id}");
        let hover_message_card_id = message_card_id.clone();
        let mut button = div()
            .id(format!(
                "mail-message-action-{}-{}-{}",
                thread_id,
                message_id,
                action.ui_segment()
            ))
            .relative()
            .size(px(18.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(!disabled, |this| this.cursor_pointer())
            .opacity(if disabled { 0.4 } else { 1.0 })
            .child(self.render_mail_row_icon(
                action.icon_view_box(),
                action.icon_body(self.appearance_mode),
                cx,
            ))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                    cx.stop_propagation();
                    match action {
                        MailMessageAction::Reply => this.reply_mail_thread(cx),
                        MailMessageAction::Forward => this.forward_mail_thread(cx),
                    };
                    this.mail_shortcuts_focused = true;
                    cx.focus_self(window);
                }),
            );
        button
            .interactivity()
            .on_hover(cx.listener(move |this, is_hovered, _, cx| {
                this.set_mail_message_action_hover(
                    hover_message_card_id.clone(),
                    action,
                    *is_hovered,
                    cx,
                );
            }));
        (button.when(!disabled, |this| {
            this.tooltip(Tooltip::rows([
                TooltipRow::new(action.label()).shortcut(action.shortcut()),
                TooltipRow::new(action.pop_out_label()).shortcut(format!("⇧{}", action.shortcut())),
            ]))
        }))
        .into_any_element()
    }
}
