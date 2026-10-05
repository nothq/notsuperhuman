use crate::ui::types::{MailBackgroundSize, MailBlock, MailDisplay, MailTextAlign};

use self::support::{
    classed_layout_html, document_contains_image, document_contains_table,
    first_background_image_style, first_table, image_blocks, table_with_row_spacing,
    table_with_width,
};
use super::{parse_mail_html, parse_plain_mail_body};

mod authored_content;
mod css_selectors;
mod style_properties;
mod support;

#[test]
fn html_tables_remain_structured() {
    let html = r##"
            <body style="background-color:#f4f4f4">
              <table width="600" cellpadding="12" style="background:#ffffff;border-radius:8px">
                <tr><td><img src="https://example.test/logo.png" width="120" height="40" /></td></tr>
                <tr>
                  <td style="background:#f7fafc"><strong>Sales</strong><br/>42</td>
                  <td style="background:#f7fafc"><strong>Labor</strong><br/>8h</td>
                </tr>
              </table>
            </body>
        "##;

    let document = parse_mail_html(html).expect("html document");

    assert!(document.is_rich_layout);
    assert_eq!(document.image_urls, vec!["https://example.test/logo.png"]);
    assert!(document_contains_table(&document.blocks));
}

#[test]
fn unsafe_html_is_dropped() {
    let html = r#"<body><script>alert(1)</script><p onclick="x()">Hi <a href="javascript:bad()">bad</a></p><img src="javascript:bad()" /></body>"#;

    let document = parse_mail_html(html).expect("html document");

    assert_eq!(document.image_urls, Vec::<String>::new());
    let paragraphs = document.text_paragraphs();
    assert_eq!(paragraphs.len(), 1);
    assert_eq!(paragraphs[0][0].text, "Hi bad");
    assert_eq!(paragraphs[0][0].href, None);
}

#[test]
fn display_none_important_hidden_preheader_is_dropped() {
    let html = r#"
        <body>
          <table>
            <tr>
              <td style="display:none!important;color:#fff;font-size:1px;line-height:1px;max-height:0;overflow:hidden">hidden preheader content</td>
            </tr>
            <tr><td>Visible invoice</td></tr>
          </table>
        </body>
    "#;

    let document = parse_mail_html(html).expect("html document");

    assert_eq!(document.plain_text(), "Visible invoice");
}

#[test]
fn white_space_pre_wrap_preserves_line_breaks() {
    let html = r#"<body><span style="white-space:pre-wrap">Line one
Line two

Line four</span></body>"#;

    let document = parse_mail_html(html).expect("html document");

    assert_eq!(document.plain_text(), "Line one\nLine two\n\nLine four");
}

#[test]
fn background_shorthand_preserves_color_image_and_percentage_radius() {
    let image_url = "https://example.test/logo.png";
    let html = format!(
        r#"<body><div style="border-radius:100%;width:32px;height:32px;background-color:white;background:url('{image_url}');background-size:contain"></div></body>"#
    );

    let document = parse_mail_html(&html).expect("html document");
    let style = first_background_image_style(&document.blocks).expect("background image style");

    assert_eq!(document.image_urls, vec![image_url]);
    assert_eq!(style.background_color, Some(0xffffff));
    assert_eq!(style.background_image_url.as_deref(), Some(image_url));
    assert_eq!(style.background_size, MailBackgroundSize::Contain);
    assert!(
        style.border_radius.unwrap_or_default() > 1000.0,
        "percentage radius should remain round even before width is parsed"
    );
}

#[test]
fn nested_logo_table_preserves_background_image_style() {
    let image_url = "https://example.test/logo.png";
    let html = format!(
        r#"
        <body>
          <table cellpadding="0" cellspacing="0">
            <tr>
              <td valign="middle" height="32" style="height:32px">
                <a href="https://example.test">
                  <div>
                    <div style="border-radius:100%;width:32px;height:32px;text-align:center;background-color:white;background-position:center;background:url('{image_url}');background-size:contain;background-position:center;background-repeat:no-repeat;line-height:100%">
                      <div><div></div></div>
                    </div>
                  </div>
                </a>
              </td>
              <td style="width:12px"><span>&nbsp;</span></td>
              <td valign="middle"><span>Nash Technologies Inc.</span></td>
            </tr>
          </table>
        </body>
        "#
    );

    let document = parse_mail_html(&html).expect("html document");
    let style = first_background_image_style(&document.blocks).expect("logo background style");

    assert_eq!(document.image_urls, vec![image_url]);
    assert_eq!(style.width, Some(32.0));
    assert_eq!(style.height, Some(32.0));
    assert_eq!(style.background_color, Some(0xffffff));
    assert_eq!(style.background_image_url.as_deref(), Some(image_url));
    assert_eq!(style.background_size, MailBackgroundSize::Contain);
}

#[test]
fn plain_body_falls_back_to_preview() {
    let document = parse_plain_mail_body("", "Preview body");

    assert_eq!(document.plain_text(), "Preview body");
    assert!(!document.is_rich_layout);
}

#[test]
fn email_table_layout_attrs_are_preserved() {
    let html = r##"
            <body>
              <table align="center" width="100%" cellpadding="24" cellspacing="8" style="background:#f2f4f7">
                <tr>
                  <td width="50%" align="center"><strong>4</strong><br/>Scheduled employees</td>
                  <td width="50%" align="center"><strong>0</strong><br/>Employees off</td>
                </tr>
              </table>
            </body>
        "##;

    let document = parse_mail_html(html).expect("html document");
    let table = first_table(&document.blocks).expect("table");

    assert_eq!(table.style.background_color, Some(0xf2f4f7));
    assert_eq!(table.style.width_percent, Some(1.0));
    assert!(table.style.margin_left_auto);
    assert!(table.style.margin_right_auto);
    assert_eq!(table.style.text_align, MailTextAlign::Start);
    assert_eq!(table.cell_padding, 24.0);
    assert_eq!(table.cell_spacing, 8.0);
    assert_eq!(table.row_spacing, 8.0);
    assert_eq!(table.rows[0].cells[0].style.width_percent, Some(0.5));
    assert_eq!(table.rows[0].cells[0].style.padding.left, 24.0);
    assert_eq!(
        table.rows[0].cells[0].style.text_align,
        MailTextAlign::Center
    );
}

#[test]
fn legacy_table_cell_alignment_does_not_leak_into_nested_cells() {
    let document = parse_mail_html(
        r#"
        <table><tr><td align="center">
          centered direct text
          <table><tr><td><p>nested default text</p></td></tr></table>
        </td></tr></table>
        "#,
    )
    .expect("email HTML should parse");
    let outer = first_table(&document.blocks).expect("outer table");
    let outer_cell = &outer.rows[0].cells[0];
    let nested = outer_cell
        .children
        .iter()
        .find_map(|block| match block {
            MailBlock::Table(table) => Some(table),
            _ => None,
        })
        .expect("nested table");
    let nested_cell = &nested.rows[0].cells[0];
    let MailBlock::Paragraph(nested_text) = &nested_cell.children[0] else {
        panic!("nested cell should contain text");
    };

    assert_eq!(outer_cell.style.text_align, MailTextAlign::Center);
    assert_eq!(nested_cell.style.text_align, MailTextAlign::Start);
    assert_eq!(nested_text.style.text_align, MailTextAlign::Start);
}

#[test]
fn html_uses_css_initial_font_size_and_resolves_em_spacing() {
    let document =
        parse_mail_html(r#"<p style="margin:0.4em 0 1.1875em; font-size:16px">Subscription</p>"#)
            .expect("email HTML should parse");
    let MailBlock::Paragraph(paragraph) = &document.blocks[0] else {
        panic!("paragraph should remain text");
    };

    assert_eq!(paragraph.style.font_size, Some(16.0));
    assert!((paragraph.style.margin.top - 6.4).abs() < 0.001);
    assert!((paragraph.style.margin.bottom - 19.0).abs() < 0.001);

    let unstyled = parse_mail_html("<p>Browser default</p>").expect("unstyled email HTML");
    let MailBlock::Paragraph(unstyled) = &unstyled.blocks[0] else {
        panic!("unstyled paragraph should remain text");
    };
    assert_eq!(unstyled.style.font_size, Some(16.0));

    let small = parse_mail_html(r#"<p style="margin:0.4em 0 1.1875em; font-size:13px">Footer</p>"#)
        .expect("small email paragraph");
    let MailBlock::Paragraph(small) = &small.blocks[0] else {
        panic!("small paragraph should remain text");
    };
    assert!((small.style.margin.top - 5.2).abs() < 0.001);
    assert!((small.style.margin.bottom - 15.4375).abs() < 0.001);
}

#[test]
fn inline_table_cell_box_style_is_not_reapplied_to_its_text() {
    let document = parse_mail_html(
        r#"
        <table width="100%">
          <tr>
            <td width="20%" style="padding:10px; border-bottom:1px solid #ddd; text-align:end">
              $200.00
            </td>
          </tr>
        </table>
        "#,
    )
    .expect("email HTML should parse");
    let table = first_table(&document.blocks).expect("table");
    let cell = &table.rows[0].cells[0];
    let MailBlock::Paragraph(text) = &cell.children[0] else {
        panic!("inline table-cell content should remain selectable text");
    };

    assert_eq!(cell.style.width_percent, Some(0.2));
    assert_eq!(cell.style.padding.left, 10.0);
    assert_eq!(cell.style.border_edges.bottom.width, Some(1.0));
    assert_eq!(cell.style.text_align, MailTextAlign::End);
    assert_eq!(text.style.width_percent, None);
    assert!(!text.style.padding.any());
    assert!(!text.style.border_edges.any());
    assert_eq!(text.style.text_align, MailTextAlign::End);
}

#[test]
fn boxed_email_link_remains_a_clickable_inline_layout_box() {
    let document = parse_mail_html(
        r#"
        <table><tr><td align="center">
          <a href="https://example.test/manage"
             style="color:#fff; background-color:#10a37f; border:12px solid #10a37f; display:inline-block; text-decoration:none">
            Manage subscription
          </a>
        </td></tr></table>
        "#,
    )
    .expect("email HTML should parse");
    let table = first_table(&document.blocks).expect("table");
    let MailBlock::Container(link) = &table.rows[0].cells[0].children[0] else {
        panic!("boxed anchor should remain a layout container");
    };
    let MailBlock::Paragraph(label) = &link.children[0] else {
        panic!("boxed anchor should keep selectable text");
    };

    assert_eq!(
        link.link_target.as_deref(),
        Some("https://example.test/manage")
    );
    assert_eq!(link.style.display, MailDisplay::InlineBlock);
    assert_eq!(link.style.background_color, Some(0x10a37f));
    assert_eq!(link.style.border_width, Some(12.0));
    assert_eq!(link.style.text_align, MailTextAlign::Center);
    assert!(label.runs.iter().all(|run| run.href.is_none()));
    assert_eq!(label.style.color, Some(0xffffff));
    assert!(!label.style.text_decoration.underline);
}

#[test]
fn email_style_rules_shape_classed_layout() {
    let document = parse_mail_html(classed_layout_html()).expect("html document");
    // The body's background is the canvas colour, carried on the document
    // rather than making the body a box of its own.
    assert_eq!(document.body_background, Some(0xf2f5f7));
    assert!(!document.plain_text().contains("hidden spacer"));

    let main_body = table_with_width(&document.blocks, 600.0).expect("main body table");
    assert_eq!(main_body.style.width, Some(600.0));

    let summary = table_with_row_spacing(&document.blocks, 20.0).expect("summary table");
    assert_eq!(summary.cell_spacing, 0.0);
    assert_eq!(summary.row_spacing, 20.0);
    assert_eq!(summary.rows[0].cells[0].style.width_percent, Some(0.5));
    assert_eq!(summary.rows[0].cells[1].style.width_percent, Some(0.5));

    let runs = document
        .text_paragraphs()
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let brand = runs
        .iter()
        .find(|run| run.text.contains("Daily summary"))
        .expect("brand-colored run");
    assert_eq!(brand.style.color, Some(0x0085ff));
    let link = runs
        .iter()
        .find(|run| run.text.contains("open report"))
        .expect("link run");
    assert_eq!(link.href.as_deref(), Some("https://example.test"));
    assert_eq!(link.style.color, Some(0x0085ff));
}

#[test]
fn descendant_class_css_applies_to_matching_ancestor_scope() {
    css_selectors::descendant_class_css_applies_to_matching_ancestor_scope();
}

#[test]
fn child_css_applies_only_to_the_exact_email_ancestry() {
    css_selectors::child_css_applies_only_to_the_exact_email_ancestry();
}

#[test]
fn adjacent_css_matches_actual_inline_and_table_siblings() {
    css_selectors::adjacent_css_matches_actual_inline_and_table_siblings();
}

#[test]
fn hubspot_layout_css_properties_survive_parsing() {
    style_properties::hubspot_layout_css_properties_survive_parsing();
}
