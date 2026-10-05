use crate::model::{MailBodySource, MailDisplayBody, MailMessage};
use crate::ui::types::{MailDirection, MailDisplay};
use crate::ui::{MailBlock, MailContainer};

use super::{body_paragraph_plain_text, mail_body_display, MailBodySegmentKind};

fn mail_message(body_text: &str, body_html: &str) -> MailMessage {
    MailMessage {
        subject: "Summer Sale".to_string(),
        preview: String::new(),
        body_text: body_text.to_string(),
        body_html: Some(body_html.to_string()),
        body_loaded: true,
        ..Default::default()
    }
}

#[test]
fn html_hubspot_columns_survive_cleanup_for_alternating_layout() {
    let display = mail_body_display(&mail_message(
        "raw fallback should not render",
        r##"
            <html>
              <head>
                <style>
                  @media only screen and (min-width:640px) {
                    .hse-column { display:table-cell; vertical-align:top; }
                    .hse-section .hse-size-6 { width:300px !important; max-width:300px !important; }
                  }
                </style>
              </head>
              <body>
                <div class="hse-section">
                  <div class="hse-column-container" style="direction:rtl">
                    <div class="hse-column hse-size-6" style="direction:ltr">
                      <table><tr><td>GPUs</td></tr></table>
                    </div>
                    <div class="hse-column hse-size-6" style="direction:ltr">
                      <table><tr><td><img alt="GPUs" src="https://example.test/gpu.png" width="260" /></td></tr></table>
                    </div>
                  </div>
                </div>
              </body>
            </html>
            "##,
    ));

    let rtl_container = first_container_matching(&display.document.blocks, |container| {
        container.style.direction == MailDirection::Rtl
    })
    .expect("rtl HubSpot column container should survive body cleanup");
    let column_count = rtl_container
        .children
        .iter()
        .filter(|block| {
            matches!(
                block,
                MailBlock::Container(container)
                    if container.style.display == MailDisplay::TableCell
                        && container.style.width == Some(300.0)
            )
        })
        .count();

    assert_eq!(column_count, 2);
}

#[test]
fn plaintext_body_linkifies_bare_urls() {
    let display = mail_body_display(&MailMessage {
        subject: "Activate your Parcel Developer Account".to_string(),
        preview: String::new(),
        display_body: MailDisplayBody::PlainText {
            text: "Activate your account\n\nOpen https://developer.parcel.example/activate?token=redacted.".to_string(),
            source: MailBodySource::CompatibilityRepair,
        },
        body_loaded: true,
        ..Default::default()
    });

    assert_eq!(display.paragraphs.len(), 2);
    assert_eq!(
        body_paragraph_plain_text(&display.paragraphs[1]),
        "Open https://developer.parcel.example/activate?token=redacted."
    );
    assert_eq!(display.paragraphs[1].segments.len(), 3);
    assert_eq!(display.paragraphs[1].segments[0].text, "Open ");
    assert_eq!(
        display.paragraphs[1].segments[1].text,
        "https://developer.parcel.example/activate?token=redacted"
    );
    assert_eq!(
        display.paragraphs[1].segments[1].kind,
        MailBodySegmentKind::Link
    );
    assert_eq!(display.paragraphs[1].segments[2].text, ".");
}

fn first_container_matching(
    blocks: &[MailBlock],
    predicate: impl Copy + Fn(&MailContainer) -> bool,
) -> Option<&MailContainer> {
    blocks.iter().find_map(|block| match block {
        MailBlock::Container(container) if predicate(container) => Some(container),
        MailBlock::Container(container) => first_container_matching(&container.children, predicate),
        MailBlock::Table(table) => table.rows.iter().find_map(|row| {
            row.cells
                .iter()
                .find_map(|cell| first_container_matching(&cell.children, predicate))
        }),
        MailBlock::Paragraph(_)
        | MailBlock::Image(_)
        | MailBlock::Rule(_)
        | MailBlock::Spacer(_) => None,
    })
}
