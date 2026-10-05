use super::*;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    thread,
};

fn session() -> Session {
    Session {
        google_id: "100000000000000000001".into(),
        cookie: "synthetic-session".into(),
    }
}

fn auth() -> serde_json::Value {
    serde_json::json!({"authData": {
        "accessToken": "synthetic-access-token", "expiresIn": 3600,
        "emailAddress": "alex@example.com", "googleId": session().google_id,
        "scope": "https://www.googleapis.com/auth/gmail.modify"
    }})
}

/// A local server exercises real HTTP, cookie handling and CSRF retries.
fn server(
    responses: Vec<(u16, String, serde_json::Value)>,
) -> (Url, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = Url::parse(&format!("http://{}", listener.local_addr().unwrap())).unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for (status, headers, body) in responses {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                assert!(reader.read_line(&mut line).unwrap() > 0);
                request.push_str(&line);
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
                if line == "\r\n" {
                    break;
                }
            }
            let mut bytes = vec![0; length];
            reader.read_exact(&mut bytes).unwrap();
            request.push_str(&String::from_utf8(bytes).unwrap());
            requests.push(request);
            let body = body.to_string();
            write!(stream, "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}", body.len()).unwrap();
        }
        requests
    });
    (url, handle)
}

#[test]
fn exchanges_csrf_and_session_for_a_matching_gmail_token() {
    let (url, handle) = server(vec![
        (
            200,
            String::new(),
            serde_json::json!({"csrfToken":"synthetic-csrf"}),
        ),
        (
            200,
            format!(
                "Set-Cookie: {}=rotated-session; Path=/; HttpOnly\r\n",
                session().google_id
            ),
            auth(),
        ),
    ]);
    let mut session = session();
    let client = SessionClient::at_origin(&session, url).unwrap();
    let result = client
        .exchange(&mut session, Some("alex@example.com"))
        .unwrap();
    assert_eq!(result.access_token, "synthetic-access-token");
    assert_eq!(session.cookie, "rotated-session");
    let requests = handle.join().unwrap();
    assert!(requests[0].starts_with("GET /~backend/v3/sessions.getCsrfToken "));
    assert!(requests[1].starts_with("POST /~backend/v3/sessions.getTokens "));
    assert!(requests[1].contains("x-csrf-token: synthetic-csrf"));
    assert!(requests[1].contains("cookie: 100000000000000000001=synthetic-session"));
    assert!(requests[1].contains("\"emailAddress\":\"alex@example.com\""));
}

#[test]
fn renews_csrf_once_on_forbidden() {
    let (url, handle) = server(vec![
        (
            200,
            String::new(),
            serde_json::json!({"csrfToken":"stale-csrf"}),
        ),
        (
            403,
            String::new(),
            serde_json::json!({"detail":"invalid-csrf"}),
        ),
        (
            200,
            String::new(),
            serde_json::json!({"csrfToken":"fresh-csrf"}),
        ),
        (200, String::new(), auth()),
    ]);
    let mut session = session();
    let result = SessionClient::at_origin(&session, url)
        .unwrap()
        .exchange(&mut session, None);
    assert!(result.is_ok());
    let requests = handle.join().unwrap();
    assert_eq!(requests.len(), 4);
    assert!(requests[3].contains("x-csrf-token: fresh-csrf"));
}

#[test]
fn expired_session_has_reconnect_action_and_never_echoes_response() {
    let (url, handle) = server(vec![
        (
            200,
            String::new(),
            serde_json::json!({"csrfToken":"synthetic-csrf"}),
        ),
        (
            401,
            String::new(),
            serde_json::json!({"detail":"synthetic-secret-must-not-leak"}),
        ),
    ]);
    let mut session = session();
    let error = SessionClient::at_origin(&session, url)
        .unwrap()
        .exchange(&mut session, None)
        .err()
        .unwrap();
    assert_eq!(error, RECONNECT);
    assert!(!error.contains("synthetic-secret"));
    handle.join().unwrap();
}

#[test]
fn rejects_wrong_account_expired_token_and_missing_gmail_scope() {
    let response: TokenResponse = serde_json::from_value(auth()).unwrap();
    let mut auth = response.auth_data;
    assert!(validate_auth(&auth, &session(), Some("ALEX@example.com")).is_ok());
    assert!(validate_auth(&auth, &session(), Some("another@example.com")).is_err());
    auth.google_id = "100000000000000000002".into();
    assert!(validate_auth(&auth, &session(), None).is_err());
    auth.google_id = session().google_id;
    auth.expires_in = 0;
    assert!(validate_auth(&auth, &session(), None).is_err());
    auth.expires_in = 3600;
    auth.scope = "https://www.googleapis.com/auth/gmail.readonly".into();
    assert!(validate_auth(&auth, &session(), None).is_err());
}

#[test]
fn rejects_cookie_header_injection() {
    for cookie in [
        "",
        "value; other=bad",
        "value\r\nX-Injected: bad",
        "value,other",
    ] {
        let mut session = session();
        session.cookie = cookie.into();
        assert!(validate_session(&session).is_err());
    }
}
