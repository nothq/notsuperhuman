use std::time::Duration;

use remote_image_model::{RemoteImageApi, RemoteImageData};
use reqwest::{header::HeaderMap, Url};

mod fetch;
mod svg;

#[cfg(test)]
use fetch::validate_resolved_addrs;
use fetch::{
    load_remote_image_data, parse_absolute_remote_image_url,
    same_origin, validate_remote_image_url,
};
pub use svg::validate_safe_svg;

const IMAGE_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const IMAGE_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const IMAGE_MAX_REDIRECTS: usize = 3;
const IMAGE_MAX_BYTES: usize = 2 * 1024 * 1024;
const IMAGE_MAX_SIDE: u32 = 8192;
const IMAGE_MAX_PIXELS: u64 = 40_000_000;

#[derive(Clone, Copy, Debug, Default)]
pub struct PublicRemoteImageApi;

pub fn load_public_remote_image_data(url: &str) -> Result<RemoteImageData, String> {
    load_remote_image_data(
        parse_absolute_remote_image_url(url)?,
        FetchScope::Public,
        &HeaderMap::new(),
    )
}

impl RemoteImageApi for PublicRemoteImageApi {
    fn load_remote_image(&self, url: &str) -> Result<Option<RemoteImageData>, String> {
        load_public_remote_image_data(url).map(Some)
    }
}

#[derive(Clone)]
pub struct SameOriginRemoteImageApi {
    base_url: Url,
    headers: HeaderMap,
}

impl SameOriginRemoteImageApi {
    pub fn new(base_url: Url, headers: HeaderMap) -> Result<Self, String> {
        validate_remote_image_url(&base_url)?;
        Ok(Self { base_url, headers })
    }
}

impl RemoteImageApi for SameOriginRemoteImageApi {
    fn load_remote_image(&self, url: &str) -> Result<Option<RemoteImageData>, String> {
        let resolved_url = self
            .base_url
            .join(url)
            .map_err(|error| format!("invalid remote image URL {url}: {error}"))?;
        if same_origin(&self.base_url, &resolved_url) {
            return load_remote_image_data(
                resolved_url,
                FetchScope::SameOrigin(&self.base_url),
                &self.headers,
            )
            .map(Some);
        }
        load_public_remote_image_data(resolved_url.as_str()).map(Some)
    }
}

#[derive(Clone, Copy)]
enum FetchScope<'a> {
    Public,
    SameOrigin(&'a Url),
}

#[cfg(test)]
mod tests;
