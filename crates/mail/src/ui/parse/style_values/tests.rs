use crate::ui::types::{
    MailBorderCollapse, MailBoxSizing, MailDisplay, MailFloat, MailFontFamily, MailFontStyle,
    MailLineHeight, MailOverflow, MailPosition, MailStyle, MailTextTransform, MailWordBreak,
};

use super::apply_style_attr;

#[test]
fn display_inline_block_and_float_styles_are_preserved() {
    let mut style = MailStyle::default();

    apply_style_attr("display:inline-block;float:right", &mut style);

    assert_eq!(style.display, MailDisplay::InlineBlock);
    assert_eq!(style.display, MailDisplay::InlineBlock);
    assert_eq!(style.float, MailFloat::Right);
}

#[test]
fn table_block_and_inline_display_styles_are_preserved() {
    let mut table = MailStyle::default();
    let mut block = MailStyle::default();
    let mut inline = MailStyle::default();

    apply_style_attr("display:table", &mut table);
    apply_style_attr("display:block", &mut block);
    apply_style_attr("display:inline", &mut inline);

    assert_eq!(table.display, MailDisplay::Table);
    assert_eq!(block.display, MailDisplay::Block);
    assert_eq!(inline.display, MailDisplay::Inline);
}

#[test]
fn typed_mail_css_properties_are_preserved() {
    let mut style = MailStyle::default();

    apply_style_attr(
        "font-family:Arial, sans-serif; font-style:italic; word-break:break-word;\
         border-collapse:collapse; box-sizing:border-box; max-height:0;\
         position:absolute; clip:rect(1px, 1px, 1px, 1px); overflow:hidden;\
         text-transform:uppercase; border-left-width:2px; border-left-color:#23496d",
        &mut style,
    );

    assert_eq!(style.font_family, Some(MailFontFamily::Arial));
    assert_eq!(style.font_style, MailFontStyle::Italic);
    assert_eq!(style.word_break, MailWordBreak::BreakWord);
    assert_eq!(style.border_collapse, MailBorderCollapse::Collapse);
    assert_eq!(style.box_sizing, MailBoxSizing::BorderBox);
    assert_eq!(style.max_height, Some(0.0));
    assert_eq!(style.position, MailPosition::Absolute);
    assert_eq!(style.clip.as_deref(), Some("rect(1px, 1px, 1px, 1px)"));
    assert_eq!(style.overflow, MailOverflow::Hidden);
    assert_eq!(style.text_transform, MailTextTransform::Uppercase);
    assert_eq!(style.border_edges.left.width, Some(2.0));
    assert_eq!(style.border_edges.left.color, Some(0x23496d));
}

#[test]
fn font_stack_uses_first_supported_email_font() {
    let mut style = MailStyle::default();

    apply_style_attr(
        "font-family: Colfax, Helvetica, Arial, sans-serif",
        &mut style,
    );

    assert_eq!(style.font_family, Some(MailFontFamily::Helvetica));
}

#[test]
fn relative_line_height_remains_relative_until_rendering() {
    let mut unitless = MailStyle::default();
    let mut percentage = MailStyle::default();

    apply_style_attr("line-height:1.5", &mut unitless);
    apply_style_attr("line-height:150%", &mut percentage);

    assert_eq!(unitless.line_height, Some(MailLineHeight::Number(1.5)));
    assert_eq!(
        percentage.line_height,
        Some(MailLineHeight::Percentage(1.5))
    );

    unitless.font_size = Some(20.0);
    percentage.font_size = Some(20.0);
    let mut unitless_child = MailStyle::inherit_text(&unitless);
    let mut percentage_child = MailStyle::inherit_text(&percentage);
    unitless_child.font_size = Some(10.0);
    percentage_child.font_size = Some(10.0);

    assert_eq!(
        unitless_child
            .line_height
            .expect("unitless line height")
            .resolve(10.0),
        15.0
    );
    assert_eq!(
        percentage_child
            .line_height
            .expect("percentage line height")
            .resolve(10.0),
        30.0
    );
}

#[test]
fn vendor_and_mso_css_properties_are_intentionally_ignored() {
    let mut style = MailStyle::default();

    apply_style_attr(
        "mso-padding-alt:12px 18px; -webkit-text-size-adjust:100%; -moz-box-sizing:border-box",
        &mut style,
    );

    assert_eq!(style, MailStyle::default());
}
