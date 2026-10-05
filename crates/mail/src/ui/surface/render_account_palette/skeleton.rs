use crate::ui::{
    alpha, div, if_light, mail_palette, px, rgb, AnyElement, Div, IntoElement, ParentElement,
    Styled, SurfaceState,
};

impl SurfaceState {
    pub(crate) fn render_mail_account_switching_content(&self) -> AnyElement {
        let palette = mail_palette(self.appearance_mode);
        div()
            .flex_grow(1.0)
            .min_h(px(0.0))
            .px(px(22.0))
            .pt(px(18.0))
            .flex()
            .flex_col()
            .gap(px(13.0))
            .children((0..8).map(|index| {
                div()
                    .h(px(54.0))
                    .rounded(px(7.0))
                    .bg(alpha(
                        palette.text_rgb,
                        if_light(self.appearance_mode, 0.035, 0.06),
                    ))
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .size(px(25.0))
                            .rounded_full()
                            .bg(alpha(palette.text_rgb, 0.07)),
                    )
                    .child(
                        div()
                            .h(px(8.0))
                            .w(px(if index % 3 == 0 { 230.0 } else { 170.0 }))
                            .rounded(px(4.0))
                            .bg(alpha(palette.text_rgb, 0.07)),
                    )
            }))
            .into_any_element()
    }

    pub(crate) fn render_mail_account_switching_sidebar(&self, width: f32) -> Div {
        let palette = mail_palette(self.appearance_mode);
        div()
            .w(px(width))
            .min_w(px(width))
            .max_w(px(width))
            .h_full()
            .flex_none()
            .bg(rgb(palette.activity_bg))
            .flex()
            .flex_col()
            .px(px(28.0))
            .pt(px(28.0))
            .gap(px(14.0))
            .children([122.0, 184.0, 154.0, 96.0].into_iter().map(|width| {
                div()
                    .h(px(9.0))
                    .w(px(width))
                    .rounded(px(4.0))
                    .bg(alpha(palette.text_rgb, 0.07))
            }))
    }
}
