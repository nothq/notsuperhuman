use crate::model::{MailSearchSnippetSegment, MailSearchSnippetText};

pub(super) fn parse_snippet_text(source: &str) -> Result<MailSearchSnippetText, String> {
    const MARK_OPEN: &str = "<mark>";
    const MARK_CLOSE: &str = "</mark>";

    let mut remaining = source;
    let mut highlighted = false;
    let mut segments = Vec::new();
    while !remaining.is_empty() {
        if remaining.starts_with(MARK_OPEN) {
            if highlighted {
                return Err("SearchSnippet/get returned nested <mark> tags".to_string());
            }
            highlighted = true;
            remaining = &remaining[MARK_OPEN.len()..];
            continue;
        }
        if remaining.starts_with(MARK_CLOSE) {
            if !highlighted {
                return Err("SearchSnippet/get returned an unmatched </mark> tag".to_string());
            }
            highlighted = false;
            remaining = &remaining[MARK_CLOSE.len()..];
            continue;
        }
        let next_open = remaining.find(MARK_OPEN).unwrap_or(remaining.len());
        let next_close = remaining.find(MARK_CLOSE).unwrap_or(remaining.len());
        let next_marker = next_open.min(next_close);
        let text = decode_snippet_entities(&remaining[..next_marker]);
        if !text.is_empty() {
            segments.push(MailSearchSnippetSegment::new(text, highlighted));
        }
        remaining = &remaining[next_marker..];
    }
    if highlighted {
        return Err("SearchSnippet/get returned an unclosed <mark> tag".to_string());
    }
    Ok(MailSearchSnippetText::new(segments))
}

fn decode_snippet_entities(source: &str) -> String {
    let mut decoded = String::with_capacity(source.len());
    let mut remaining = source;
    while let Some(entity_start) = remaining.find('&') {
        decoded.push_str(&remaining[..entity_start]);
        remaining = &remaining[entity_start..];
        let Some(entity_end) = remaining.find(';') else {
            decoded.push_str(remaining);
            return decoded;
        };
        let entity = &remaining[1..entity_end];
        if let Some(character) = decode_snippet_entity(entity) {
            decoded.push(character);
        } else {
            decoded.push_str(&remaining[..=entity_end]);
        }
        remaining = &remaining[entity_end + 1..];
    }
    decoded.push_str(remaining);
    decoded
}

fn decode_snippet_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        _ => entity
            .strip_prefix("#x")
            .or_else(|| entity.strip_prefix("#X"))
            .and_then(|value| u32::from_str_radix(value, 16).ok())
            .or_else(|| {
                entity
                    .strip_prefix('#')
                    .and_then(|value| value.parse::<u32>().ok())
            })
            .and_then(char::from_u32),
    }
}
