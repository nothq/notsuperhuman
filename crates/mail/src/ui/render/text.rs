use super::{apply_box_style, hsla_color, image::render_box_background_image, MailRenderOptions};
use crate::ui::types::*;
use gpui::{
    div, prelude::FluentBuilder as _, prelude::*, px, relative, AnyElement, Div, FontStyle,
    FontWeight, HighlightStyle, SharedString, StrikethroughStyle, StyledText, TextAlign, TextStyle,
    UnderlineStyle,
};
use gpui_components::selectable_text::SelectableText;
use std::ops::Range;

mod metrics;
mod spacing;
pub(super) use metrics::line_extents;
pub(super) use spacing::spaced_text;

type TextHighlight = (Range<usize>, HighlightStyle);

struct RenderedMailText {
    text: SharedString,
    styled_text: StyledText,
    link_ranges: Vec<Range<usize>>,
    link_targets: Vec<String>,
    selection_color: gpui::Hsla,
}

impl RenderedMailText {
    fn into_element(self, text_element_id: String) -> AnyElement {
        let element = SelectableText::new(
            text_element_id,
            self.text,
            self.styled_text,
            self.selection_color,
        );
        if self.link_targets.is_empty() {
            return element.into_any_element();
        }
        let link_targets = self.link_targets;
        element
            .on_link_click(self.link_ranges, move |range_index, _window, cx| {
                if let Some(target) = link_targets.get(range_index) {
                    cx.open_url(target.as_str());
                }
            })
            .into_any_element()
    }
}

pub(super) fn render_paragraph(
    paragraph: &MailParagraph,
    options: &MailRenderOptions,
    text_element_id: String,
) -> AnyElement {
    // CSS resolves the innermost declaration, so a font size every run agrees
    // on beats the size the block inherited: an `<a style="font-size: 11px">`
    // filling a 16px paragraph renders at 11px, not 16px.
    let font_size = homogeneous_inline_font_size(paragraph)
        .or(paragraph.style.font_size)
        .unwrap_or(options.base_font_size);
    let family = text_font_family(paragraph, options);
    let (line_height, baseline_shift) =
        metrics::paragraph_line_metrics(paragraph, &family, font_size, options);
    let mut element = div()
        .text_size(px(font_size))
        .line_height(px(line_height))
        .text_color(
            paragraph
                .style
                .color
                .map(hsla_color)
                .unwrap_or(options.text_color),
        )
        .text_align(text_align(
            paragraph.style.text_align,
            paragraph.style.direction,
        ))
        .when(paragraph.style.nowrap, |element| {
            // Unwrappable text also must not be shrunk below its own width.
            element.whitespace_nowrap().flex_shrink_0()
        });
    if let Some(background) =
        render_box_background_image(&paragraph.style, options, &text_element_id)
    {
        element = element.child(background);
    }
    let rendered = rendered_text(paragraph, options, font_size, line_height);
    element = element.child(
        div()
            .relative()
            .top(px(baseline_shift))
            .child(rendered.into_element(text_element_id)),
    );
    let element = apply_box_style(element, &paragraph.style, options);
    if paragraph.style.display == MailDisplay::InlineBlock {
        return super::overlay::overlay(align_inline_block(element, &paragraph.style))
            .into_any_element();
    }
    element.into_any_element()
}

/// An `inline-block` box shrinks to fit its content instead of filling the
/// block that contains it — a rating pill is 33px wide, not the width of the
/// card. Aligning it rather than fixing its width is what keeps long text in
/// such a box still wrapping at the container.
fn align_inline_block(element: Div, style: &MailStyle) -> Div {
    match style.line_align.resolve(style.direction) {
        MailTextAlign::Center => element.self_center(),
        MailTextAlign::Right => element.self_end(),
        _ => element.self_start(),
    }
}

pub(super) fn homogeneous_inline_font_size(paragraph: &MailParagraph) -> Option<f32> {
    homogeneous_inline_metric(paragraph, |run| run.style.font_size)
}

/// The family the text is set in: the one every run agrees on, as with the
/// size, or the paragraph's own.
pub(super) fn text_font_family(
    paragraph: &MailParagraph,
    options: &MailRenderOptions,
) -> gpui::SharedString {
    rendered_font_family(
        homogeneous_inline_metric(paragraph, |run| run.style.font_family)
            .or(paragraph.style.font_family),
        options,
    )
}

pub(super) fn homogeneous_inline_line_height(paragraph: &MailParagraph) -> Option<MailLineHeight> {
    homogeneous_inline_metric(paragraph, |run| run.style.line_height)
}

fn homogeneous_inline_metric<T: Copy + PartialEq>(
    paragraph: &MailParagraph,
    metric: impl Fn(&MailTextRun) -> Option<T>,
) -> Option<T> {
    let mut value = None;
    for run in paragraph
        .runs
        .iter()
        // A non-breaking space is text that brings its font to the line.
        .filter(|run| {
            run.text
                .contains(|c| !matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{000C}'))
        })
    {
        let next = metric(run)?;
        if let Some(existing) = value {
            if existing != next {
                return None;
            }
        } else {
            value = Some(next);
        }
    }
    value
}

fn rendered_text(
    paragraph: &MailParagraph,
    options: &MailRenderOptions,
    font_size: f32,
    line_height: f32,
) -> RenderedMailText {
    let mut text = String::new();
    let mut highlights: Vec<TextHighlight> = Vec::new();
    let mut link_ranges = Vec::new();
    let mut link_targets = Vec::new();
    let family = text_font_family(paragraph, options);
    for run in &paragraph.runs {
        let start = text.len();
        let transform = effective_text_transform(run, paragraph.style.text_transform);
        let transformed = transform_text(run.text.as_str(), transform);
        let font = run_font(paragraph, run, &family, font_size);
        text.push_str(spaced_text(&transformed, paragraph, run, &font, options).as_str());
        let end = text.len();
        if start == end {
            continue;
        }
        if let Some(href) = &run.href {
            link_ranges.push(start..end);
            link_targets.push(href.clone());
        }
        let decoration = effective_text_decoration(run, paragraph.style.text_decoration);
        let highlight = text_run_highlight(run, decoration, options);
        if highlight != HighlightStyle::default() {
            highlights.push((start..end, highlight));
        }
    }
    let default_style = TextStyle {
        color: paragraph
            .style
            .color
            .map(hsla_color)
            .unwrap_or(options.text_color),
        font_family: text_font_family(paragraph, options),
        font_size: px(font_size).into(),
        line_height: relative((line_height / font_size.max(0.01)).max(0.0)),
        ..Default::default()
    };
    let text: SharedString = text.into();
    let styled_text =
        StyledText::new(text.clone()).with_default_highlights(&default_style, highlights);
    RenderedMailText {
        text,
        styled_text,
        link_ranges,
        link_targets,
        selection_color: options.link_color.opacity(0.18),
    }
}

pub(super) fn rendered_font_family(
    family: Option<MailFontFamily>,
    options: &MailRenderOptions,
) -> gpui::SharedString {
    match family {
        Some(MailFontFamily::Arial) => "Arial".into(),
        Some(MailFontFamily::Helvetica) => "Helvetica".into(),
        Some(MailFontFamily::HelveticaNeue) => "Helvetica Neue".into(),
        Some(MailFontFamily::Verdana) => "Verdana".into(),
        Some(MailFontFamily::Geneva) => "Geneva".into(),
        Some(MailFontFamily::Tahoma) => "Tahoma".into(),
        Some(MailFontFamily::TrebuchetMs) => "Trebuchet MS".into(),
        Some(MailFontFamily::Georgia) => "Georgia".into(),
        Some(MailFontFamily::LucidaGrande) => "Lucida Grande".into(),
        Some(MailFontFamily::CourierNew) => "Courier New".into(),
        Some(MailFontFamily::Serif) => "Times".into(),
        Some(MailFontFamily::Monospace) => "Menlo".into(),
        // `sans-serif` is a request for the platform's default sans, which is
        // what a browser resolves it to — not for the mail UI's own typeface.
        // Resolving it to the UI font made every `Calibri, sans-serif` email
        // (which is to say every one composed in Outlook) set in a wider face
        // and wrap a line early.
        Some(MailFontFamily::SansSerif) => "Helvetica".into(),
        // Named fonts, none of them available and no generic after them: a
        // browser draws its default standard font, which is Times.
        Some(MailFontFamily::Other) => "Times".into(),
        None => options.font_family.clone(),
    }
}

pub(super) fn text_run_highlight(
    run: &MailTextRun,
    decoration: MailTextDecoration,
    options: &MailRenderOptions,
) -> HighlightStyle {
    let mut highlight = HighlightStyle::default();
    if let Some(color) = run.style.color {
        highlight.color = Some(hsla_color(color));
    } else if run.href.is_some() {
        // Only supply the theme's link colour when the document styles none of
        // its own; forcing it discarded the email's own link palette, weight and
        // decoration.
        highlight.color = Some(options.link_color);
    }
    if let Some(weight) = run.style.font_weight {
        highlight.font_weight = Some(font_weight(weight));
    }
    if run.style.italic {
        highlight.font_style = Some(FontStyle::Italic);
    }
    if decoration.underline {
        highlight.underline = Some(UnderlineStyle {
            color: highlight.color,
            thickness: px(1.0),
            wavy: false,
        });
    }
    if decoration.line_through {
        highlight.strikethrough = Some(StrikethroughStyle {
            color: highlight.color,
            thickness: px(1.0),
        });
    }
    highlight
}

fn effective_text_decoration(
    run: &MailTextRun,
    paragraph_decoration: MailTextDecoration,
) -> MailTextDecoration {
    MailTextDecoration {
        underline: run.style.text_decoration.underline || paragraph_decoration.underline,
        line_through: run.style.text_decoration.line_through || paragraph_decoration.line_through,
    }
}

pub(super) fn effective_text_transform(
    run: &MailTextRun,
    paragraph_transform: MailTextTransform,
) -> MailTextTransform {
    if run.style.text_transform == MailTextTransform::None {
        paragraph_transform
    } else {
        run.style.text_transform
    }
}

/// The font a run is shaped in: the paragraph's family, the run's own size,
/// and its weight and slant with the paragraph's as the fallback.
pub(super) fn run_font(
    paragraph: &MailParagraph,
    run: &MailTextRun,
    family: &gpui::SharedString,
    base_size: f32,
) -> super::MailTextFont {
    super::MailTextFont {
        family: family.clone(),
        size: run.style.font_size.unwrap_or(base_size),
        weight: run
            .style
            .font_weight
            .or(paragraph.style.font_weight)
            .unwrap_or(MailFontWeight::Normal),
        italic: run.style.italic || paragraph.style.font_style == MailFontStyle::Italic,
    }
}

pub(super) fn transform_text(text: &str, transform: MailTextTransform) -> String {
    match transform {
        MailTextTransform::None => text.to_string(),
        MailTextTransform::Uppercase => text.to_uppercase(),
        MailTextTransform::Lowercase => text.to_lowercase(),
        MailTextTransform::Capitalize => capitalize_text(text),
    }
}
fn capitalize_text(text: &str) -> String {
    let mut capitalize_next = true;
    let mut transformed = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_alphanumeric() {
            if capitalize_next {
                transformed.extend(character.to_uppercase());
                capitalize_next = false;
            } else {
                transformed.push(character);
            }
        } else {
            transformed.push(character);
            capitalize_next = true;
        }
    }
    transformed
}

pub(crate) fn font_weight(weight: MailFontWeight) -> FontWeight {
    match weight {
        MailFontWeight::Normal => FontWeight::NORMAL,
        MailFontWeight::Medium => FontWeight::MEDIUM,
        MailFontWeight::Bold => FontWeight::BOLD,
    }
}

fn text_align(align: MailTextAlign, direction: MailDirection) -> TextAlign {
    match align.resolve(direction) {
        MailTextAlign::Start | MailTextAlign::End => {
            unreachable!("logical text alignment must resolve to a physical edge")
        }
        MailTextAlign::Left => TextAlign::Left,
        MailTextAlign::Center => TextAlign::Center,
        MailTextAlign::Right => TextAlign::Right,
    }
}
