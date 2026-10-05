use super::{document_contains_image, first_table, image_blocks, parse_mail_html, MailBlock};

#[test]
fn linked_images_are_preserved_as_image_blocks() {
    let html = r##"
            <body>
              <div style="text-align:center">
                <a href="https://example.test"><img src="https://example.test/logo.png" width="200" height="65" /></a>
              </div>
            </body>
        "##;

    let document = parse_mail_html(html).expect("html document");

    assert_eq!(document.image_urls, vec!["https://example.test/logo.png"]);
    assert!(document_contains_image(&document.blocks));
}

#[test]
fn image_dimensions_use_the_final_css_cascade() {
    let html = r##"
        <head>
          <style>img { height:auto; }</style>
        </head>
        <body>
          <img
            src="https://example.test/openai.png"
            width="560"
            height="168"
            style="width:140px; max-width:100%"
          />
        </body>
    "##;

    let document = parse_mail_html(html).expect("html document");
    let images = image_blocks(&document.blocks);

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].width, Some(140.0));
    assert_eq!(images[0].height, None);
    assert_eq!(images[0].style.max_width_percent, Some(1.0));
}

#[test]
fn image_dimensions_preserve_presentational_attributes_without_css() {
    let html = r##"
        <body>
          <img src="https://example.test/logo.png" width="120" height="40" />
        </body>
    "##;

    let document = parse_mail_html(html).expect("html document");
    let images = image_blocks(&document.blocks);

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].width, Some(120.0));
    assert_eq!(images[0].height, Some(40.0));
}

#[test]
fn image_auto_width_overrides_presentational_width() {
    let html = r##"
        <body>
          <img
            src="https://example.test/logo.png"
            width="206"
            height="51"
            style="width:auto; height:36px"
          />
        </body>
    "##;

    let document = parse_mail_html(html).expect("html document");
    let images = image_blocks(&document.blocks);

    assert_eq!(images.len(), 1);
    assert_eq!(images[0].width, None);
    assert_eq!(images[0].height, Some(36.0));
}

#[test]
fn adjacent_paragraph_css_preserves_email_spacing() {
    let html = r##"
        <head><style>p { margin: 0; } p + p { margin-top: 16px; }</style></head>
        <body><p>First</p>
        <p>Second</p>
        <span>Divider</span>
        <p>Third</p></body>
    "##;

    let document = parse_mail_html(html).expect("html document");
    let [MailBlock::Paragraph(first), MailBlock::Paragraph(second), MailBlock::Paragraph(divider), MailBlock::Paragraph(third)] =
        document.blocks.as_slice()
    else {
        panic!("expected four paragraphs");
    };

    assert_eq!(first.style.margin.top, 0.0);
    assert_eq!(second.style.margin.top, 16.0);
    assert_eq!(divider.style.margin.top, 0.0);
    assert_eq!(third.style.margin.top, 0.0);
}

#[test]
fn nonbreaking_space_paragraph_preserves_email_blank_line() {
    let html = r##"<body><p>Before</p><p>&nbsp;</p><p>After</p></body>"##;

    let document = parse_mail_html(html).expect("html document");
    let [MailBlock::Paragraph(_), MailBlock::Paragraph(blank), MailBlock::Paragraph(_)] =
        document.blocks.as_slice()
    else {
        panic!("expected the authored blank paragraph to remain in layout");
    };

    assert_eq!(blank.runs.len(), 1);
    assert_eq!(blank.runs[0].text, "\u{00a0}");
}

#[test]
fn paragraph_preserves_an_email_image_child() {
    let document = parse_mail_html(
        r#"<p style="text-align:left;margin:0"><img src="https://example.test/logo.png" style="width:64px;height:auto"></p>"#,
    )
    .expect("email HTML should parse");

    let MailBlock::Container(paragraph) = &document.blocks[0] else {
        panic!("expected image paragraph container");
    };
    let MailBlock::Image(image) = &paragraph.children[0] else {
        panic!("expected image inside paragraph container");
    };
    assert_eq!(image.src, "https://example.test/logo.png");
    assert_eq!(image.width, Some(64.0));
}

#[test]
fn empty_painted_paragraph_preserves_its_email_layout_box() {
    let document = parse_mail_html(
        r#"<p style="height:1px;width:100%;background:#e5e5e5;margin:40px 0"></p>"#,
    )
    .expect("email HTML should parse");

    let MailBlock::Container(divider) = &document.blocks[0] else {
        panic!("expected painted paragraph container");
    };
    assert!(divider.children.is_empty());
    assert_eq!(divider.style.height, Some(1.0));
    assert_eq!(divider.style.width_percent, Some(1.0));
    assert_eq!(divider.style.background_color, Some(0xe5e5e5));
    assert_eq!(divider.style.margin.top, 40.0);
    assert_eq!(divider.style.margin.bottom, 40.0);
}

#[test]
fn inline_html_preserves_authored_whitespace_without_inserting_before_punctuation() {
    let html = r##"
        <body><p><strong>App</strong>: ChatGPT <strong>Web</strong> login</p></body>
    "##;

    let document = parse_mail_html(html).expect("html document");

    assert_eq!(document.plain_text(), "App: ChatGPT Web login");
}

#[test]
fn centered_table_cell_does_not_mutate_child_table_margins() {
    let html = r##"
        <body>
          <table width="100%"><tr><td align="center">
            <table width="560"><tr><td>Message</td></tr></table>
          </td></tr></table>
        </body>
    "##;

    let document = parse_mail_html(html).expect("html document");
    let outer = first_table(&document.blocks).expect("outer table");
    let inner = first_table(&outer.rows[0].cells[0].children).expect("inner table");

    assert_eq!(inner.style.width, Some(560.0));
    assert!(!inner.style.margin_left_auto);
    assert!(!inner.style.margin_right_auto);
}

#[test]
fn image_no_border_styles_do_not_create_synthetic_border() {
    let html = r##"
        <body>
          <img src="https://example.test/logo.png" width="200" style="border:none" />
          <img src="https://example.test/hero.png" width="200" border="0" style="border:0 solid #d8dde6" />
        </body>
    "##;

    let document = parse_mail_html(html).expect("html document");
    let images = image_blocks(&document.blocks);

    assert_eq!(images.len(), 2);
    assert_eq!(images[0].style.border_width, Some(0.0));
    assert_eq!(images[0].style.border_color, None);
    assert_eq!(images[1].style.border_width, Some(0.0));
    assert_eq!(images[1].style.border_color, Some(0xd8dde6));
}

#[test]
fn author_important_and_selector_specificity_control_email_widths() {
    let html = r##"
        <style>
          .fluid { width:auto !IMPORTANT; }
          .specific { width:200px; }
          img { width:300px; }
        </style>
        <body>
          <img class="fluid" src="https://example.test/fluid.png" width="560" style="width:140px" />
          <img class="specific" src="https://example.test/specific.png" />
        </body>
    "##;

    let document = parse_mail_html(html).expect("email HTML should parse");
    let images = image_blocks(&document.blocks);

    assert_eq!(images.len(), 2);
    assert_eq!(images[0].style.width, None);
    assert_eq!(images[0].style.width_percent, None);
    assert_eq!(images[1].style.width, Some(200.0));
}

#[test]
fn type_qualified_email_selectors_do_not_style_other_tags() {
    let document = parse_mail_html(
        r#"
        <style>img.fluid { width:200px; }</style>
        <body>
          <div class="fluid">Text</div>
          <img class="fluid" src="https://example.test/fluid.png" />
        </body>
        "#,
    )
    .expect("email HTML should parse");

    let MailBlock::Paragraph(text) = &document.blocks[0] else {
        panic!("non-image class match should remain an unstyled paragraph");
    };
    assert_eq!(text.style.width, None);
    let images = image_blocks(&document.blocks);
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].style.width, Some(200.0));
}

#[test]
fn css_comments_do_not_hide_the_first_email_selector() {
    let document = parse_mail_html(
        r#"
        <style>/* Email client reset. */ p { margin-top:11px; }</style>
        <body><p>Message</p></body>
        "#,
    )
    .expect("email HTML should parse");

    let MailBlock::Paragraph(paragraph) = &document.blocks[0] else {
        panic!("expected one paragraph");
    };
    assert_eq!(paragraph.style.margin.top, 11.0);
}

#[test]
fn id_email_selectors_apply_with_css_specificity() {
    let document = parse_mail_html(
        r#"
        <style>
          #bodyTable { max-width:560px; }
          table.wide { max-width:620px; }
          table#bodyTable { min-width:400px; }
          #bodyCell { padding:20px; }
        </style>
        <body>
          <table id="bodyTable" class="wide" width="100%">
            <tr><td id="bodyCell">Message</td></tr>
          </table>
        </body>
        "#,
    )
    .expect("email HTML should parse");

    let table = first_table(&document.blocks).expect("body table");
    assert_eq!(table.style.max_width, Some(560.0));
    assert_eq!(table.style.min_width, Some(400.0));
    assert_eq!(table.rows[0].cells[0].style.padding.top, 20.0);
    assert_eq!(table.rows[0].cells[0].style.padding.right, 20.0);
    assert_eq!(table.rows[0].cells[0].style.padding.bottom, 20.0);
    assert_eq!(table.rows[0].cells[0].style.padding.left, 20.0);
}

#[test]
fn mixed_paragraph_text_inherits_only_text_properties() {
    let document = parse_mail_html(
        r##"<p style="padding:12px;margin:8px;background:#f2f4f7;color:#23496d">before<img src="https://example.test/logo.png" width="20" height="20">after</p>"##,
    )
    .expect("email HTML should parse");

    let MailBlock::Container(paragraph_box) = &document.blocks[0] else {
        panic!("mixed paragraph should retain one layout container");
    };
    assert_eq!(paragraph_box.style.padding.left, 12.0);
    assert_eq!(paragraph_box.style.margin.left, 8.0);
    assert_eq!(paragraph_box.style.background_color, Some(0xf2f4f7));
    assert_eq!(paragraph_box.children.len(), 3);
    for child in [&paragraph_box.children[0], &paragraph_box.children[2]] {
        let MailBlock::Paragraph(text) = child else {
            panic!("direct text should remain a paragraph child");
        };
        assert_eq!(text.style.color, Some(0x23496d));
        assert!(!text.style.padding.any());
        assert!(!text.style.margin.any());
        assert_eq!(text.style.background_color, None);
    }
}
