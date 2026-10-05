//! Intrinsic widths, as CSS defines them.
//!
//! Automatic table layout needs a min-content and a max-content width for every
//! cell before it can choose column widths, and neither is available from the
//! layout engine: GPUI's text elements report their full unwrapped width for
//! both. They are therefore computed here from the platform's font metrics.

use super::MailRenderOptions;
use crate::ui::types::*;

/// The two intrinsic sizes of a box: the narrowest it can be without breaking
/// content apart, and the width it takes with no wrapping at all.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct MailIntrinsicWidth {
    pub(super) min: f32,
    pub(super) max: f32,
}

impl MailIntrinsicWidth {
    fn zero() -> Self {
        Self { min: 0.0, max: 0.0 }
    }

    /// Boxes stacked in a block formatting context are as wide as the widest of
    /// them; neither bound accumulates.
    fn stacked(self, other: Self) -> Self {
        Self {
            min: self.min.max(other.min),
            max: self.max.max(other.max),
        }
    }

    /// Boxes on one line accumulate their max-content widths, while the line can
    /// still break between them, so the minimum is the widest single box.
    fn inline(self, other: Self) -> Self {
        Self {
            min: self.min.max(other.min),
            max: self.max + other.max,
        }
    }

    pub(super) fn expand(self, edges: f32) -> Self {
        Self {
            min: self.min + edges,
            max: self.max + edges,
        }
    }

    pub(super) fn clamp_to_style(self, style: &MailStyle, available: f32) -> Self {
        let mut bounds = self;
        if let Some(width) = intrinsic_definite_width(style) {
            bounds = Self {
                min: width,
                max: width,
            };
        }
        bounds.clamp_to_bounds(style, available)
    }

    /// Apply only the `min-width` and `max-width` constraints, leaving `width`
    /// to the caller: a table cell's declared width raises its column's
    /// preferred width rather than pinning both bounds to it.
    pub(super) fn clamp_to_bounds(self, style: &MailStyle, available: f32) -> Self {
        let mut bounds = self;
        if let Some(min_width) = style.min_width {
            bounds.min = bounds.min.max(min_width);
            bounds.max = bounds.max.max(min_width);
        }
        if let Some(max_width) = max_width_constraint(style, available) {
            bounds.min = bounds.min.min(max_width);
            bounds.max = bounds.max.min(max_width);
        }
        bounds
    }
}

/// A box's own horizontal padding and border, which sit outside its content.
pub(super) fn horizontal_edges(style: &MailStyle) -> f32 {
    let border = style.border_width.unwrap_or_default();
    let left = style
        .border_edges
        .left
        .width
        .or(Some(border))
        .unwrap_or(0.0);
    let right = style
        .border_edges
        .right
        .width
        .or(Some(border))
        .unwrap_or(0.0);
    style.padding.left + style.padding.right + left + right
}

/// The width a style pins the box to when computing an *intrinsic* contribution.
///
/// Percentages are deliberately excluded. CSS Sizing §5.2 treats a percentage
/// size as `auto` while computing intrinsic contributions, and it has to: the
/// percentage resolves against the containing block, so feeding the container's
/// width back into its own content's min-content width makes the two grow off
/// each other. A nested email table doubled in width at every level until it was
/// twenty thousand pixels wide.
pub(super) fn intrinsic_definite_width(style: &MailStyle) -> Option<f32> {
    style.width.map(|width| content_box_width(style, width))
}

/// The content width a declared width stands for.
///
/// CSS 2.1 §10.2 sizes the content box, so a box's own padding and border sit
/// outside its `width`; `box-sizing: border-box` folds them in instead. Every
/// intrinsic contribution here is a content width that has its edges added back
/// once, so a border-box width has to give them up first.
pub(super) fn content_box_width(style: &MailStyle, width: f32) -> f32 {
    match style.box_sizing {
        MailBoxSizing::ContentBox => width,
        MailBoxSizing::BorderBox => (width - horizontal_edges(style)).max(0.0),
    }
}

/// The tighter of the two `max-width` constraints a style can carry.
///
/// CSS 2.1 §10.4: a percentage `max-width` resolves against the containing
/// block. Responsive email writes `width: 700px; max-width: 100%` on the same
/// box constantly, and the percentage is what keeps the declared width from
/// pushing the box — and every box nested inside it — past its container.
pub(super) fn max_width_constraint(style: &MailStyle, available: f32) -> Option<f32> {
    let percent = style
        .max_width_percent
        .map(|fraction| (available * fraction).max(0.0));
    match (style.max_width, percent) {
        (Some(absolute), Some(relative)) => Some(absolute.min(relative)),
        (absolute, relative) => absolute.or(relative),
    }
}

/// The width a style pins the box to, if any, resolved against its container.
pub(super) fn definite_width(style: &MailStyle, available: f32) -> Option<f32> {
    if let Some(width) = style.width {
        return Some(width);
    }
    style
        .width_percent
        .map(|fraction| (available * fraction).max(0.0))
}

/// The used width of a block-level box, per CSS 2.1 §10.3.3 and §10.4.
///
/// An auto width fills the containing block minus this box's own edges and
/// margins. If that exceeds `max-width` the width is recomputed as `max-width`,
/// which is what lets auto margins then centre the box in the leftover space —
/// the `max-width: 600px; margin: 0 auto` idiom every email wrapper uses.
///
/// The result is a **border-box** width, because that is what GPUI applies and
/// what the caller subtracts this box's edges from to size its content. CSS
/// resolves and clamps the *content* width, so the edges are added back once at
/// the end — subtracting them here as well shrank every padded box twice over.
pub(super) fn block_used_width(style: &MailStyle, available: f32) -> f32 {
    let margins = if style.margin_left_auto {
        0.0
    } else {
        style.margin.left
    } + if style.margin_right_auto {
        0.0
    } else {
        style.margin.right
    };
    let edges = horizontal_edges(style);
    let mut content = definite_width(style, available)
        .map(|width| content_box_width(style, width))
        .unwrap_or(available - edges - margins);
    if let Some(max_width) = max_width_constraint(style, available) {
        content = content.min(content_box_width(style, max_width));
    }
    if let Some(min_width) = style.min_width {
        content = content.max(content_box_width(style, min_width));
    }
    content.max(0.0) + edges
}

pub(super) fn block_intrinsic_width(
    block: &MailBlock,
    options: &MailRenderOptions,
    available: f32,
) -> MailIntrinsicWidth {
    match block {
        MailBlock::Paragraph(paragraph) => paragraph_intrinsic_width(paragraph, options)
            .expand(horizontal_edges(&paragraph.style) + paragraph.style.margin.horizontal())
            .clamp_to_style(&paragraph.style, available),
        MailBlock::Container(container) => {
            let inner = available - horizontal_edges(&container.style);
            let children = children_intrinsic_width(&container.children, options, inner);
            children
                .expand(horizontal_edges(&container.style) + container.style.margin.horizontal())
                .clamp_to_style(&container.style, available)
        }
        MailBlock::Table(table) => super::layout::table_intrinsic_width(table, options, available),
        MailBlock::Image(image) => {
            // A percentage width says nothing about the image's own size, so
            // the pixels stand in for it, as a browser's table layout has them
            // do: two `width: 100%` photos of 256px make two equal columns,
            // where two zero-width photos left the columns to their captions.
            let declared = image
                .width
                .or(image.style.width)
                .or_else(|| intrinsic_definite_width(&image.style));
            let natural = declared
                .is_none()
                .then(|| {
                    let resolved = options.image_resolver.as_ref()?.as_ref()(&image.src)?;
                    super::image::natural_image_size(&resolved).map(|(width, _)| width)
                })
                .flatten();
            let width = declared.or(natural).unwrap_or(0.0);
            // A responsive image carries `max-width: 100%`; without honouring it
            // the image's full pixel width became the table's min-content width,
            // forcing the table past its container and handing that overflowed
            // width to everything nested inside it.
            // An image sized by its pixels alone can still shrink to nothing
            // under its percentage width, so only its max-content is the pixels.
            MailIntrinsicWidth {
                min: if declared.is_some() { width } else { 0.0 },
                max: width,
            }
            .expand(image.style.margin.horizontal())
            .clamp_to_style(&image.style, available)
        }
        MailBlock::Rule(_) | MailBlock::Spacer(_) => MailIntrinsicWidth::zero(),
    }
}

pub(super) fn children_intrinsic_width(
    blocks: &[MailBlock],
    options: &MailRenderOptions,
    available: f32,
) -> MailIntrinsicWidth {
    let inline = super::layout::mail_blocks_use_inline_flow(blocks);
    if !inline {
        return blocks
            .iter()
            .map(|block| block_intrinsic_width(block, options, available))
            .fold(MailIntrinsicWidth::zero(), |total, child| {
                total.stacked(child)
            });
    }
    // Inline siblings share a line until a spacer breaks it (a `<br>` between
    // a name and a button), and the lines stack: a card's width is its widest
    // line, not the name and the button laid end to end.
    let mut total = MailIntrinsicWidth::zero();
    let mut line = MailIntrinsicWidth::zero();
    for block in blocks {
        if matches!(block, MailBlock::Spacer(_)) {
            total = total.stacked(line);
            line = MailIntrinsicWidth::zero();
            continue;
        }
        line = line.inline(block_intrinsic_width(block, options, available));
    }
    total.stacked(line)
}

fn paragraph_intrinsic_width(
    paragraph: &MailParagraph,
    options: &MailRenderOptions,
) -> MailIntrinsicWidth {
    let family = super::text::text_font_family(paragraph, options);
    let base_size = paragraph
        .style
        .font_size
        .or_else(|| super::text::homogeneous_inline_font_size(paragraph))
        .unwrap_or(options.base_font_size);
    // Measured as rendered: `letter-spacing` is emulated with spacer characters,
    // and the same string is shaped here and at paint time.
    let measure = |text: &str, run: &MailTextRun, font: &super::MailTextFont| {
        (options.measure_text)(
            super::text::spaced_text(text, paragraph, run, font, options).as_str(),
            font,
        )
    };
    let mut min = 0.0f32;
    let mut line = 0.0f32;
    let mut max = 0.0f32;
    for run in &paragraph.runs {
        let font = super::text::run_font(paragraph, run, &family, base_size);
        // Measured as painted: an uppercased label is wider than its source
        // text, and a box sized to the source wrapped the label it then drew.
        let transform = super::text::effective_text_transform(run, paragraph.style.text_transform);
        let text = super::text::transform_text(run.text.as_str(), transform);
        for (index, segment) in text.split('\n').enumerate() {
            if index > 0 {
                // A hard break ends the line, so it bounds max-content too.
                max = max.max(line);
                line = 0.0;
            }
            line += measure(segment, run, &font);
            if paragraph.style.nowrap {
                min = min.max(line);
                continue;
            }
            for word in segment.split_whitespace() {
                min = min.max(measure(word, run, &font));
            }
        }
    }
    // Whole pixels, rounded up. Each run is measured on its own here while the
    // renderer shapes the whole line, and the two disagree by fractions of a
    // pixel; a column solved to exactly the measured width then wrapped text
    // that the browser fits on one line. Content never needs less than its
    // measurement, so rounding up is the safe side.
    MailIntrinsicWidth {
        min: min.ceil(),
        max: max.max(line).ceil(),
    }
}
