use super::{
    image::{background_image_object_fit, image_display_size, pending_image_should_show_alt},
    layout::table_cell_uses_inline_flow,
    text::{homogeneous_inline_font_size, homogeneous_inline_line_height},
};
use crate::ui::render::*;
use gpui::{point, size, RenderImage, TestAppContext};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

mod image_properties;
mod style_properties;
mod table_layout;

#[gpui::test]
fn root_paragraph_margins_collapse_to_the_larger_margin(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let measured_height = Rc::new(Cell::new(0.0));
    let measured_height_listener = Rc::clone(&measured_height);
    let paragraph = |text: &str, top: f32, bottom: f32| {
        MailBlock::Paragraph(MailParagraph {
            runs: vec![MailTextRun::text(text)],
            style: MailStyle {
                font_size: Some(16.0),
                line_height: Some(MailLineHeight::Pixels(20.0)),
                margin: MailEdgeInsets {
                    top,
                    right: 0.0,
                    bottom,
                    left: 0.0,
                },
                ..Default::default()
            },
        })
    };
    let document = MailDocument::from_blocks(vec![
        paragraph("First", 0.0, 20.0),
        paragraph("Second", 10.0, 0.0),
    ]);

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(240.0), px(100.0)),
        |_, _| {
            div()
                .w(px(240.0))
                .on_children_prepainted(move |bounds, _, _| {
                    measured_height_listener.set(
                        bounds
                            .first()
                            .map(|bounds| bounds.size.height.as_f32())
                            .unwrap_or_default(),
                    );
                })
                .child(render_mail_document(&document, mail_render_options()))
                .into_any_element()
        },
    );

    assert!(
        (measured_height.get() - 60.0).abs() <= 1.0,
        "root paragraph margins should collapse to 20px; got {}px",
        measured_height.get()
    );
}

#[gpui::test]
fn styled_container_height_does_not_clip_taller_body(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let measured_height = Rc::new(Cell::new(0.0));
    let measured_height_listener = Rc::clone(&measured_height);
    let document = MailDocument::from_blocks(vec![MailBlock::Container(MailContainer {
        link_target: None,
        style: MailStyle {
            height: Some(20.0),
            ..Default::default()
        },
        children: (0..5)
            .map(|index| {
                MailBlock::Paragraph(MailParagraph {
                    runs: vec![MailTextRun::text(format!("Line {index}"))],
                    style: MailStyle::default(),
                })
            })
            .collect(),
    })]);

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(240.0), px(240.0)),
        |_, _| {
            div()
                .w(px(200.0))
                .on_children_prepainted(move |bounds, _, _| {
                    let height = bounds
                        .first()
                        .map(|bounds| bounds.size.height.as_f32())
                        .unwrap_or_default();
                    measured_height_listener.set(height);
                })
                .child(render_mail_document(&document, mail_render_options()))
                .into_any_element()
        },
    );

    assert!(
        measured_height.get() > 80.0,
        "mail document height should expand beyond fixed HTML container height"
    );
}

#[gpui::test]
fn table_cell_background_image_is_resolved_when_cell_has_content(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let image_url = "https://example.test/hero.png";
    let resolved_count = Arc::new(AtomicUsize::new(0));
    let resolved_count_for_resolver = Arc::clone(&resolved_count);
    let expected_url = image_url.to_string();
    let document = MailDocument::from_blocks(vec![MailBlock::Table(MailTable {
        style: MailStyle::default(),
        rows: vec![MailTableRow {
            cells: vec![MailTableCell {
                children: vec![MailBlock::Paragraph(MailParagraph {
                    runs: vec![MailTextRun::text("Hero copy")],
                    style: MailStyle::default(),
                })],
                style: MailStyle {
                    background_image_url: Some(image_url.to_string()),
                    width: Some(200.0),
                    height: Some(80.0),
                    ..Default::default()
                },
                colspan: 1,
            }],
        }],
        cell_spacing: 0.0,
        row_spacing: 0.0,
        cell_padding: 0.0,
    })]);
    let mut options = mail_render_options();
    options.image_resolver = Some(Arc::new(move |url| {
        assert_eq!(url, expected_url.as_str());
        resolved_count_for_resolver.fetch_add(1, Ordering::Relaxed);
        None
    }));

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(240.0), px(120.0)),
        |_, _| {
            div()
                .w(px(240.0))
                .child(render_mail_document(&document, options.clone()))
                .into_any_element()
        },
    );

    assert_eq!(
        resolved_count.load(Ordering::Relaxed),
        1,
        "table-cell background image should be resolved for rendering"
    );
}

#[gpui::test]
fn resolved_images_without_declared_height_use_intrinsic_aspect_ratio(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let image_url = "https://example.test/hero.png";
    let measured_height = Rc::new(Cell::new(0.0));
    let measured_height_listener = Rc::clone(&measured_height);
    let resolved_image = test_render_image(400, 200);
    let expected_url = image_url.to_string();
    let block = MailBlock::Image(MailImage {
        src: image_url.to_string(),
        alt: String::new(),
        width: Some(200.0),
        height: None,
        style: MailStyle::default(),
    });
    let mut options = mail_render_options();
    options.image_resolver = Some(Arc::new(move |url| {
        assert_eq!(url, expected_url.as_str());
        Some(Arc::clone(&resolved_image))
    }));

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(240.0), px(160.0)),
        |_, _| {
            div()
                .w(px(200.0))
                .on_children_prepainted(move |bounds, _, _| {
                    let height = bounds
                        .first()
                        .map(|bounds| bounds.size.height.as_f32())
                        .unwrap_or_default();
                    measured_height_listener.set(height);
                })
                .child(render_mail_document_block(&block, options.clone()))
                .into_any_element()
        },
    );

    let height = measured_height.get();
    assert!(
        (height - 100.0).abs() < 0.5,
        "resolved image height should use natural aspect ratio; got {height}"
    );
}

#[gpui::test]
fn responsive_email_image_uses_actual_parent_width(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let image_url = "https://example.test/responsive.png";
    let measured_size = Rc::new(Cell::new((0.0, 0.0)));
    let measured_size_listener = Rc::clone(&measured_size);
    let resolved_image = test_render_image(560, 168);
    let expected_url = image_url.to_string();
    let block = MailBlock::Image(MailImage {
        src: image_url.to_string(),
        alt: String::new(),
        width: Some(560.0),
        height: Some(168.0),
        style: MailStyle {
            width: Some(560.0),
            max_width_percent: Some(1.0),
            ..Default::default()
        },
    });
    let mut options = mail_render_options();
    // The content width is the width of the box the document is drawn into, as
    // it is in the real surface: `max-width: 100%` resolves against it.
    options.content_width = 280.0;
    options.image_resolver = Some(Arc::new(move |url| {
        assert_eq!(url, expected_url.as_str());
        Some(Arc::clone(&resolved_image))
    }));

    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(280.0), px(160.0)),
        |_, _| {
            div()
                .w(px(280.0))
                .on_children_prepainted(move |bounds, _, _| {
                    if let Some(bounds) = bounds.first() {
                        measured_size_listener
                            .set((bounds.size.width.as_f32(), bounds.size.height.as_f32()));
                    }
                })
                .child(render_mail_document_block(&block, options.clone()))
                .into_any_element()
        },
    );

    let (width, height) = measured_size.get();
    assert!(
        width <= 280.0,
        "responsive image wrapper overflowed: {width}"
    );
    assert!(
        (height - 84.0).abs() <= 1.0,
        "responsive image must preserve its 10:3 aspect ratio; got {width}x{height}"
    );
}

#[test]
fn zero_width_border_style_does_not_paint_box_border() {
    let style = MailStyle {
        border_color: Some(0xd8dde6),
        border_width: Some(0.0),
        ..Default::default()
    };

    assert_eq!(super::box_border_width(&style), 0.0);
}

#[test]
fn homogeneous_inline_metrics_drive_paragraph_metrics() {
    let paragraph = MailParagraph {
        runs: vec![MailTextRun {
            text: "$10,142.48".to_string(),
            href: None,
            style: MailInlineStyle {
                font_size: Some(36.0),
                line_height: Some(MailLineHeight::Pixels(40.0)),
                font_weight: Some(MailFontWeight::Bold),
                ..Default::default()
            },
        }],
        style: MailStyle::default(),
    };

    assert_eq!(homogeneous_inline_font_size(&paragraph), Some(36.0));
    assert_eq!(
        homogeneous_inline_line_height(&paragraph),
        Some(MailLineHeight::Pixels(40.0))
    );
}

#[test]
fn homogeneous_relative_inline_line_height_is_preserved() {
    let paragraph = MailParagraph {
        runs: vec![MailTextRun {
            text: "Best,\nThe ChatGPT team".to_string(),
            href: None,
            style: MailInlineStyle {
                line_height: Some(MailLineHeight::Number(1.5)),
                ..Default::default()
            },
        }],
        style: MailStyle::default(),
    };

    assert_eq!(
        homogeneous_inline_line_height(&paragraph),
        Some(MailLineHeight::Number(1.5))
    );
}

/// Approximates Arial advance widths so intrinsic sizing is deterministic in
/// tests, independent of whatever fonts the headless renderer happens to ship.
fn test_text_measure() -> crate::ui::MailTextMeasure {
    std::sync::Arc::new(|text: &str, font: &crate::ui::MailTextFont| {
        // Bold glyphs are wider; the ratio keeps the approximation self-consistent.
        let weight = if font.weight == MailFontWeight::Bold {
            0.55
        } else {
            0.5
        };
        text.chars().count() as f32 * font.size * weight
    })
}

fn mail_render_options() -> MailRenderOptions {
    MailRenderOptions {
        text_color: Hsla::from(rgb(0x111111)),
        muted_text_color: Hsla::from(rgb(0x666666)),
        link_color: Hsla::from(rgb(0x2f79ff)),
        border_color: Hsla::from(rgb(0xd8dde6)),
        placeholder_bg: Hsla::from(rgb(0xf5f7fa)),
        font_family: "Arial".into(),
        base_font_size: 14.0,
        line_height: 19.0,
        content_width: 200.0,
        image_resolver: None,
        measure_text: test_text_measure(),
    }
}

fn test_render_image(width: u32, height: u32) -> Arc<RenderImage> {
    let pixels = ::image::RgbaImage::from_pixel(width, height, ::image::Rgba([0, 0, 0, 255]));
    // Through the same constructor the resolver uses, so the frame carries the
    // ring the renderer inset by.
    crate::ui::support::padded_image(Arc::new(RenderImage::new(vec![::image::Frame::new(
        pixels,
    )])))
    .expect("padded test image")
}
