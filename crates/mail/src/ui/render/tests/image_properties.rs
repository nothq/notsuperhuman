use crate::ui::types::MailDisplay;
use gpui::ObjectFit;

use super::{
    background_image_object_fit, image_display_size, mail_render_options,
    pending_image_should_show_alt, table_cell_uses_inline_flow, test_render_image,
    MailBackgroundSize, MailBlock, MailImage, MailParagraph, MailStyle, MailTableCell, MailTextRun,
};

#[test]
fn resolved_images_without_declared_width_use_intrinsic_aspect_ratio() {
    let resolved_image = test_render_image(206, 51);
    let image = MailImage {
        src: "https://example.test/logo.png".to_string(),
        alt: "ChatGPT".to_string(),
        width: None,
        height: Some(36.0),
        style: MailStyle::default(),
    };

    let options = mail_render_options();
    let (width, height) = image_display_size(
        &image,
        &options,
        Some(resolved_image.as_ref()),
        options.content_width,
    );

    assert!((width - (36.0 * 206.0 / 51.0)).abs() < 0.5);
    assert!((height - 36.0).abs() < 0.5);
}

#[test]
fn icon_sized_pending_images_do_not_show_alt_text() {
    let image = MailImage {
        src: "https://example.test/invoice.png".to_string(),
        alt: "invoice illustration".to_string(),
        width: Some(94.0),
        height: Some(91.0),
        style: MailStyle::default(),
    };

    assert!(!pending_image_should_show_alt(&image, 94.0, 91.0));
    assert!(pending_image_should_show_alt(&image, 200.0, 91.0));
}

#[test]
fn small_image_and_text_table_cell_uses_inline_flow() {
    let cell = MailTableCell {
        children: vec![
            MailBlock::Image(MailImage {
                src: "https://example.test/download-icon.png".to_string(),
                alt: String::new(),
                width: Some(12.0),
                height: Some(12.0),
                style: MailStyle::default(),
            }),
            MailBlock::Paragraph(MailParagraph {
                runs: vec![MailTextRun::text("Download invoice")],
                // Bare text in a cell is an anonymous inline box, as the parser builds it.
                style: MailStyle {
                    display: MailDisplay::Inline,
                    ..Default::default()
                },
            }),
        ],
        style: MailStyle::default(),
        colspan: 1,
    };

    assert!(table_cell_uses_inline_flow(&cell));
}

#[test]
fn background_image_fit_preserves_css_background_size() {
    let contain_style = MailStyle {
        background_size: MailBackgroundSize::Contain,
        ..Default::default()
    };
    let cover_style = MailStyle {
        background_size: MailBackgroundSize::Cover,
        ..Default::default()
    };

    assert!(matches!(
        background_image_object_fit(&contain_style),
        Some(ObjectFit::Contain)
    ));
    assert!(matches!(
        background_image_object_fit(&cover_style),
        Some(ObjectFit::Cover)
    ));
    // `auto` is the natural size, which no object fit expresses.
    assert!(background_image_object_fit(&MailStyle::default()).is_none());
}
