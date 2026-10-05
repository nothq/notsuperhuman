const FORBIDDEN_ELEMENTS: &[&str] = &[
    "a",
    "animate",
    "animatemotion",
    "animatetransform",
    "audio",
    "canvas",
    "discard",
    "embed",
    "feimage",
    "foreignobject",
    "iframe",
    "image",
    "object",
    "script",
    "set",
    "video",
];

pub fn validate_safe_svg(bytes: &[u8]) -> Result<(), String> {
    let source = std::str::from_utf8(bytes).map_err(|_| "SVG is not UTF-8".to_string())?;
    let lowercase = source.to_ascii_lowercase();
    if lowercase.contains("<!doctype") || lowercase.contains("<!entity") {
        return Err("SVG document declarations are not allowed".to_string());
    }
    let document = roxmltree::Document::parse(source)
        .map_err(|error| format!("SVG XML is malformed: {error}"))?;
    let root = document.root_element();
    if !root.tag_name().name().eq_ignore_ascii_case("svg") {
        return Err("SVG root element is missing".to_string());
    }
    for node in document.descendants() {
        if node.is_pi() {
            return Err("SVG processing instructions are not allowed".to_string());
        }
        if !node.is_element() {
            continue;
        }
        let element_name = node.tag_name().name().to_ascii_lowercase();
        if FORBIDDEN_ELEMENTS.contains(&element_name.as_str()) {
            return Err(format!("SVG element <{element_name}> is not allowed"));
        }
        for attribute in node.attributes() {
            let name = attribute.name().to_ascii_lowercase();
            let value = attribute.value().trim();
            if name.starts_with("on") {
                return Err(format!("SVG event attribute {name} is not allowed"));
            }
            if name == "href" && !safe_local_fragment(value) {
                return Err("SVG external references are not allowed".to_string());
            }
            validate_svg_resource_value(value)?;
        }
        if element_name == "style" {
            validate_svg_resource_value(node.text().unwrap_or_default())?;
        }
    }
    Ok(())
}

fn validate_svg_resource_value(value: &str) -> Result<(), String> {
    let lowercase = value.to_ascii_lowercase();
    if lowercase.contains("javascript:")
        || lowercase.contains("@import")
        || lowercase.contains("@font-face")
    {
        return Err("SVG active or external content is not allowed".to_string());
    }
    let mut remaining = lowercase.as_str();
    while let Some(start) = remaining.find("url(") {
        let after_start = &remaining[start + 4..];
        let Some(end) = after_start.find(')') else {
            return Err("SVG contains an unterminated URL reference".to_string());
        };
        let target = after_start[..end]
            .trim()
            .trim_matches(|character| matches!(character, '\'' | '"'));
        if !safe_local_fragment(target) {
            return Err("SVG external URL references are not allowed".to_string());
        }
        remaining = &after_start[end + 1..];
    }
    Ok(())
}

fn safe_local_fragment(value: &str) -> bool {
    value.strip_prefix('#').is_some_and(|fragment| {
        !fragment.is_empty()
            && fragment.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | ':')
            })
    })
}
