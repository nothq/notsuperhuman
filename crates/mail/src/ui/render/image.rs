use super::{apply_box_style, layout::mail_image_is_inline_icon, MailRenderOptions};
use crate::ui::types::*;
use gpui::{div, img, prelude::*, px, AnyElement, ObjectFit, RenderImage, StyledImage};
use std::sync::Arc;

/// The layers a box paints beneath its content: the fill, when it sits under
/// a border, and the background image.
///
/// Both cover the box's padding box, as CSS paints them: an absolute child
/// at `inset_0` is placed against its parent's padding edge. Offsetting the
/// fill by the border as well shrank it twice, which on a padded pill left a
/// small blot inside a ring three times too thick.
pub(super) fn render_box_background_image(
    style: &MailStyle,
    options: &MailRenderOptions,
    path: &str,
) -> Option<AnyElement> {
    let fill = super::fill_under_border(style).then(|| render_inset_fill(style));
    let image = render_background_image_layer(style, options, path);
    if fill.is_none() && image.is_none() {
        return None;
    }
    Some(
        div()
            .absolute()
            .inset_0()
            .children(fill)
            .children(image)
            .into_any_element(),
    )
}

/// The radius of a box's padding edge: its border radius less the border.
fn padding_edge_radius(style: &MailStyle) -> Option<f32> {
    let border = [
        super::box_border_top_width(style),
        super::box_border_right_width(style),
        super::box_border_bottom_width(style),
        super::box_border_left_width(style),
    ]
    .into_iter()
    .fold(0.0f32, f32::max);
    style.border_radius.map(|radius| (radius - border).max(0.0))
}

/// A box's fill drawn inside its border rather than under it.
///
/// GPUI paints a `Div`'s background and border as two quads that share the
/// outer edge, so where that edge is antialiased the fill shows through the
/// border: a hairline of the fill colour round every bordered pill and
/// button. The box's own quad takes the border colour instead (see
/// `apply_box_decoration`), and the real fill is this layer, whose edge lies
/// wholly under the opaque border.
fn render_inset_fill(style: &MailStyle) -> AnyElement {
    div()
        .absolute()
        .inset_0()
        .when_some(style.background_color, |this, color| {
            this.bg(gpui::rgb(color))
        })
        .when_some(padding_edge_radius(style), |this, radius| {
            this.rounded(px(radius))
        })
        .into_any_element()
}

fn render_background_image_layer(
    style: &MailStyle,
    options: &MailRenderOptions,
    path: &str,
) -> Option<AnyElement> {
    let url = style.background_image_url.as_ref()?;
    let image = options.image_resolver.as_ref()?.as_ref()(url)?;
    Some(
        div()
            .absolute()
            .inset_0()
            .overflow_hidden()
            .when_some(padding_edge_radius(style), |this, radius| {
                this.rounded(px(radius))
            })
            .child(match background_image_object_fit(style) {
                // GPUI clips to rectangles only, so the corners are rounded
                // where the image is painted, not by the layer around it. A
                // cover image is painted larger than the box, which would put
                // those corners outside it, so it is cropped to the box first.
                Some(ObjectFit::Cover) if padding_edge_radius(style).is_some_and(|r| r > 0.0) => {
                    render_rounded_cover(
                        image,
                        padding_edge_radius(style).unwrap_or_default(),
                        style.background_position,
                        format!("mail-cover-{path}"),
                    )
                }
                Some(fit) => img(image)
                    .size_full()
                    .object_fit(fit)
                    .when_some(padding_edge_radius(style), |this, radius| {
                        this.rounded(px(radius))
                    })
                    .into_any_element(),
                None => match natural_image_size(&image) {
                    Some(natural) => padded_img(image, natural, 1.0, (0.0, 0.0)),
                    None => div().into_any_element(),
                },
            })
            .into_any_element(),
    )
}

/// The padded frame (see [`crate::ui::IMAGE_PAD`]) drawn so that exactly the
/// image shows: `scale` maps image pixels to CSS pixels and `origin` is where
/// the image's own top-left corner goes within the parent box.
fn padded_img(
    image: std::sync::Arc<RenderImage>,
    (width, height): (f32, f32),
    scale: f32,
    (left, top): (f32, f32),
) -> AnyElement {
    // The ring stays off screen: its outermost device row still samples the
    // atlas beyond the sprite, so a box the size of the image clips it, and
    // the frame is pulled outward by the ring with margins. Nothing here is
    // positioned absolutely — a sprite wider than the atlas' default page
    // draws nothing from inside an absolutely positioned element.
    let pad = crate::ui::IMAGE_PAD as f32 * scale;
    div()
        .flex_none()
        .ml(px(left))
        .mt(px(top))
        .w(px(width * scale))
        .h(px(height * scale))
        .overflow_hidden()
        .child(
            div()
                .flex_none()
                .ml(px(-pad))
                .mt(px(-pad))
                .w(px(width * scale + 2.0 * pad))
                .h(px(height * scale + 2.0 * pad))
                .child(img(image).size_full().object_fit(ObjectFit::Fill)),
        )
        .into_any_element()
}

/// How a background image fills its box, or `None` for `auto`: the image at
/// its own size, anchored top-left and clipped by the box, never scaled.
/// Fitting an `auto` image to the box resampled every edge texel against the
/// atlas and drew a gray hairline round a white 700px header image.
pub(super) fn background_image_object_fit(style: &MailStyle) -> Option<ObjectFit> {
    match style.background_size {
        MailBackgroundSize::Auto => None,
        MailBackgroundSize::Contain => Some(ObjectFit::Contain),
        MailBackgroundSize::Cover => Some(ObjectFit::Cover),
    }
}

/// A decoded image scaled to fit a `width` × `height` box, centred in it.
fn render_contained_image(resolved: Arc<RenderImage>, width: f32, height: f32) -> AnyElement {
    let Some((natural_width, natural_height)) = natural_image_size(&resolved) else {
        return img(resolved)
            .size_full()
            .object_fit(ObjectFit::Contain)
            .into_any_element();
    };
    let scale = (width / natural_width).min(height / natural_height);
    let origin = (
        (width - natural_width * scale) / 2.0,
        (height - natural_height * scale) / 2.0,
    );
    padded_img(resolved, (natural_width, natural_height), scale, origin)
}

pub(super) fn render_image(
    image: &MailImage,
    options: &MailRenderOptions,
    available: f32,
) -> AnyElement {
    let resolved = options
        .image_resolver
        .as_ref()
        .and_then(|resolver| resolver(&image.src));
    let (width, height) = image_display_size(image, options, resolved.as_deref(), available);
    let content = match resolved {
        Some(resolved) => render_contained_image(resolved, width, height),
        None => render_pending_image(image, width, height, options).into_any_element(),
    };
    let mut layout_style = image.style.clone();
    layout_style.height = None;
    // Both dimensions are already resolved, so both are applied. Deriving the
    // height from an aspect ratio instead left the box zero-high whenever the
    // image also carried `max-width: 100%` — the picture still painted, so it
    // overlapped whatever followed it, which is most of the icon rows in email.
    let image_box = apply_box_style(
        div()
            .w(px(width))
            .h(px(height))
            .overflow_hidden()
            .child(content),
        &layout_style,
        options,
    )
    .flex_shrink_1();
    let uses_inline_flow = mail_image_is_inline_icon(image)
        || matches!(
            image.style.display,
            MailDisplay::Inline | MailDisplay::InlineBlock | MailDisplay::TableCell
        )
        || image.style.float != MailFloat::None;
    if uses_inline_flow {
        return image_box.into_any_element();
    }
    let text_align = image.style.text_align.resolve(image.style.direction);
    let center_image = text_align == MailTextAlign::Center
        || (image.style.margin_left_auto && image.style.margin_right_auto);
    let right_image =
        text_align == MailTextAlign::Right || matches!(image.style.float, MailFloat::Right);
    let wrapper = div()
        .w_full()
        .min_w(px(0.0))
        .max_w_full()
        .flex()
        .child(image_box)
        .when(center_image, |this| this.justify_center())
        .when(right_image, |this| this.justify_end());
    wrapper.into_any_element()
}

fn render_pending_image(
    image: &MailImage,
    width: f32,
    height: f32,
    options: &MailRenderOptions,
) -> gpui::Div {
    let show_alt = pending_image_should_show_alt(image, width, height);
    div()
        .size_full()
        .rounded(px(4.0))
        .border_1()
        .border_color(options.border_color)
        .bg(options.placeholder_bg)
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(11.0))
        .line_height(px(14.0))
        .text_color(options.muted_text_color)
        .when(show_alt, |this| this.child(image.alt.clone()))
}

pub(super) fn pending_image_should_show_alt(image: &MailImage, width: f32, height: f32) -> bool {
    !image.alt.trim().is_empty() && width >= 160.0 && height >= 40.0
}

pub(super) fn image_display_size(
    image: &MailImage,
    _options: &MailRenderOptions,
    resolved: Option<&RenderImage>,
    available: f32,
) -> (f32, f32) {
    let natural_size = resolved.and_then(natural_image_size);
    // A percentage width resolves against the container. Only absolute widths
    // were consulted here, so `width: 100%` — which responsive email CSS applies
    // to images constantly — fell through to a hardcoded default size.
    let declared_width = image
        .width
        .or(image.style.width)
        .or_else(|| super::intrinsic::definite_width(&image.style, available));
    let declared_height = image.height.or(image.style.height);
    let mut width = declared_width
        .or_else(|| match (declared_height, natural_size) {
            (Some(height), Some((natural_width, natural_height))) => {
                Some(height * natural_width / natural_height)
            }
            _ => None,
        })
        .or_else(|| natural_size.map(|(width, _)| width))
        .unwrap_or(320.0)
        .max(1.0);
    if let Some(min_width) = image.style.min_width {
        width = width.max(min_width);
    }
    if let Some(max_width) = image.style.max_width {
        width = width.min(max_width);
    }
    // `max-width: 100%` is how every responsive email keeps a large image inside
    // the body. Without resolving it the image drew at its natural size, which
    // made it both too wide and — through its aspect ratio — far too tall.
    if let Some(max_width) = super::intrinsic::max_width_constraint(&image.style, available) {
        width = width.min(max_width.max(1.0));
    }
    let scaled_by_max_width = declared_width.is_some_and(|declared| width < declared);
    let mut height = declared_height
        .filter(|_| !scaled_by_max_width || image.style.height.is_some())
        .or_else(|| {
            natural_size
                .map(|(natural_width, natural_height)| width * natural_height / natural_width)
        })
        .unwrap_or(width * 0.5625)
        .max(1.0);
    if let Some(max_height) = image.style.max_height {
        height = height.min(max_height.max(1.0));
    }
    (width, height)
}

pub(super) fn natural_image_size(image: &RenderImage) -> Option<(f32, f32)> {
    if image.frame_count() == 0 {
        return None;
    }
    let size = image.size(0);
    let pad = 2.0 * crate::ui::IMAGE_PAD as f32;
    let width = u32::from(size.width) as f32 - pad;
    let height = u32::from(size.height) as f32 - pad;
    (width > 0.0 && height > 0.0).then_some((width, height))
}

/// A `background-size: cover` image with rounded corners: the part of the
/// image the box shows is cut out, then painted at exactly the box with the
/// corners rounded, so the rounding lands on the visible edge. The element
/// keeps its crop across frames and cuts a new one only when the box changes.
fn render_rounded_cover(
    image: std::sync::Arc<RenderImage>,
    radius: f32,
    position: MailBackgroundPosition,
    id: String,
) -> AnyElement {
    gpui::canvas(
        move |bounds, window, cx| {
            let rect = cover_rect(&image, bounds.size, position)?;
            let key = (image.id.0, rect);
            let crop = window.use_keyed_state(gpui::ElementId::Name(id.into()), cx, |_, _| {
                None::<((usize, [u32; 4]), std::sync::Arc<RenderImage>)>
            });
            if let Some((cached, visible)) = crop.read(cx) {
                if *cached == key {
                    return Some(std::sync::Arc::clone(visible));
                }
            }
            let visible = crop_image(&image, rect)?;
            crop.update(cx, |crop, _| *crop = Some((key, visible.clone())));
            Some(visible)
        },
        move |bounds, visible, window, _| {
            let Some(visible) = visible else {
                return;
            };
            let corners = gpui::Corners::all(px(radius)).clamp_radii_for_quad_size(bounds.size);
            let _ = window.paint_image(bounds, corners, visible, 0, false);
        },
    )
    .size_full()
    .into_any_element()
}

/// The pixels of `image` that `cover` shows in a box of `size`, centred or
/// anchored to the top-left as `background-position` says: x, y, width and
/// height within the padded frame.
fn cover_rect(
    image: &RenderImage,
    size: gpui::Size<gpui::Pixels>,
    position: MailBackgroundPosition,
) -> Option<[u32; 4]> {
    let (natural_width, natural_height) = natural_image_size(image)?;
    let (box_width, box_height) = (f32::from(size.width), f32::from(size.height));
    if box_width <= 0.0 || box_height <= 0.0 {
        return None;
    }
    let scale = (box_width / natural_width).max(box_height / natural_height);
    let width = (box_width / scale).round().clamp(1.0, natural_width);
    let height = (box_height / scale).round().clamp(1.0, natural_height);
    let (x, y) = match position {
        MailBackgroundPosition::Initial
        | MailBackgroundPosition::Top
        | MailBackgroundPosition::Left => (0.0, 0.0),
        _ => (
            ((natural_width - width) / 2.0).round(),
            ((natural_height - height) / 2.0).round(),
        ),
    };
    let pad = crate::ui::IMAGE_PAD;
    Some([x as u32 + pad, y as u32 + pad, width as u32, height as u32])
}

fn crop_image(image: &RenderImage, rect: [u32; 4]) -> Option<std::sync::Arc<RenderImage>> {
    let stride = u32::from(image.size(0).width) as usize * 4;
    let bytes = image.as_bytes(0)?;
    let mut buffer = ::image::RgbaImage::new(rect[2], rect[3]);
    for (row, line) in buffer.rows_mut().enumerate() {
        let start = (rect[1] as usize + row) * stride + rect[0] as usize * 4;
        for (column, pixel) in line.enumerate() {
            let offset = start + column * 4;
            pixel.0.copy_from_slice(&bytes[offset..offset + 4]);
        }
    }
    Some(std::sync::Arc::new(RenderImage::new(vec![
        ::image::Frame::new(buffer),
    ])))
}
