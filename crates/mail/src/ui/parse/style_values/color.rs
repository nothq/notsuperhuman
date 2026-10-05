use super::clean_css_value;

pub(in crate::ui::parse) fn parse_css_color(value: &str) -> Option<u32> {
    let value = clean_css_value(value);
    if value.eq_ignore_ascii_case("transparent") {
        return None;
    }
    parse_css_color_token(&value)
        .or_else(|| parse_css_function_color(&value))
        .or_else(|| {
            value
                .split_whitespace()
                .filter(|part| !part.eq_ignore_ascii_case("none"))
                .find_map(parse_css_color_token)
        })
}

fn parse_css_color_token(value: &str) -> Option<u32> {
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    if value.starts_with("rgb(") || value.starts_with("rgba(") {
        return parse_rgb_color(value);
    }
    named_color(value)
}

fn parse_css_function_color(value: &str) -> Option<u32> {
    ["rgb(", "rgba("].into_iter().find_map(|prefix| {
        let start = value.find(prefix)?;
        let rest = &value[start..];
        let end = rest.find(')')?;
        parse_rgb_color(&rest[..=end])
    })
}

fn parse_hex_color(hex: &str) -> Option<u32> {
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            Some(((r as u32 * 17) << 16) | ((g as u32 * 17) << 8) | (b as u32 * 17))
        }
        6 => u32::from_str_radix(hex, 16).ok(),
        _ => None,
    }
}

fn parse_rgb_color(value: &str) -> Option<u32> {
    let body = value
        .strip_prefix("rgb(")
        .or_else(|| value.strip_prefix("rgba("))?
        .strip_suffix(')')?;
    let channels = if body.contains(',') {
        body.split(',').collect::<Vec<_>>()
    } else {
        body.split_whitespace().collect::<Vec<_>>()
    }
    .into_iter()
    .take(3)
    .map(|part| part.trim().trim_end_matches('/').parse::<u8>().ok())
    .collect::<Option<Vec<_>>>()?;
    let [r, g, b, ..] = channels.as_slice() else {
        return None;
    };
    Some(((*r as u32) << 16) | ((*g as u32) << 8) | *b as u32)
}

fn named_color(value: &str) -> Option<u32> {
    match value.to_ascii_lowercase().as_str() {
        "black" => Some(0x000000),
        "blue" => Some(0x0000ff),
        "gray" | "grey" => Some(0x808080),
        "green" => Some(0x008000),
        "red" => Some(0xff0000),
        "white" => Some(0xffffff),
        "yellow" => Some(0xffff00),
        _ => None,
    }
}
