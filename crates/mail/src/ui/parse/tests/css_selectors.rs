use super::{parse_mail_html, MailBlock};

pub(super) fn descendant_class_css_applies_to_matching_ancestor_scope() {
    let html = r#"
        <body>
          <style>
            .hse-section .hse-size-6 { width:300px !important; max-width:300px !important; }
            .hse-section .middle .hse-size-6 { height:40px; }
            .unused .hse-size-6 { width:123px; }
          </style>
          <div class="hse-section">
            <div class="hse-size-6">inside</div>
            <div class="middle"><div class="hse-size-6">inside middle</div></div>
          </div>
          <div class="hse-size-6">outside</div>
        </body>
    "#;
    let document = parse_mail_html(html).expect("html document");
    let [MailBlock::Container(section), MailBlock::Paragraph(outside)] = document.blocks.as_slice()
    else {
        panic!("expected section and outside paragraph");
    };
    let [MailBlock::Paragraph(inside), MailBlock::Container(middle)] = section.children.as_slice()
    else {
        panic!("expected section children");
    };
    let [MailBlock::Paragraph(inside_middle)] = middle.children.as_slice() else {
        panic!("expected middle child");
    };

    assert_eq!(
        (inside.style.width, inside.style.max_width),
        (Some(300.0), Some(300.0))
    );
    assert_eq!(
        (inside_middle.style.width, inside_middle.style.height),
        (Some(300.0), Some(40.0))
    );
    assert_eq!((outside.style.width, outside.style.max_width), (None, None));
}

pub(super) fn child_css_applies_only_to_the_exact_email_ancestry() {
    let html = r#"
        <body>
          <style>
            .desktop > .mobile { display:none; color:#ff0000; }
            table > tbody > tr > td.hidden-cell { display:none; }
          </style>
          <div class="desktop"><div class="mobile">hidden nested copy</div></div>
          <div class="mobile">visible sibling copy</div>
          <table><tbody><tr>
            <td class="hidden-cell">hidden table copy</td>
            <td>visible table copy</td>
          </tr></tbody></table>
        </body>
    "#;
    let document = parse_mail_html(html).expect("html document");

    let text = document.plain_text();
    assert!(text.contains("visible sibling copy"));
    assert!(text.contains("visible table copy"));
    assert!(!text.contains("hidden nested copy"));
    assert!(!text.contains("hidden table copy"));
    let MailBlock::Paragraph(visible) = &document.blocks[0] else {
        panic!("outside .mobile should remain visible");
    };
    assert_eq!(visible.style.color, None);
}

pub(super) fn adjacent_css_matches_actual_inline_and_table_siblings() {
    let html = r#"
        <body>
          <style>
            p.notice + p.notice { color:#123456; }
            span.label + span.value { color:#abcdef; }
            td.label + td.hidden { display:none; }
          </style>
          <p class="notice">first paragraph</p>
          <p class="notice">second paragraph</p>
          <p><span class="label">label</span><span class="value">value</span></p>
          <table><tr>
            <td class="label">first cell</td>
            <td class="hidden">hidden cell</td>
            <td>visible cell</td>
          </tr></table>
        </body>
    "#;
    let document = parse_mail_html(html).expect("html document");
    let [MailBlock::Paragraph(first), MailBlock::Paragraph(second), MailBlock::Paragraph(inline), MailBlock::Table(table)] =
        document.blocks.as_slice()
    else {
        panic!("expected three paragraphs and one table");
    };

    assert_eq!(first.style.color, None);
    assert_eq!(second.style.color, Some(0x123456));
    assert_eq!(inline.runs.len(), 2);
    assert_eq!(inline.runs[0].style.color, None);
    assert_eq!(inline.runs[1].style.color, Some(0xabcdef));
    assert_eq!(table.rows[0].cells.len(), 2);
    assert_eq!(
        table.rows[0].cells[0].children[0].plain_text(),
        "first cell"
    );
    assert_eq!(
        table.rows[0].cells[1].children[0].plain_text(),
        "visible cell"
    );
}
