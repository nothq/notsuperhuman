//! Line boxes as Chromium builds them on macOS.

use super::{
    homogeneous_inline_font_size, homogeneous_inline_line_height, rendered_font_family,
    text_font_family,
};
use crate::ui::render::MailRenderOptions;
use crate::ui::types::*;

/// The paragraph's line height and how far its text moves down to sit on the
/// line's baseline.
pub(super) fn paragraph_line_metrics(
    paragraph: &MailParagraph,
    family: &str,
    font_size: f32,
    options: &MailRenderOptions,
) -> (f32, f32) {
    homogeneous_inline_line_height(paragraph)
        .or(paragraph.style.line_height)
        .map(|line_height| {
            line_box_height(
                paragraph,
                family,
                Some(line_height.resolve(font_size)),
                font_size,
                options,
            )
        })
        .unwrap_or_else(|| normal_line_height(paragraph, font_size, options))
}

/// A face's vertical metrics as fractions of its size: ascent, descent and
/// line gap, fitted to Chromium's line boxes on macOS.
fn face_metrics(family: &str) -> (f32, f32, f32) {
    match family {
        "Helvetica" => (0.77, 0.23, 0.0),
        "Helvetica Neue" => (0.95, 0.213, 0.028),
        "Times" => (0.75, 0.25, 0.0),
        "Times New Roman" => (0.891, 0.216, 0.042),
        "Verdana" => (1.004, 0.209, 0.0),
        "Geneva" => (0.994, 0.25, 0.084),
        "Tahoma" => (0.994, 0.207, 0.0),
        "Trebuchet MS" => (0.938, 0.222, 0.0),
        "Georgia" => (0.917, 0.219, 0.0),
        "Lucida Grande" => (0.9668, 0.2109, 0.0),
        "Courier New" => (0.829, 0.297, 0.0),
        "Menlo" => (0.929, 0.235, 0.0),
        _ => (0.904, 0.212, 0.032),
    }
}

/// How far a line reaches above and below its baseline for one font at one
/// size: the glyph extents plus the leading, split as Chromium splits it on
/// a 2x display. Each metric rounds to whole device pixels, Times and
/// Helvetica ascents are padded by 15% to match their Windows counterparts,
/// and the ascent takes the floor of half the leading.
pub(in crate::ui::render) fn line_extents(
    family: &str,
    size: f32,
    line_height: Option<f32>,
) -> (f32, f32) {
    let (ascent, descent, gap) = face_metrics(family);
    let device = |metric: f32| (metric * size * 2.0).round();
    let (mut ascent, descent) = (device(ascent), device(descent));
    if matches!(family, "Times" | "Helvetica") {
        ascent += ((ascent + descent) * 0.15).round();
    }
    let leading = line_height.map_or(device(gap), |line_height| {
        line_height * 2.0 - ascent - descent
    });
    let above = ascent + (leading / 2.0).floor();
    (
        above / 2.0,
        (ascent + descent + leading) / 2.0 - above / 2.0,
    )
}

/// The height of a line holding text of one size inside a block of another.
///
/// CSS 2.1 §10.8: every line box holds the block's strut, and each box on
/// the line spreads its line height around its own glyphs. When the block's
/// font is larger than the text in it, the strut reaches higher and lower
/// than the text alone, and the line is the union — an Outlook signature of
/// 9pt spans on 11pt lines in a 16px cell sets on 16px lines. With
/// `line-height: normal` each font brings its own: 14.67px Arial in a 16px
/// Times block is an 18.5px line, not 1.15 × 16.
///
/// The text sits on the line's baseline, where GPUI centres it instead, so
/// the second value is how far the text moves down to reach that baseline.
fn line_box_height(
    paragraph: &MailParagraph,
    family: &str,
    line_height: Option<f32>,
    text_size: f32,
    options: &MailRenderOptions,
) -> (f32, f32) {
    let (text_above, text_below) = line_extents(family, text_size, line_height);
    let strut_size = paragraph.style.font_size.unwrap_or(options.base_font_size);
    let (strut_above, strut_below) = line_extents(
        &rendered_font_family(paragraph.style.font_family, options),
        strut_size,
        paragraph
            .style
            .line_height
            .map(|line_height| line_height.resolve(strut_size)),
    );
    let height = text_above.max(strut_above) + text_below.max(strut_below);
    let (ascent, descent, _) = face_metrics(family);
    let centred_baseline = (height - (ascent + descent) * text_size) / 2.0 + ascent * text_size;
    (height, text_above.max(strut_above) - centred_baseline)
}

/// `line-height: normal` for a paragraph that sets a font size: the line box
/// of its text and the block's strut, from the face's own metrics. A
/// paragraph with no size of its own is the plain-text card, which keeps the
/// card's line height.
fn normal_line_height(
    paragraph: &MailParagraph,
    font_size: f32,
    options: &MailRenderOptions,
) -> (f32, f32) {
    if paragraph.style.font_size.is_none() && homogeneous_inline_font_size(paragraph).is_none() {
        return (options.line_height, 0.0);
    }
    line_box_height(
        paragraph,
        &text_font_family(paragraph, options),
        None,
        font_size,
        options,
    )
}
