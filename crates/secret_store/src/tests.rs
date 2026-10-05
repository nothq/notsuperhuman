use std::fs;

use serde_json::Value;
use tempfile::TempDir;

use crate::*;

struct TestStore {
    _tempdir: TempDir,
    auth_path: PathBuf,
    store: SecretStore,
}

impl TestStore {
    fn new() -> Self {
        let tempdir = TempDir::new().expect("tempdir");
        let auth_path = tempdir.path().join(".notsuperhuman").join("auth.json");
        let store = SecretStore::for_test(auth_path.clone());
        Self {
            _tempdir: tempdir,
            auth_path,
            store,
        }
    }

    fn json(&self) -> Value {
        let contents = fs::read_to_string(&self.auth_path).expect("auth file contents");
        serde_json::from_str(&contents).expect("auth file json")
    }
}

#[test]
fn read_returns_none_when_credentials_file_is_missing() {
    let fixture = TestStore::new();

    assert!(!fixture.auth_path.exists());
    assert_eq!(
        fixture
            .store
            .read_secret("mail:jmap:ada@example.com")
            .expect("read secret"),
        None
    );
    assert_eq!(
        fixture
            .store
            .read_cache_key("mail:jmap:ada@example.com")
            .expect("read cache key"),
        None
    );
}

#[test]
fn upsert_and_read_round_trip_generic_secrets() {
    let fixture = TestStore::new();
    fixture
        .store
        .upsert_secret("mail:jmap:ada@example.com", "top-secret")
        .expect("store secret");

    assert_eq!(
        fixture
            .store
            .read_secret("mail:jmap:ada@example.com")
            .expect("read secret")
            .as_deref(),
        Some("top-secret")
    );
    assert_eq!(
        fixture.json()["secrets"]["mail:jmap:ada@example.com"].as_str(),
        Some("top-secret")
    );
}

#[test]
fn secrets_and_cache_keys_use_separate_storage() {
    let fixture = TestStore::new();
    fixture
        .store
        .upsert_secret("mail:superhuman:ada@example.com", "secret-1")
        .expect("store secret");
    fixture
        .store
        .upsert_cache_key("mail:superhuman:ada@example.com", "cache-key-1")
        .expect("store cache key");

    assert_eq!(
        fixture
            .store
            .read_secret("mail:superhuman:ada@example.com")
            .expect("read secret")
            .as_deref(),
        Some("secret-1")
    );
    assert_eq!(
        fixture
            .store
            .read_cache_key("mail:superhuman:ada@example.com")
            .expect("read cache key")
            .as_deref(),
        Some("cache-key-1")
    );
    let json = fixture.json();
    assert_eq!(
        json["secrets"]["mail:superhuman:ada@example.com"].as_str(),
        Some("secret-1")
    );
    assert_eq!(
        json["cache_keys"]["mail:superhuman:ada@example.com"].as_str(),
        Some("cache-key-1")
    );
}

#[test]
fn cache_keys_are_isolated() {
    let fixture = TestStore::new();
    fixture
        .store
        .upsert_cache_key("mail:superhuman:a@example.com|mail", "cache-key-a")
        .expect("store cache key a");
    fixture
        .store
        .upsert_cache_key("mail:superhuman:b@example.com|mail", "cache-key-b")
        .expect("store cache key b");

    assert_eq!(
        fixture
            .store
            .read_cache_key("mail:superhuman:a@example.com|mail")
            .expect("read cache key a")
            .as_deref(),
        Some("cache-key-a")
    );
    assert_eq!(
        fixture
            .store
            .read_cache_key("mail:superhuman:b@example.com|mail")
            .expect("read cache key b")
            .as_deref(),
        Some("cache-key-b")
    );
}

#[test]
fn corrupt_credentials_file_fails_loudly() {
    let fixture = TestStore::new();
    fs::create_dir_all(fixture.auth_path.parent().expect("auth path parent")).expect("auth dir");
    fs::write(&fixture.auth_path, "not json").expect("corrupt auth file");

    let error = fixture
        .store
        .read_secret("mail:jmap:ada@example.com")
        .expect_err("corrupt credentials should fail");

    assert!(
        format!("{error:#}").contains("failed to decode notsuperhuman credentials"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn unsupported_credentials_version_fails_loudly() {
    let fixture = TestStore::new();
    fs::create_dir_all(fixture.auth_path.parent().expect("auth path parent")).expect("auth dir");
    fs::write(
        &fixture.auth_path,
        r#"{"version":2,"secrets":{},"cache_keys":{}}"#,
    )
    .expect("unsupported version auth file");

    let error = fixture
        .store
        .read_secret("mail:jmap:ada@example.com")
        .expect_err("unsupported credentials version should fail");

    assert!(
        format!("{error:#}").contains("unsupported notsuperhuman credentials version 2"),
        "unexpected error: {error:#}"
    );
}

#[cfg(unix)]
#[test]
fn credentials_file_uses_private_unix_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = TestStore::new();
    fixture
        .store
        .upsert_secret("mail:jmap:ada@example.com", "top-secret")
        .expect("store secret");

    let auth_dir = fixture.auth_path.parent().expect("auth path parent");
    let dir_mode = fs::metadata(auth_dir)
        .expect("auth dir metadata")
        .permissions()
        .mode()
        & 0o777;
    let file_mode = fs::metadata(&fixture.auth_path)
        .expect("auth file metadata")
        .permissions()
        .mode()
        & 0o777;

    assert_eq!(dir_mode, 0o700);
    assert_eq!(file_mode, 0o600);
}
