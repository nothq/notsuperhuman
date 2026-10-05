use super::{
    div, img, mail_palette, px, AnyElement, Context, FluentBuilder, InteractiveElement,
    IntoElement, MailRowAction, MailTriageControl, MailTriageFlags, MouseButton, MouseDownEvent,
    ParentElement, Styled, SurfaceState,
};
use gpui::StatefulInteractiveElement;
use gpui_components::tooltip::{Tooltip, TooltipRow};

impl SurfaceState {
    pub(crate) fn render_mail_triage_button(
        &self,
        thread_id: &str,
        control: MailTriageControl,
        triage: MailTriageFlags,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let MailTriageFlags { unread, starred } = triage;
        let disabled = !self.mail_thread_triage_allowed(thread_id, control);
        let icon = mail_palette(self.appearance_mode).icon_rgb;
        let active = mail_triage_control_active(control, unread, starred);
        let icon_body = mail_triage_icon_body(control, active, icon);
        let action_thread_id = thread_id.to_string();
        div()
            .id(format!(
                "mail-thread-triage-{}-{}",
                thread_id,
                control.ui_segment()
            ))
            .w(px(22.0))
            .h(px(24.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(!disabled, |this| this.cursor_pointer())
            .child(
                img(self.render_mail_svg_icon("0 0 16 16", icon_body, cx))
                    .opacity(mail_triage_icon_opacity(disabled, active))
                    .size(px(15.0)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    if !disabled {
                        this.toggle_mail_thread_triage(action_thread_id.clone(), control, cx);
                    }
                }),
            )
            .into_any_element()
    }

    pub(crate) fn render_mail_thread_action_button(
        &self,
        thread_id: &str,
        action: MailRowAction,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let disabled = !self.mail_thread_action_allowed(thread_id, action);
        let thread_id = thread_id.to_string();
        let action_thread_id = thread_id.clone();
        let (icon_width, icon_height) = action.icon_size();
        div()
            .id(format!(
                "mail-thread-action-{}-{}",
                thread_id,
                action.ui_segment()
            ))
            .relative()
            .w(px(23.0))
            .h(px(24.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .when(!disabled, |this| this.cursor_pointer())
            .child(
                img(self.render_mail_svg_icon(
                    action.icon_view_box(),
                    action.icon_body(self.appearance_mode),
                    cx,
                ))
                .opacity(if disabled { 0.12 } else { 0.3 })
                .w(px(icon_width))
                .h(px(icon_height)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.handle_mail_thread_action(action_thread_id.clone(), action, cx);
                }),
            )
            .when(!disabled, |this| {
                this.tooltip(Tooltip::rows([
                    TooltipRow::new(action.label()).shortcut(action.shortcut())
                ]))
            })
            .into_any_element()
    }
}

fn mail_triage_control_active(control: MailTriageControl, unread: bool, starred: bool) -> bool {
    match control {
        MailTriageControl::Star => starred,
        MailTriageControl::Read => unread,
    }
}

fn mail_triage_icon_body(control: MailTriageControl, active: bool, icon: u32) -> String {
    match (control, active) {
        (MailTriageControl::Star, true) => format!(
            r##"<path d="M8 0.8L10.2 5.25L15.1 5.96L11.55 9.42L12.39 14.3L8 12L3.61 14.3L4.45 9.42L0.9 5.96L5.8 5.25L8 0.8Z" fill="#{icon:06X}"/>"##
        ),
        (MailTriageControl::Star, false) => format!(
            r##"<path d="M8 0.8L10.2 5.25L15.1 5.96L11.55 9.42L12.39 14.3L8 12L3.61 14.3L4.45 9.42L0.9 5.96L5.8 5.25L8 0.8Z" fill="none" stroke="#{icon:06X}" stroke-linejoin="round"/>"##
        ),
        (MailTriageControl::Read, true) => format!(
            r##"<rect x="1" y="3" width="14" height="10" rx="1.5" fill="none" stroke="#{icon:06X}"/><path d="M2 4L8 8.5L14 4" fill="none" stroke="#{icon:06X}" stroke-linejoin="round"/>"##
        ),
        (MailTriageControl::Read, false) => format!(
            r##"<path d="M1 6L8 1.5L15 6V13.5H1V6Z" fill="none" stroke="#{icon:06X}" stroke-linejoin="round"/><path d="M1.5 6.5L8 10.5L14.5 6.5" fill="none" stroke="#{icon:06X}" stroke-linejoin="round"/>"##
        ),
    }
}

fn mail_triage_icon_opacity(disabled: bool, active: bool) -> f32 {
    if disabled {
        0.12
    } else if active {
        0.62
    } else {
        0.3
    }
}
