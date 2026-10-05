use super::{
    alpha, div, if_light, mail_palette, point, px, rgb, AnyElement, BoxShadow, Context, Div,
    FontWeight, InteractiveElement, IntoElement, MailComposeAutocompleteItem, MouseButton,
    MouseDownEvent, ParentElement, Styled, SurfaceState,
};

struct MailComposeAutocompleteItemProps<'a> {
    item: &'a MailComposeAutocompleteItem,
    selected: bool,
}

impl SurfaceState {
    pub(super) fn render_mail_compose_autocomplete(
        &self,
        suggestions: &[MailComposeAutocompleteItem],
        selected_index: usize,
        cx: &mut Context<Self>,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .absolute()
            .left(px(0.0))
            .right(px(-84.0))
            .top(px(34.0))
            .rounded(px(12.0))
            .border_1()
            .border_color(rgb(palette.compose_suggestion_border))
            .bg(rgb(palette.compose_suggestion_bg))
            .shadow(vec![BoxShadow {
                color: alpha(0x000000, if_light(self.appearance_mode, 0.08, 0.32)),
                offset: point(px(0.0), px(14.0)),
                blur_radius: px(28.0),
                spread_radius: px(0.0),
                inset: false,
            }])
            .child(div().py(px(6.0)).flex().flex_col().children(
                suggestions.iter().enumerate().map(|(index, item)| {
                    self.render_mail_compose_autocomplete_item(
                        MailComposeAutocompleteItemProps {
                            item,
                            selected: index == selected_index,
                        },
                        cx,
                    )
                }),
            ))
    }

    fn render_mail_compose_autocomplete_item(
        &self,
        props: MailComposeAutocompleteItemProps<'_>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        let address = props.item.address.clone();
        (div()
            .px(px(16.0))
            .py(px(10.0))
            .bg(rgb(if props.selected {
                palette.compose_suggestion_selected_bg
            } else {
                palette.compose_suggestion_bg
            }))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                    this.select_mail_compose_autocomplete_item(address.clone(), cx);
                }),
            )
            .child(self.render_mail_compose_autocomplete_item_content(props.item, props.selected)))
        .into_any_element()
    }

    fn render_mail_compose_autocomplete_item_content(
        &self,
        item: &MailComposeAutocompleteItem,
        selected: bool,
    ) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(18.0))
            .child(
                div()
                    .flex_grow(1.0)
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(alpha(
                        palette.text_rgb,
                        if selected {
                            if_light(self.appearance_mode, 0.92, 0.88)
                        } else {
                            if_light(self.appearance_mode, 0.8, 0.74)
                        },
                    ))
                    .child(item.label.clone()),
            )
            .child(
                div()
                    .flex_none()
                    .text_size(px(13.0))
                    .font_weight(FontWeight::NORMAL)
                    .text_color(alpha(
                        if selected {
                            palette.compose_suggestion_match
                        } else {
                            palette.text_rgb
                        },
                        if selected {
                            0.72
                        } else {
                            if_light(self.appearance_mode, 0.5, 0.44)
                        },
                    ))
                    .child(item.detail.clone()),
            )
    }
}
