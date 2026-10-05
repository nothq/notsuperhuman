use super::{
    alpha, div, mail_palette, px, rgb, AnyElement, Context, Div, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailSignInField, MailSignInPending, MailSignInState,
    MouseButton, MouseDownEvent, ParentElement, StatefulInteractiveElement, Styled, SurfaceState,
    MAIL_FONT_FAMILY,
};
use gpui::font;
use gpui_components::backdrop::{dismissible_backdrop, BackdropDismissal};

const SIGN_IN_CARD_WIDTH: f32 = 400.0;

impl SurfaceState {
    /// The whole surface while no account is signed in.
    pub(crate) fn render_mail_sign_in_screen(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let card = self
            .mail_sign_in
            .as_ref()
            .map(|state| self.render_mail_sign_in_card(state, cx));
        div()
            .flex_grow(1.0)
            .min_w(px(0.0))
            .min_h(px(0.0))
            .flex()
            .items_center()
            .justify_center()
            .font(font(MAIL_FONT_FAMILY))
            .bg(rgb(palette.activity_bg))
            .children(card)
            .into_any_element()
    }

    /// Adding another account from the account palette.
    pub(crate) fn render_mail_sign_in_overlay(
        &self,
        state: &MailSignInState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let top = self.mail_content_top_inset();
        div()
            .id("mail-sign-in")
            .absolute()
            .top(px(top))
            .right(px(0.0))
            .bottom(px(0.0))
            .left(px(0.0))
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .child(dismissible_backdrop(
                div()
                    .absolute()
                    .top(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .left(px(0.0))
                    .bg(alpha(0x000000, 0.32)),
                BackdropDismissal::new(|this: &mut SurfaceState, _, _, cx| {
                    this.close_mail_sign_in(cx);
                }),
                cx,
            ))
            .child(self.render_mail_sign_in_card(state, cx))
            .into_any_element()
    }

    fn render_mail_sign_in_card(&self, state: &MailSignInState, cx: &mut Context<Self>) -> Div {
        let palette = mail_palette(self.appearance_mode);
        let text = palette.text_rgb;
        let title = if state.dismissible {
            "Add an account"
        } else {
            "notsuperhuman"
        };
        let subtitle = if state.dismissible {
            "Connect your Superhuman inboxes or a JMAP account."
        } else {
            "Your inbox. A fast, native app."
        };
        div()
            .relative()
            .w(px(SIGN_IN_CARD_WIDTH))
            .rounded(px(8.0))
            .px(px(36.0))
            .pt(px(32.0))
            .pb(px(28.0))
            .bg(rgb(palette.message_card_bg))
            .border_1()
            .border_color(alpha(text, 0.08))
            .shadow_lg()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _: &MouseDownEvent, _, cx| cx.stop_propagation()),
            )
            .child(
                div()
                    .text_size(px(22.0))
                    .line_height(px(28.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(alpha(text, 0.92))
                    .child(title),
            )
            .child(
                div()
                    .mt(px(-8.0))
                    .text_size(px(13.0))
                    .line_height(px(19.0))
                    .text_color(alpha(text, 0.55))
                    .child(subtitle),
            )
            .child(self.render_mail_superhuman_button(state, cx))
            .child(div().text_size(px(11.0)).line_height(px(16.0))
                .text_color(alpha(text, 0.48))
                .child("Connects Gmail accounts signed in to Superhuman Desktop. macOS may ask for Keychain access."))
            .child(render_mail_sign_in_divider(text))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(self.mail_sign_in_input_entity(MailSignInField::Server, cx))
                    .child(self.mail_sign_in_input_entity(MailSignInField::Token, cx)),
            )
            .child(self.render_mail_jmap_button(state, cx))
            .child(
                div()
                    .text_size(px(11.0))
                    .line_height(px(16.0))
                    .text_color(alpha(text, 0.42))
                    .child("Fastmail tokens live in Settings › Privacy & Security › API tokens."),
            )
            .when_some(state.error.as_deref(), |this, error| {
                this.child(
                    div()
                        .text_size(px(12.0))
                        .line_height(px(17.0))
                        .text_color(rgb(0xd14343))
                        .child(error.to_string()),
                )
            })
            .when(state.dismissible, |this| {
                this.child(
                    div()
                        .absolute()
                        .top(px(14.0))
                        .right(px(16.0))
                        .text_size(px(11.0))
                        .text_color(alpha(text, 0.36))
                        .child("Esc"),
                )
            })
    }

    fn render_mail_superhuman_button(
        &self,
        state: &MailSignInState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let text = palette.text_rgb;
        let waiting = state.pending == Some(MailSignInPending::Superhuman);
        let busy = state.pending.is_some();
        let label = if waiting {
            "Connecting to Superhuman…"
        } else {
            "Continue with Superhuman"
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .id("mail-sign-in-superhuman")
                    .h(px(40.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(alpha(text, 0.16))
                    .bg(alpha(text, if waiting { 0.04 } else { 0.0 }))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(px(10.0))
                    .when(!busy, |this| {
                        this.cursor_pointer()
                            .hover(|style| style.bg(alpha(text, 0.04)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_mail_superhuman_sign_in(cx);
                            }))
                    })
                    .child(
                        div()
                            .text_size(px(13.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(alpha(text, 0.82))
                            .child(label),
                    ),
            )
            .into_any_element()
    }

    fn render_mail_jmap_button(
        &self,
        state: &MailSignInState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let connecting = state.pending == Some(MailSignInPending::Jmap);
        let ready = state.pending.is_none()
            && !state.server.trim().is_empty()
            && !state.token.trim().is_empty();
        div()
            .id("mail-sign-in-jmap")
            .h(px(38.0))
            .rounded(px(4.0))
            .bg(rgb(palette.compose_caret))
            .opacity(if ready || connecting { 1.0 } else { 0.45 })
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(13.0))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(0xffffff))
            .when(ready, |this| {
                this.cursor_pointer()
                    .hover(|style| style.opacity(0.9))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submit_mail_jmap_sign_in(cx);
                    }))
            })
            .child(if connecting {
                "Connecting…"
            } else {
                "Connect JMAP account"
            })
            .into_any_element()
    }
}

fn render_mail_sign_in_divider(text: u32) -> Div {
    div()
        .my(px(2.0))
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(div().flex_grow(1.0).h(px(1.0)).bg(alpha(text, 0.1)))
        .child(
            div()
                .text_size(px(11.0))
                .text_color(alpha(text, 0.42))
                .child("or a JMAP server"),
        )
        .child(div().flex_grow(1.0).h(px(1.0)).bg(alpha(text, 0.1)))
}
