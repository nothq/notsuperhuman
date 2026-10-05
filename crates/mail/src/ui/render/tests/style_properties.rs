use super::{mail_render_options, test_render_image};
use crate::ui::render::*;

use super::super::{
    image::image_display_size,
    layout::mail_blocks_use_inline_flow,
    text::{text_run_highlight, transform_text},
};

#[test]
fn table_cell_display_columns_use_horizontal_box_flow() {
    let blocks = vec![
        MailBlock::Container(MailContainer {
            link_target: None,
            children: vec![MailBlock::Paragraph(MailParagraph {
                runs: vec![MailTextRun::text("GPUs")],
                style: MailStyle::default(),
            })],
            style: MailStyle {
                display: MailDisplay::TableCell,
                width: Some(300.0),
                max_width: Some(300.0),
                ..Default::default()
            },
        }),
        MailBlock::Container(MailContainer {
            link_target: None,
            children: vec![MailBlock::Image(MailImage {
                src: "https://example.test/gpu.png".to_string(),
                alt: String::new(),
                width: Some(260.0),
                height: Some(180.0),
                style: MailStyle::default(),
            })],
            style: MailStyle {
                display: MailDisplay::TableCell,
                width: Some(300.0),
                max_width: Some(300.0),
                ..Default::default()
            },
        }),
    ];

    assert!(mail_blocks_use_inline_flow(&blocks));
}

#[test]
fn product_image_keeps_declared_width_in_fixed_column() {
    let image = MailImage {
        src: "https://example.test/product.png".to_string(),
        alt: "Dedicated Servers".to_string(),
        width: Some(260.0),
        height: None,
        style: MailStyle {
            max_width_percent: Some(1.0),
            ..Default::default()
        },
    };
    let resolved_image = test_render_image(520, 360);
    let mut options = mail_render_options();
    options.content_width = 300.0;

    let (width, height) = image_display_size(
        &image,
        &options,
        Some(&resolved_image),
        options.content_width,
    );

    assert!(
        (width - 260.0).abs() < 0.5,
        "declared product image width should be preserved; got {width}"
    );
    assert!(
        (height - 180.0).abs() < 0.5,
        "height should use intrinsic aspect ratio at declared width; got {height}"
    );
}

#[test]
fn border_top_side_paints_without_full_box_border() {
    let style = MailStyle {
        border_edges: MailBorderEdges {
            top: MailBorderSide {
                color: Some(0x23496d),
                width: Some(1.0),
            },
            ..Default::default()
        },
        ..Default::default()
    };

    assert_eq!(super::super::box_border_width(&style), 0.0);
    assert_eq!(super::super::box_border_top_width(&style), 1.0);
}

#[test]
fn asymmetric_email_button_border_widths_are_preserved() {
    let style = MailStyle {
        border_edges: MailBorderEdges {
            top: MailBorderSide {
                color: Some(0x10a37f),
                width: Some(12.0),
            },
            right: MailBorderSide {
                color: Some(0x10a37f),
                width: Some(24.0),
            },
            bottom: MailBorderSide {
                color: Some(0x10a37f),
                width: Some(12.0),
            },
            left: MailBorderSide {
                color: Some(0x10a37f),
                width: Some(24.0),
            },
        },
        ..Default::default()
    };

    assert_eq!(super::super::box_border_top_width(&style), 12.0);
    assert_eq!(super::super::box_border_right_width(&style), 24.0);
    assert_eq!(super::super::box_border_bottom_width(&style), 12.0);
    assert_eq!(super::super::box_border_left_width(&style), 24.0);
}

#[test]
fn text_decoration_and_transform_rendering_helpers_apply_styles() {
    let run = MailTextRun {
        text: "Summer Sale".to_string(),
        href: None,
        style: MailInlineStyle {
            text_decoration: MailTextDecoration {
                underline: true,
                line_through: true,
            },
            ..Default::default()
        },
    };
    let highlight = text_run_highlight(&run, run.style.text_decoration, &mail_render_options());

    assert_eq!(
        transform_text("Summer Sale", MailTextTransform::Uppercase),
        "SUMMER SALE"
    );
    assert!(highlight.underline.is_some());
    assert!(highlight.strikethrough.is_some());
}

#[test]
fn inline_block_columns_use_horizontal_box_flow() {
    let blocks = vec![
        MailBlock::Container(MailContainer {
            link_target: None,
            children: vec![MailBlock::Paragraph(MailParagraph {
                runs: vec![MailTextRun::text("Object Storage")],
                style: MailStyle::default(),
            })],
            style: MailStyle {
                display: MailDisplay::InlineBlock,
                width_percent: Some(0.5),
                ..Default::default()
            },
        }),
        MailBlock::Container(MailContainer {
            link_target: None,
            children: vec![MailBlock::Image(MailImage {
                src: "https://example.test/storage.png".to_string(),
                alt: String::new(),
                width: Some(200.0),
                height: Some(200.0),
                style: MailStyle::default(),
            })],
            style: MailStyle {
                display: MailDisplay::InlineBlock,
                width_percent: Some(0.5),
                ..Default::default()
            },
        }),
    ];

    assert!(mail_blocks_use_inline_flow(&blocks));
}
