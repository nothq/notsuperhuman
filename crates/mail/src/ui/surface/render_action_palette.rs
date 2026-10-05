use super::{
    command_menu_empty_state, command_menu_panel_shell, command_menu_row_shell,
    command_menu_section_title, div, px, rgb, AnyElement, Context, Div, FluentBuilder, FontWeight,
    InteractiveElement, IntoElement, MailActionPaletteKind, MailActionPaletteOption,
    MailActionPaletteOptionAction, MailActionPaletteState, MailRowAction, MouseButton,
    MouseDownEvent, ParentElement, Styled, SurfaceState, MAIL_TABBAR_HEIGHT,
};

impl SurfaceState {
    pub(crate) fn render_mail_action_palette(
        &self,
        state: &MailActionPaletteState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let options = self.mail_action_palette_options(state);
        let selected_index = state.selected_index.min(options.len().saturating_sub(1));
        let title = match state.kind {
            MailActionPaletteKind::RemindMe => "Remind me",
            MailActionPaletteKind::Move => "Move to",
        };
        let top = if self.mail_open_thread_id.is_some() {
            76.0
        } else {
            MAIL_TABBAR_HEIGHT + 10.0
        };

        (div().absolute().top(px(top)).right(px(24.0)).child(
            command_menu_panel_shell(self.theme)
                .w(px(292.0))
                .child(command_menu_section_title(self.theme, title))
                .when(options.is_empty(), |this| {
                    this.child(command_menu_empty_state(self.theme, "No folders available"))
                })
                .children(options.iter().enumerate().map(|(index, option)| {
                    self.render_mail_action_palette_option(
                        index,
                        option,
                        index == selected_index,
                        cx,
                    )
                })),
        ))
        .into_any_element()
    }

    fn render_mail_action_palette_option(
        &self,
        index: usize,
        option: &MailActionPaletteOption,
        is_selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let option = option.clone();
        let mut row = command_menu_row_shell(self.theme, is_selected)
            .h(px(34.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    if let Some(state) = this.mail_action_palette.as_mut() {
                        state.selected_index = index;
                    }
                    this.activate_mail_action_palette(cx);
                }),
            )
            .child(self.render_mail_action_palette_option_body(&option, cx));
        row.interactivity()
            .on_hover(cx.listener(move |this, is_hovered, _, cx| {
                if *is_hovered {
                    if let Some(state) = this.mail_action_palette.as_mut() {
                        state.selected_index = index;
                    }
                    cx.notify();
                }
            }));
        row.into_any_element()
    }

    fn render_mail_action_palette_option_body(
        &self,
        option: &MailActionPaletteOption,
        cx: &mut Context<Self>,
    ) -> Div {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .min_w(px(0.0))
                    .child(self.render_mail_action_palette_icon(option, cx))
                    .child(
                        div()
                            .min_w(px(0.0))
                            .overflow_hidden()
                            .text_size(px(12.5))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(self.theme.text_primary))
                            .text_ellipsis()
                            .child(option.label.clone()),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(10.5))
                    .text_color(rgb(self.theme.text_hint))
                    .child(option.detail.clone()),
            )
    }

    fn render_mail_action_palette_icon(
        &self,
        option: &MailActionPaletteOption,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let action = match option.action {
            MailActionPaletteOptionAction::Snooze { .. } => MailRowAction::RemindMe,
            MailActionPaletteOptionAction::Move { .. } => MailRowAction::Move,
        };
        self.render_mail_row_icon(
            action.icon_view_box(),
            action.icon_body(self.appearance_mode),
            cx,
        )
    }
}
