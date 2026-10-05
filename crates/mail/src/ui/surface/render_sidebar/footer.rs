use super::{
    div, img, mail_palette, px, rgb, AnyElement, Context, Div, FontWeight, InteractiveElement,
    IntoElement, MailFooterAction, MouseButton, ParentElement, Styled, SurfaceState,
};
use crate::ui::surface::MAIL_FONT_FAMILY;
use gpui::{font, StatefulInteractiveElement};
use gpui_components::tooltip::Tooltip;

impl SurfaceState {
    pub(crate) fn render_mail_activity_footer(&self, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .h(px(62.0))
            .min_h(px(62.0))
            .px(px(30.0))
            .pt(px(22.0))
            .pb(px(12.0))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .font(font(MAIL_FONT_FAMILY))
                    .text_size(px(12.0))
                    .line_height(px(16.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(palette.text_rgb))
                    .child("Create Team"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.0))
                    .child(self.render_mail_footer_action_button(MailFooterAction::Invite, cx))
                    .child(self.render_mail_footer_action_button(MailFooterAction::ContactUs, cx))
                    .child(self.render_mail_footer_action_button(MailFooterAction::Calendar, cx))
                    .child(self.render_mail_footer_action_button(MailFooterAction::Settings, cx)),
            )
    }

    pub(crate) fn render_mail_footer_action_button(
        &self,
        action: MailFooterAction,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(format!("mail-footer-action-{}", action.ui_segment()))
            .relative()
            .size(px(14.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                img(self.render_mail_svg_icon(
                    action.icon_view_box(),
                    action.icon_body(self.appearance_mode),
                    cx,
                ))
                .opacity(0.3)
                .size(px(action.icon_size())),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    if action == MailFooterAction::Settings {
                        this.open_mail_split_settings(cx);
                    }
                }),
            )
            .tooltip(Tooltip::text(action.label()))
            .into_any_element()
    }
}
