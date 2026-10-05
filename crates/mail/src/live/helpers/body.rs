use crate::model::{MailBodySource, MailDisplayBody};

use crate::live::types::{JmapEmail, JmapEmailBodyPart};

pub(super) fn extract_mail_body_text(
    message: &JmapEmail,
    display_body: &MailDisplayBody,
) -> String {
    if let MailDisplayBody::PlainText { text, .. } = display_body {
        return text.clone();
    }
    for part in &message.text_body {
        if let Some(value) = message.body_values.get(part.part_id.as_str()) {
            if let Some(body) = normalized_text_part_body(part, value.value.as_str()) {
                if !body.is_empty() {
                    return body;
                }
            }
        }
    }
    fallback_mail_body_text(message)
}

pub(super) fn extract_mail_display_body(message: &JmapEmail) -> MailDisplayBody {
    let mut repaired_plain_text = None;
    for part in &message.html_body {
        if let Some(value) = message.body_values.get(part.part_id.as_str()) {
            let body = normalize_mail_body_html(value.value.as_str());
            if body.is_empty() {
                continue;
            }
            if body_part_declares_html(part, true) {
                if html_source_has_authored_markup(&body) {
                    return MailDisplayBody::Html {
                        html: body,
                        source: MailBodySource::JmapHtmlBody,
                    };
                }
                repaired_plain_text.get_or_insert_with(|| MailDisplayBody::PlainText {
                    text: normalize_html_text_body(&body),
                    source: MailBodySource::CompatibilityRepair,
                });
            } else {
                repaired_plain_text.get_or_insert_with(|| MailDisplayBody::PlainText {
                    text: normalize_mail_body_text(&body),
                    source: MailBodySource::CompatibilityRepair,
                });
            }
        }
    }
    for part in &message.text_body {
        if let Some(value) = message.body_values.get(part.part_id.as_str()) {
            let body = normalize_mail_body_html(value.value.as_str());
            if body.is_empty() {
                continue;
            }
            if body_part_declares_html(part, false) {
                if html_source_has_authored_markup(&body) {
                    return MailDisplayBody::Html {
                        html: body,
                        source: MailBodySource::CompatibilityRepair,
                    };
                }
                return MailDisplayBody::PlainText {
                    text: normalize_html_text_body(&body),
                    source: MailBodySource::CompatibilityRepair,
                };
            }
            return MailDisplayBody::PlainText {
                text: normalize_mail_body_text(&body),
                source: MailBodySource::JmapTextBody,
            };
        }
    }
    if let Some(display_body) = repaired_plain_text {
        return display_body;
    }
    fallback_mail_display_body(message)
}

fn fallback_mail_body_text(message: &JmapEmail) -> String {
    let preview = super::cleaned_preview(message.preview.as_deref());
    if preview.is_empty() {
        "No message body available.".to_string()
    } else {
        preview
    }
}

fn fallback_mail_display_body(message: &JmapEmail) -> MailDisplayBody {
    if message.html_body.is_empty() && message.text_body.is_empty() {
        for value in message.body_values.values() {
            let body = normalize_mail_body_html(value.value.as_str());
            if body.is_empty() {
                continue;
            }
            if html_source_has_authored_markup(&body) {
                return MailDisplayBody::Html {
                    html: body,
                    source: MailBodySource::CompatibilityRepair,
                };
            }
            return MailDisplayBody::PlainText {
                text: normalize_html_text_body(&body),
                source: MailBodySource::CompatibilityRepair,
            };
        }
    }
    let preview = super::cleaned_preview(message.preview.as_deref());
    if preview.is_empty() {
        MailDisplayBody::default()
    } else {
        MailDisplayBody::PlainText {
            text: preview,
            source: MailBodySource::Preview,
        }
    }
}

fn normalized_text_part_body(part: &JmapEmailBodyPart, value: &str) -> Option<String> {
    let body = normalize_mail_body_html(value);
    if body_part_declares_html(part, false) && !html_source_has_authored_markup(&body) {
        return Some(normalize_html_text_body(&body));
    }
    if body_part_declares_html(part, false) {
        return None;
    }
    Some(normalize_mail_body_text(value))
}

fn body_part_declares_html(part: &JmapEmailBodyPart, listed_as_html_body: bool) -> bool {
    let _charset = part.charset.as_deref();
    part.content_type
        .as_deref()
        .map(content_type_is_html)
        .unwrap_or(listed_as_html_body)
}

fn content_type_is_html(content_type: &str) -> bool {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    matches!(media_type.as_str(), "text/html" | "application/xhtml+xml")
}

fn normalize_mail_body_text(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_string()
}

fn normalize_mail_body_html(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\0', "")
        .trim()
        .to_string()
}

fn normalize_html_text_body(value: &str) -> String {
    normalize_mail_body_text(&decode_html_text_entities(value))
}

fn html_source_has_authored_markup(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;
    while let Some(relative_start) = source[index..].find('<') {
        let start = index + relative_start;
        let Some(relative_end) = source[start..].find('>') else {
            return false;
        };
        let end = start + relative_end;
        let token = source[start + 1..end].trim_start();
        if html_token_is_authored_markup(token) {
            return true;
        }
        index = end + usize::from(end < bytes.len());
    }
    false
}

fn html_token_is_authored_markup(token: &str) -> bool {
    if token.starts_with("!--") {
        return true;
    }
    if token
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("!doctype"))
    {
        return true;
    }
    let token = token.strip_prefix('/').unwrap_or(token).trim_start();
    let tag_name = token
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    if tag_name.is_empty() {
        return false;
    }
    let remainder = &token[tag_name.len()..];
    if !remainder.is_empty()
        && !remainder.starts_with(|character: char| {
            character.is_ascii_whitespace() || character == '/' || character == '>'
        })
    {
        return false;
    }
    html_tag_has_mail_body_semantics(tag_name.as_str())
}

fn html_tag_has_mail_body_semantics(tag_name: &str) -> bool {
    HTML_MAIL_BODY_TAGS.contains(&tag_name.to_ascii_lowercase().as_str())
}

const HTML_MAIL_BODY_TAGS: &[&str] = &[
    "a",
    "abbr",
    "acronym",
    "address",
    "article",
    "aside",
    "b",
    "blockquote",
    "body",
    "br",
    "button",
    "caption",
    "center",
    "cite",
    "code",
    "col",
    "colgroup",
    "dd",
    "del",
    "details",
    "div",
    "dl",
    "dt",
    "em",
    "figcaption",
    "figure",
    "font",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "i",
    "img",
    "li",
    "main",
    "ol",
    "p",
    "pre",
    "s",
    "section",
    "small",
    "span",
    "strike",
    "strong",
    "style",
    "sub",
    "summary",
    "sup",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "u",
    "ul",
];

fn decode_html_text_entities(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(start) = rest.find('&') {
        decoded.push_str(&rest[..start]);
        let entity_start = start + 1;
        let Some(relative_end) = rest[entity_start..].find(';') else {
            decoded.push_str(&rest[start..]);
            return decoded;
        };
        let entity_end = entity_start + relative_end;
        let entity = &rest[entity_start..entity_end];
        if let Some(replacement) = decode_html_text_entity(entity) {
            decoded.push_str(replacement.as_str());
        } else {
            decoded.push_str(&rest[start..=entity_end]);
        }
        rest = &rest[entity_end + 1..];
    }
    decoded.push_str(rest);
    decoded
}

fn decode_html_text_entity(entity: &str) -> Option<String> {
    match entity {
        "amp" => Some("&".to_string()),
        "lt" => Some("<".to_string()),
        "gt" => Some(">".to_string()),
        "quot" => Some("\"".to_string()),
        "apos" | "#39" => Some("'".to_string()),
        "nbsp" => Some(" ".to_string()),
        _ => decode_numeric_html_entity(entity).map(|character| character.to_string()),
    }
}

fn decode_numeric_html_entity(entity: &str) -> Option<char> {
    let number = entity.strip_prefix('#')?;
    let codepoint = if let Some(hex) = number
        .strip_prefix('x')
        .or_else(|| number.strip_prefix('X'))
    {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        number.parse::<u32>().ok()?
    };
    char::from_u32(codepoint)
}
