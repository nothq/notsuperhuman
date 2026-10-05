use super::{
    MailCssAttributeMatch, MailCssAttributeSelector, MailCssCompoundSelector, MailCssDeclaration,
    MailCssRule, MailCssSelector, MAIL_CSS_VIEWPORT_WIDTH,
};

pub(super) fn without_css_comments(source: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut characters = source.chars().peekable();
    let mut quote = None;
    let mut escaped = false;
    while let Some(character) = characters.next() {
        if let Some(active_quote) = quote {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
            output.push(character);
            continue;
        }
        if character == '/' && characters.peek() == Some(&'*') {
            characters.next();
            let mut previous = None;
            for comment_character in characters.by_ref() {
                if previous == Some('*') && comment_character == '/' {
                    break;
                }
                previous = Some(comment_character);
            }
            output.push(' ');
            continue;
        }
        output.push(character);
    }
    output
}

pub(super) fn collect_css_rules(source: &str, rules: &mut Vec<MailCssRule>) {
    let mut offset = 0;
    while let Some(open_relative) = source[offset..].find('{') {
        let open = offset + open_relative;
        let selector = source[offset..open].trim();
        let Some(close) = matching_css_brace(source, open) else {
            break;
        };
        let body = &source[open + 1..close];
        if selector.starts_with("@media") {
            if media_query_applies(selector) {
                collect_css_rules(body, rules);
            }
        } else if !selector.starts_with('@') {
            let declarations = parse_css_declarations(body);
            if !declarations.is_empty() {
                for selector in selector.split(',') {
                    if let Some(selector) = parse_css_selector(selector) {
                        rules.push(MailCssRule {
                            selector,
                            declarations: declarations.clone(),
                        });
                    }
                }
            }
        }
        offset = close + 1;
    }
}

fn matching_css_brace(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, byte) in source.as_bytes().iter().enumerate().skip(open) {
        match byte {
            b'{' => depth += 1,
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn media_query_applies(selector: &str) -> bool {
    let selector = selector.to_ascii_lowercase();
    let max_widths = css_media_px_values(&selector, "max-width")
        .into_iter()
        .chain(css_media_px_values(&selector, "max-device-width"))
        .collect::<Vec<_>>();
    if !max_widths.is_empty()
        && max_widths
            .iter()
            .all(|max_width| *max_width < MAIL_CSS_VIEWPORT_WIDTH)
    {
        return false;
    }
    css_media_px_values(&selector, "min-width")
        .into_iter()
        .all(|min_width| min_width <= MAIL_CSS_VIEWPORT_WIDTH)
}

fn css_media_px_values(selector: &str, property: &str) -> Vec<f32> {
    let mut values = Vec::new();
    let mut rest = selector;
    while let Some(property_index) = rest.find(property) {
        rest = &rest[property_index + property.len()..];
        let Some(px_index) = rest.find("px") else {
            break;
        };
        let number = rest[..px_index]
            .chars()
            .rev()
            .take_while(|char| char.is_ascii_digit() || *char == '.')
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>();
        if let Ok(value) = number.parse::<f32>() {
            values.push(value);
        }
        rest = &rest[px_index + 2..];
    }
    values
}

pub(super) fn parse_css_declarations(body: &str) -> Vec<MailCssDeclaration> {
    split_css_declarations(body)
        .into_iter()
        .filter_map(|declaration| {
            let (property, value) = declaration.split_once(':')?;
            Some(MailCssDeclaration {
                property: property.trim().to_ascii_lowercase(),
                value: value.trim().to_string(),
                important: css_value_is_important(value),
            })
        })
        .collect()
}

/// Split a declaration block on the semicolons that actually separate
/// declarations.
///
/// A `;` inside `url(...)` or a quoted string is part of the value:
/// `background-image: url(data:image/jpeg;base64,...)` carries one, and cutting
/// the block there left every inline data-URI background as `url(data:image/jpeg`
/// — no closing parenthesis, no image.
fn split_css_declarations(body: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (index, character) in body.char_indices() {
        match (quote, character) {
            (Some(open), c) if c == open => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(character),
            (None, '(') => depth += 1,
            (None, ')') => depth = depth.saturating_sub(1),
            (None, ';') if depth == 0 => {
                parts.push(&body[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(&body[start..]);
    parts
}

fn css_value_is_important(value: &str) -> bool {
    value.trim().to_ascii_lowercase().ends_with("!important")
}

fn parse_css_selector(selector: &str) -> Option<MailCssSelector> {
    let selector = selector.trim();
    if selector.is_empty() || selector.contains('*') {
        return None;
    }
    if selector.contains('>') {
        let chain = selector
            .split('>')
            .map(str::trim)
            .map(parse_css_compound)
            .collect::<Option<Vec<_>>>()?;
        let (target, ancestors) = chain.split_last()?;
        return (!ancestors.is_empty()).then(|| MailCssSelector::Child {
            ancestors: ancestors.to_vec(),
            target: target.clone(),
        });
    }
    if selector.contains('+') {
        let mut parts = selector.split('+').map(str::trim);
        let previous = parse_css_compound(parts.next()?)?;
        let target = parse_css_compound(parts.next()?)?;
        if parts.next().is_some() {
            return None;
        }
        return Some(MailCssSelector::Adjacent { previous, target });
    }
    if selector.contains(':') || selector.contains('~') {
        return None;
    }
    let selector_parts = selector.split_whitespace().collect::<Vec<_>>();
    if selector_parts.len() > 1 {
        let compound_chain = selector_parts
            .into_iter()
            .map(parse_css_compound)
            .collect::<Option<Vec<_>>>()?;
        let (target, ancestors) = compound_chain.split_last()?;
        return Some(MailCssSelector::Descendant {
            ancestors: ancestors.to_vec(),
            target: target.clone(),
        });
    }
    parse_css_compound(selector).map(MailCssSelector::Compound)
}

fn parse_css_tag_selector(selector: &str) -> Option<String> {
    (!selector.is_empty()
        && selector
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || char == '-'))
    .then(|| selector.to_ascii_lowercase())
}

fn parse_css_compound(selector: &str) -> Option<MailCssCompoundSelector> {
    if selector.is_empty() || selector.contains(':') {
        return None;
    }
    let (selector, attributes) = split_attribute_selectors(selector)?;
    let selector = selector.as_str();
    let first_qualifier = selector.find(['.', '#']).unwrap_or(selector.len());
    let tag = match &selector[..first_qualifier] {
        "" => None,
        tag => Some(parse_css_tag_selector(tag)?),
    };
    let mut id = None;
    let mut classes = Vec::new();
    let mut remainder = &selector[first_qualifier..];
    while !remainder.is_empty() {
        let qualifier = remainder.as_bytes()[0];
        let next = remainder[1..]
            .find(['.', '#'])
            .map(|offset| offset + 1)
            .unwrap_or(remainder.len());
        let value = &remainder[1..next];
        if !css_identifier(value) {
            return None;
        }
        match qualifier {
            b'.' => classes.push(value.to_ascii_lowercase()),
            b'#' if id.is_none() => id = Some(value.to_string()),
            _ => return None,
        }
        remainder = &remainder[next..];
    }
    (tag.is_some() || id.is_some() || !classes.is_empty() || !attributes.is_empty()).then_some(
        MailCssCompoundSelector {
            tag,
            id,
            classes,
            attributes,
        },
    )
}

type SplitAttributeSelectors = (String, Vec<MailCssAttributeSelector>);

/// The compound without its `[...]` parts, and those parts parsed. A form it
/// does not know (`^=`, `$=`, `*=`, `|=`) drops the whole selector rather
/// than matching more than the author meant.
fn split_attribute_selectors(selector: &str) -> Option<SplitAttributeSelectors> {
    let mut rest = String::new();
    let mut attributes = Vec::new();
    let mut remainder = selector;
    while let Some(open) = remainder.find('[') {
        rest.push_str(&remainder[..open]);
        let close = open + remainder[open..].find(']')?;
        let body = remainder[open + 1..close].trim();
        let (name, value) = match body.split_once('=') {
            None => (body, None),
            Some((name, value)) => {
                let value = value
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'')
                    .to_string();
                match name.strip_suffix('~') {
                    Some(name) => (name, Some((MailCssAttributeMatch::Word, value))),
                    None if name.ends_with(['^', '$', '*', '|']) => return None,
                    None => (name, Some((MailCssAttributeMatch::Exact, value))),
                }
            }
        };
        let name = name.trim();
        if !css_identifier(name) {
            return None;
        }
        attributes.push(MailCssAttributeSelector {
            name: name.to_ascii_lowercase(),
            value,
        });
        remainder = &remainder[close + 1..];
    }
    rest.push_str(remainder);
    Some((rest, attributes))
}

fn css_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_'))
}

#[cfg(test)]
mod tests {
    use super::without_css_comments;

    #[test]
    fn css_comment_removal_preserves_quoted_email_resource_urls() {
        let css = r#"/* reset */ .hero { background-image:url('https://example.test/a/*literal*/b.png'); }"#;

        assert_eq!(
            without_css_comments(css),
            r#"  .hero { background-image:url('https://example.test/a/*literal*/b.png'); }"#
        );
    }
}
