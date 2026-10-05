use std::{
    collections::HashSet,
    io::{Cursor, Read},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
};

use image::{ImageFormat, ImageReader, Limits};
use remote_image_model::RemoteImageData;
use reqwest::{
    blocking::{Client, Response},
    header::{HeaderMap, ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, LOCATION},
    redirect::Policy as RedirectPolicy,
    StatusCode, Url,
};

use super::{
    validate_safe_svg, FetchScope, IMAGE_CONNECT_TIMEOUT, IMAGE_MAX_BYTES, IMAGE_MAX_PIXELS,
    IMAGE_MAX_REDIRECTS, IMAGE_MAX_SIDE, IMAGE_REQUEST_TIMEOUT,
};

enum RemoteImageFormat {
    Raster(ImageFormat),
    Svg,
}

struct RemoteImageTarget {
    url: Url,
    host: String,
    addrs: Vec<SocketAddr>,
}

pub(super) fn load_remote_image_data(
    mut current_url: Url,
    scope: FetchScope<'_>,
    headers: &HeaderMap,
) -> Result<RemoteImageData, String> {
    load_remote_image_data_with_formats(&mut current_url, scope, headers, false)
}

fn load_remote_image_data_with_formats(
    current_url: &mut Url,
    scope: FetchScope<'_>,
    headers: &HeaderMap,
    allow_svg: bool,
) -> Result<RemoteImageData, String> {
    let accept = if allow_svg {
        "image/png,image/jpeg,image/gif,image/webp,image/svg+xml"
    } else {
        "image/png,image/jpeg,image/gif,image/webp"
    };
    for redirect_count in 0..=IMAGE_MAX_REDIRECTS {
        let target = resolve_remote_image_target(current_url.clone(), scope)?;
        let client = remote_image_client(&target)?;
        let response = client
            .get(target.url.clone())
            .headers(headers.clone())
            .header(ACCEPT, accept)
            .send()
            .map_err(|error| format!("failed to fetch remote image {}: {error}", target.url))?;
        let status = response.status();
        if status.is_redirection() {
            if redirect_count == IMAGE_MAX_REDIRECTS {
                return Err(format!(
                    "failed to fetch remote image {}: too many redirects",
                    target.url
                ));
            }
            *current_url = redirect_target_url(&target.url, &response, scope)?;
            continue;
        }
        return read_remote_image_response(target.url, response, allow_svg);
    }
    unreachable!("redirect loop is bounded by IMAGE_MAX_REDIRECTS")
}

pub(super) fn parse_absolute_remote_image_url(url: &str) -> Result<Url, String> {
    let parsed =
        Url::parse(url).map_err(|error| format!("invalid remote image URL {url}: {error}"))?;
    validate_remote_image_url(&parsed)?;
    Ok(parsed)
}

pub(super) fn validate_remote_image_url(url: &Url) -> Result<(), String> {
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!(
            "blocked remote image {url}: unsupported URL scheme"
        ));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(format!(
            "blocked remote image {url}: URL credentials are not allowed"
        ));
    }
    url.host_str()
        .filter(|host| !host.trim().is_empty())
        .ok_or_else(|| format!("blocked remote image {url}: missing host"))?;
    url.port_or_known_default()
        .ok_or_else(|| format!("blocked remote image {url}: missing port"))?;
    Ok(())
}

fn validate_scope(url: &Url, scope: FetchScope<'_>) -> Result<(), String> {
    validate_remote_image_url(url)?;
    if let FetchScope::SameOrigin(base_url) = scope {
        if !same_origin(base_url, url) {
            return Err(format!(
                "blocked authenticated remote image redirect from {base_url} to {url}"
            ));
        }
    }
    Ok(())
}

pub(super) fn same_origin(base_url: &Url, url: &Url) -> bool {
    base_url.scheme() == url.scheme()
        && base_url.host_str() == url.host_str()
        && base_url.port_or_known_default() == url.port_or_known_default()
}

fn resolve_remote_image_target(
    url: Url,
    scope: FetchScope<'_>,
) -> Result<RemoteImageTarget, String> {
    validate_scope(&url, scope)?;
    let host = url
        .host_str()
        .expect("validated remote image URL must have a host")
        .to_string();
    let port = url
        .port_or_known_default()
        .expect("validated remote image URL must have a port");
    let addrs = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|error| format!("failed to resolve remote image host {host}: {error}"))?
        .collect::<Vec<_>>();
    let addrs = validate_resolved_addrs(&host, addrs, scope)?;
    Ok(RemoteImageTarget { url, host, addrs })
}

pub(super) fn validate_resolved_addrs(
    host: &str,
    addrs: Vec<SocketAddr>,
    scope: FetchScope<'_>,
) -> Result<Vec<SocketAddr>, String> {
    if addrs.is_empty() {
        return Err(format!(
            "blocked remote image host {host}: DNS returned no addresses"
        ));
    }
    let mut seen = HashSet::new();
    let mut unique_addrs = Vec::new();
    for addr in addrs {
        if matches!(scope, FetchScope::Public) && is_forbidden_remote_image_ip(addr.ip()) {
            return Err(format!(
                "blocked remote image host {host}: resolved to non-public IP {}",
                addr.ip()
            ));
        }
        if seen.insert(addr) {
            unique_addrs.push(addr);
        }
    }
    Ok(unique_addrs)
}

fn remote_image_client(target: &RemoteImageTarget) -> Result<Client, String> {
    Client::builder()
        .redirect(RedirectPolicy::none())
        .connect_timeout(IMAGE_CONNECT_TIMEOUT)
        .timeout(IMAGE_REQUEST_TIMEOUT)
        .resolve_to_addrs(&target.host, &target.addrs)
        .build()
        .map_err(|error| format!("failed to build remote image HTTP client: {error}"))
}

fn redirect_target_url(
    base_url: &Url,
    response: &Response,
    scope: FetchScope<'_>,
) -> Result<Url, String> {
    let location = response
        .headers()
        .get(LOCATION)
        .ok_or_else(|| format!("remote image redirect from {base_url} did not include Location"))?
        .to_str()
        .map_err(|error| {
            format!("remote image redirect from {base_url} has invalid Location: {error}")
        })?;
    let target = base_url.join(location).map_err(|error| {
        format!("remote image redirect from {base_url} has invalid Location: {error}")
    })?;
    validate_scope(&target, scope)?;
    Ok(target)
}

fn read_remote_image_response(
    url: Url,
    mut response: Response,
    allow_svg: bool,
) -> Result<RemoteImageData, String> {
    let status = response.status();
    if !status.is_success() {
        return Err(format!(
            "failed to fetch remote image {url}: unexpected HTTP {status}"
        ));
    }
    reject_unsupported_success_status(url.as_str(), status)?;
    reject_oversized_content_length(url.as_str(), &response)?;
    let (mimetype, format) = response_image_mimetype(url.as_str(), &response, allow_svg)?;
    let bytes = read_capped_response_body(url.as_str(), &mut response)?;
    match format {
        RemoteImageFormat::Raster(format) => {
            validate_image_dimensions(url.as_str(), &bytes, format)?
        }
        RemoteImageFormat::Svg => validate_safe_svg(&bytes)
            .map_err(|error| format!("failed to fetch remote image {url}: {error}"))?,
    }
    Ok(RemoteImageData { bytes, mimetype })
}

fn reject_unsupported_success_status(url: &str, status: StatusCode) -> Result<(), String> {
    if status == StatusCode::NO_CONTENT || status == StatusCode::RESET_CONTENT {
        return Err(format!(
            "failed to fetch remote image {url}: unexpected HTTP {status}"
        ));
    }
    Ok(())
}

fn reject_oversized_content_length(url: &str, response: &Response) -> Result<(), String> {
    let Some(length) = response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|header| header.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok())
    else {
        return Ok(());
    };
    if length > IMAGE_MAX_BYTES as u64 {
        return Err(format!(
            "failed to fetch remote image {url}: Content-Length {length} exceeds {IMAGE_MAX_BYTES} bytes"
        ));
    }
    Ok(())
}

fn response_image_mimetype(
    url: &str,
    response: &Response,
    allow_svg: bool,
) -> Result<(String, RemoteImageFormat), String> {
    let value = response
        .headers()
        .get(CONTENT_TYPE)
        .ok_or_else(|| format!("failed to fetch remote image {url}: missing Content-Type"))?
        .to_str()
        .map_err(|error| {
            format!("failed to fetch remote image {url}: invalid Content-Type: {error}")
        })?;
    let mimetype = value
        .split(';')
        .next()
        .unwrap_or(value)
        .trim()
        .to_ascii_lowercase();
    let format = match mimetype.as_str() {
        "image/png" => RemoteImageFormat::Raster(ImageFormat::Png),
        "image/jpeg" => RemoteImageFormat::Raster(ImageFormat::Jpeg),
        "image/gif" => RemoteImageFormat::Raster(ImageFormat::Gif),
        "image/webp" => RemoteImageFormat::Raster(ImageFormat::WebP),
        "image/svg+xml" if allow_svg => RemoteImageFormat::Svg,
        _ => {
            return Err(format!(
                "failed to fetch remote image {url}: unsupported Content-Type {mimetype}"
            ));
        }
    };
    Ok((mimetype, format))
}

fn read_capped_response_body(url: &str, response: &mut Response) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("failed to read remote image {url}: {error}"))?;
        if read == 0 {
            return Ok(bytes);
        }
        if bytes.len() + read > IMAGE_MAX_BYTES {
            return Err(format!(
                "failed to read remote image {url}: response exceeds {IMAGE_MAX_BYTES} bytes"
            ));
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
}

fn validate_image_dimensions(url: &str, bytes: &[u8], format: ImageFormat) -> Result<(), String> {
    let mut limits = Limits::default();
    limits.max_image_width = Some(IMAGE_MAX_SIDE);
    limits.max_image_height = Some(IMAGE_MAX_SIDE);
    limits.max_alloc = Some(IMAGE_MAX_PIXELS.saturating_mul(4));
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    reader.limits(limits);
    let (width, height) = reader
        .into_dimensions()
        .map_err(|error| format!("failed to inspect remote image {url}: {error}"))?;
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > IMAGE_MAX_PIXELS {
        return Err(format!(
            "failed to inspect remote image {url}: dimensions {width}x{height} exceed {IMAGE_MAX_PIXELS} pixels"
        ));
    }
    Ok(())
}

fn is_forbidden_remote_image_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_forbidden_remote_image_ipv4(ip),
        IpAddr::V6(ip) => is_forbidden_remote_image_ipv6(ip),
    }
}

fn is_forbidden_remote_image_ipv4(ip: Ipv4Addr) -> bool {
    let [a, b, _, _] = ip.octets();
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_broadcast()
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 192 && b == 0)
        || (a == 198 && (18..=19).contains(&b))
        || a >= 224
}

fn is_forbidden_remote_image_ipv6(ip: Ipv6Addr) -> bool {
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        return is_forbidden_remote_image_ipv4(ipv4);
    }
    let segments = ip.segments();
    ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || (segments[0] & 0xfe00) == 0xfc00
        || (segments[0] & 0xffc0) == 0xfe80
        || (segments[0] == 0x2001 && segments[1] == 0x0db8)
}
