use std::{
    io::{Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    sync::mpsc,
    thread,
};

use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use reqwest::header::HeaderMap;

use super::{
    load_public_remote_image_data, load_remote_image_data, validate_resolved_addrs, FetchScope,
    IMAGE_MAX_BYTES, IMAGE_MAX_REDIRECTS, IMAGE_MAX_SIDE,
};

#[test]
fn fetch_preserves_mime() {
    let body = png_bytes(1, 1);
    let response = http_response("200 OK", &[("Content-Type", "image/png")], &body);
    let server = TestImageServer::serve_once(response);

    let base_url = reqwest::Url::parse(&server.url).expect("test image URL");
    let image = load_remote_image_data(
        base_url.clone(),
        FetchScope::SameOrigin(&base_url),
        &HeaderMap::new(),
    )
    .expect("load remote image");
    server.finish();

    assert_eq!(image.mimetype, "image/png");
    assert_eq!(image.bytes, body);
}

#[test]
fn rejects_private_literal_urls() {
    let error = load_public_remote_image_data("http://127.0.0.1/image.png")
        .expect_err("loopback image URL should be blocked");

    assert!(error.contains("non-public IP 127.0.0.1"), "{error}");
}

#[test]
fn rejects_mixed_dns_answers() {
    let addrs = vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34)), 80),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), 80),
    ];

    let error = validate_resolved_addrs("example.test", addrs, FetchScope::Public)
        .expect_err("mixed public and private DNS answers should be blocked");

    assert!(error.contains("non-public IP 10.0.0.1"), "{error}");
}

#[test]
fn rejects_too_many_redirects() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect server");
    let url = format!("http://{}/image.png", listener.local_addr().expect("addr"));
    let redirect_response = http_response(
        "302 Found",
        &[("Location", "/image.png"), ("Content-Type", "text/plain")],
        b"",
    );
    let server = thread::spawn(move || {
        for _ in 0..=IMAGE_MAX_REDIRECTS {
            let (mut stream, _) = listener.accept().expect("accept redirect request");
            read_request(&mut stream);
            stream
                .write_all(&redirect_response)
                .expect("write redirect response");
        }
    });

    let base_url = reqwest::Url::parse(&url).expect("test image URL");
    let error = load_remote_image_data(
        base_url.clone(),
        FetchScope::SameOrigin(&base_url),
        &HeaderMap::new(),
    )
    .expect_err("redirect chain should be bounded");
    server.join().expect("redirect server thread");

    assert!(error.contains("too many redirects"), "{error}");
}

#[test]
fn rejects_non_image_content_type() {
    let response = http_response("200 OK", &[("Content-Type", "text/plain")], b"not image");
    let server = TestImageServer::serve_once(response);

    let base_url = reqwest::Url::parse(&server.url).expect("test image URL");
    let error = load_remote_image_data(
        base_url.clone(),
        FetchScope::SameOrigin(&base_url),
        &HeaderMap::new(),
    )
    .expect_err("text response should be blocked");
    server.finish();

    assert!(
        error.contains("unsupported Content-Type text/plain"),
        "{error}"
    );
}

#[test]
fn rejects_oversized_content_length() {
    let response = http_response(
        "200 OK",
        &[
            ("Content-Type", "image/png"),
            ("Content-Length", &(IMAGE_MAX_BYTES + 1).to_string()),
        ],
        b"",
    );
    let server = TestImageServer::serve_once(response);

    let base_url = reqwest::Url::parse(&server.url).expect("test image URL");
    let error = load_remote_image_data(
        base_url.clone(),
        FetchScope::SameOrigin(&base_url),
        &HeaderMap::new(),
    )
    .expect_err("oversized declared body should be blocked");
    server.finish();

    assert!(error.contains("Content-Length"), "{error}");
}

#[test]
fn rejects_streaming_body_over_cap() {
    let body = vec![0_u8; IMAGE_MAX_BYTES + 1];
    let mut response =
        b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: image/png\r\n\r\n".to_vec();
    response.extend_from_slice(&body);
    let server = TestImageServer::serve_once(response);

    let base_url = reqwest::Url::parse(&server.url).expect("test image URL");
    let error = load_remote_image_data(
        base_url.clone(),
        FetchScope::SameOrigin(&base_url),
        &HeaderMap::new(),
    )
    .expect_err("oversized streaming body should be blocked");
    server.finish();

    assert!(error.contains("response exceeds"), "{error}");
}

#[test]
fn rejects_oversized_dimensions() {
    let body = png_bytes(IMAGE_MAX_SIDE + 1, 1);
    let response = http_response("200 OK", &[("Content-Type", "image/png")], &body);
    let server = TestImageServer::serve_once(response);

    let base_url = reqwest::Url::parse(&server.url).expect("test image URL");
    let error = load_remote_image_data(
        base_url.clone(),
        FetchScope::SameOrigin(&base_url),
        &HeaderMap::new(),
    )
    .expect_err("oversized image dimensions should be blocked");
    server.finish();

    assert!(error.contains("Image size exceeds limit"), "{error}");
}

struct TestImageServer {
    url: String,
    request_rx: mpsc::Receiver<String>,
    server: thread::JoinHandle<()>,
}

impl TestImageServer {
    fn serve_once(response: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind image server");
        let url = format!(
            "http://{}/image.png",
            listener.local_addr().expect("image server addr")
        );
        let (request_tx, request_rx) = std::sync::mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept image request");
            let request = read_request(&mut stream);
            request_tx
                .send(request)
                .expect("send captured image request");
            let _ = stream.write_all(&response);
        });
        Self {
            url,
            request_rx,
            server,
        }
    }

    fn finish(self) -> String {
        let request = self.request_rx.recv().expect("captured image request");
        self.server.join().expect("image server thread");
        request
    }
}

fn read_request(stream: &mut impl Read) -> String {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 1024];
    loop {
        let read = stream.read(&mut buffer).expect("read image request");
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }
    String::from_utf8_lossy(&request).into_owned()
}

fn http_response(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut response = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
    if !headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("content-length"))
    {
        response.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    for (name, value) in headers {
        response.push_str(name);
        response.push_str(": ");
        response.push_str(value);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    let mut bytes = response.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn png_bytes(width: u32, height: u32) -> Vec<u8> {
    let rgba = vec![0_u8; width as usize * height as usize * 4];
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&rgba, width, height, ExtendedColorType::Rgba8)
        .expect("encode test png");
    bytes
}
