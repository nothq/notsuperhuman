use super::{
    mail_preview_label, MailBodyDisplay, MailBodyParagraph, MailBodySegment, MailBodySegmentKind,
    MailMessage,
};
use crate::model::MailDisplayBody;
use crate::ui::parse_mail_html;
use crate::ui::MailBlock;
use crate::ui::MailDocument;
use crate::ui::MailParagraph;
use crate::ui::MailTextRun;

const SUPERHUMAN_REFERRAL_URL: &str = "mail.superhuman.com/deeplink/refer";
const SUPERHUMAN_REFERRAL_HREF: &str = "https://mail.superhuman.com/deeplink/refer";

pub(crate) fn mail_body_display(message: &MailMessage) -> MailBodyDisplay {
    let document = mail_body_document(message);
    let document = fallback_empty_mail_document(document, mail_preview_label(message));
    let document = clean_body_document(document);
    let paragraphs = body_display_paragraphs(&document);
    MailBodyDisplay {
        clipped: document.clipped,
        document,
        paragraphs,
    }
}

fn mail_body_document(message: &MailMessage) -> MailDocument {
    match &message.display_body {
        MailDisplayBody::Html { html, .. } if !html.trim().is_empty() => {
            parse_mail_html(html).unwrap_or_default()
        }
        MailDisplayBody::PlainText { text, .. } if !text.trim().is_empty() => {
            plain_mail_document_from_text(text)
        }
        MailDisplayBody::PlainText { .. } | MailDisplayBody::Html { .. } => message
            .body_html
            .as_deref()
            .and_then(parse_mail_html)
            .unwrap_or_else(|| plain_mail_document_from_text(&message.body_text)),
    }
}

fn plain_mail_document_from_text(text: &str) -> MailDocument {
    let paragraphs = text
        .trim()
        .split("\n\n")
        .map(|paragraph| paragraph.trim().replace('\n', " "))
        .filter(|paragraph| !paragraph.is_empty())
        .map(plain_body_runs)
        .collect::<Vec<_>>();
    MailDocument::from_text_paragraphs(paragraphs)
}

fn plain_body_runs(paragraph: String) -> Vec<MailTextRun> {
    if paragraph.contains(SUPERHUMAN_REFERRAL_URL) {
        return vec![
            MailTextRun::text("Just "),
            MailTextRun::link("click here", SUPERHUMAN_REFERRAL_HREF),
            MailTextRun::text(" to give your friends email superpowers 😁"),
        ];
    }
    plain_text_runs_with_links(&remove_bracket_urls(&paragraph))
}

fn plain_text_runs_with_links(text: &str) -> Vec<MailTextRun> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let Some(url_start) = mail_next_bare_url_start(text, index) else {
            push_plain_text_run(&mut runs, &text[index..]);
            break;
        };
        push_plain_text_run(&mut runs, &text[index..url_start]);
        let url_end = mail_bare_url_end(text, url_start);
        if url_start == url_end {
            push_plain_text_run(&mut runs, &text[url_start..url_start + 1]);
            index = url_start + 1;
            continue;
        }
        let url = &text[url_start..url_end];
        runs.push(MailTextRun::link(url.to_string(), url.to_string()));
        index = url_end;
    }
    if runs.is_empty() {
        runs.push(MailTextRun::text(String::new()));
    }
    runs
}

fn push_plain_text_run(runs: &mut Vec<MailTextRun>, text: &str) {
    if !text.is_empty() {
        runs.push(MailTextRun::text(text));
    }
}

fn mail_next_bare_url_start(text: &str, start: usize) -> Option<usize> {
    let mut search_start = start;
    while search_start < text.len() {
        let rest = &text[search_start..];
        let relative_start = ["http://", "https://", "mailto:", "tel:"]
            .iter()
            .filter_map(|prefix| rest.find(prefix))
            .min()?;
        let candidate = search_start + relative_start;
        if mail_bare_url_boundary(text, candidate) {
            return Some(candidate);
        }
        search_start = candidate + 1;
    }
    None
}

fn mail_bare_url_boundary(text: &str, start: usize) -> bool {
    text[..start]
        .chars()
        .next_back()
        .is_none_or(|character| character.is_whitespace() || "([{\"'".contains(character))
}

fn mail_bare_url_end(text: &str, start: usize) -> usize {
    let raw_end = text[start..]
        .find(|character: char| character.is_whitespace() || character == '<' || character == '>')
        .map(|offset| start + offset)
        .unwrap_or(text.len());
    let mut end = raw_end;
    while end > start {
        let character = text[..end]
            .chars()
            .next_back()
            .expect("url end should stay on a character boundary");
        if ".,;:!?)]}".contains(character) {
            end -= character.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn fallback_empty_mail_document(document: MailDocument, preview: String) -> MailDocument {
    if !document.blocks.is_empty() {
        return document;
    }
    MailDocument::from_text_paragraphs(vec![vec![MailTextRun::text(preview)]])
}

fn clean_body_document(mut document: MailDocument) -> MailDocument {
    let mut clipped = document.clipped;
    document.blocks = clean_body_blocks(document.blocks, &mut clipped);
    document.blocks = flatten_layout_only_containers(document.blocks);
    document.clipped = clipped;
    document.refresh_metadata();
    document
}

fn clean_body_blocks(blocks: Vec<MailBlock>, clipped: &mut bool) -> Vec<MailBlock> {
    let mut cleaned = Vec::new();
    for block in blocks {
        if *clipped {
            break;
        }
        if let Some(block) = clean_body_block(block, clipped) {
            cleaned.push(block);
        }
    }
    cleaned
}

fn clean_body_block(block: MailBlock, clipped: &mut bool) -> Option<MailBlock> {
    match block {
        MailBlock::Paragraph(paragraph) => clean_body_paragraph(paragraph, clipped),
        MailBlock::Container(mut container) => {
            container.children = clean_body_blocks(container.children, clipped);
            (!container.children.is_empty() || container.style.has_box_style())
                .then_some(MailBlock::Container(container))
        }
        MailBlock::Table(mut table) => {
            for row in &mut table.rows {
                for cell in &mut row.cells {
                    let children = std::mem::take(&mut cell.children);
                    cell.children = clean_body_blocks(children, clipped);
                }
            }
            table.rows.retain(|row| {
                row.cells
                    .iter()
                    .any(|cell| !cell.children.is_empty() || cell.style.has_box_style())
            });
            (!table.rows.is_empty()).then_some(MailBlock::Table(table))
        }
        MailBlock::Image(_) | MailBlock::Rule(_) | MailBlock::Spacer(_) => Some(block),
    }
}

fn clean_body_paragraph(paragraph: MailParagraph, clipped: &mut bool) -> Option<MailBlock> {
    let text = mail_runs_plain_text(&paragraph.runs);
    if clips_visible_mail_body(&text) {
        *clipped = true;
        return None;
    }
    if text.contains("iterable-links.superhuman.com") {
        *clipped = true;
        return None;
    }
    Some(MailBlock::Paragraph(paragraph))
}

fn flatten_layout_only_containers(blocks: Vec<MailBlock>) -> Vec<MailBlock> {
    let mut flattened = Vec::new();
    for block in blocks {
        match block {
            MailBlock::Container(container)
                if container.link_target.is_none()
                    && mail_style_is_layout_only_container(&container.style) =>
            {
                flattened.extend(flatten_layout_only_containers(container.children));
            }
            MailBlock::Container(mut container) => {
                container.children = flatten_layout_only_containers(container.children);
                flattened.push(MailBlock::Container(container));
            }
            MailBlock::Table(mut table) => {
                for row in &mut table.rows {
                    for cell in &mut row.cells {
                        cell.children =
                            flatten_layout_only_containers(std::mem::take(&mut cell.children));
                    }
                }
                flattened.push(MailBlock::Table(table));
            }
            other => flattened.push(other),
        }
    }
    flattened
}

fn mail_style_is_layout_only_container(style: &crate::ui::MailStyle) -> bool {
    !style.has_box_style()
}

fn clips_visible_mail_body(text: &str) -> bool {
    let text = text.trim();
    text.starts_with("Take me to Superhuman Mail")
        || text.starts_with("If you don't want to hear from me again")
}

fn remove_bracket_urls(text: &str) -> String {
    let mut cleaned = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("[http") {
        cleaned.push_str(rest[..start].trim_end());
        let Some(end) = rest[start..].find(']') else {
            rest = &rest[start..];
            break;
        };
        rest = &rest[start + end + 1..];
    }
    cleaned.push_str(rest);
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn body_display_paragraphs(document: &MailDocument) -> Vec<MailBodyParagraph> {
    document
        .text_paragraphs()
        .into_iter()
        .filter_map(body_runs_paragraph)
        .collect()
}

fn body_runs_paragraph(runs: Vec<MailTextRun>) -> Option<MailBodyParagraph> {
    let mut segments = runs
        .into_iter()
        .filter(|run| !run.text.is_empty())
        .map(|run| {
            let kind = if run.href.is_some() {
                MailBodySegmentKind::Link
            } else {
                MailBodySegmentKind::Text
            };
            body_segment(run.text, kind)
        })
        .collect::<Vec<_>>();
    trim_body_segments(&mut segments);
    (!segments.is_empty()).then_some(MailBodyParagraph { segments })
}

fn trim_body_segments(segments: &mut Vec<MailBodySegment>) {
    if let Some(first) = segments.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    if let Some(last) = segments.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    segments.retain(|segment| !segment.text.is_empty());
}

fn mail_runs_plain_text(runs: &[MailTextRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect::<String>()
}

fn body_segment(text: impl Into<String>, kind: MailBodySegmentKind) -> MailBodySegment {
    MailBodySegment {
        text: text.into(),
        kind,
    }
}

#[cfg(test)]
pub(crate) fn body_paragraph_plain_text(paragraph: &MailBodyParagraph) -> String {
    paragraph
        .segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<String>()
}

#[cfg(test)]
mod tests;
