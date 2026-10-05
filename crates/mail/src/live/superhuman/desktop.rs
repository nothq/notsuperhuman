use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use rusqlite::{Connection, OpenFlags};
use security_framework::item::{ItemClass, ItemSearchOptions, SearchResult};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use super::Session;

const COOKIE_HOST: &str = "accounts.superhuman.com";

pub(super) fn sessions() -> Result<Vec<Session>, String> {
    let path = dirs::home_dir()
        .ok_or("Could not locate your home directory")?
        .join("Library/Application Support/Superhuman/Cookies");
    if !path.exists() {
        return Err(
            "Open Superhuman Desktop and sign in to Gmail, then choose Continue with Superhuman."
                .into(),
        );
    }
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|_| {
        "Could not read Superhuman Desktop's sessions. Open Superhuman and try again.".to_string()
    })?;
    db.busy_timeout(std::time::Duration::from_secs(3))
        .map_err(|_| "Could not read Superhuman Desktop sessions".to_string())?;
    let version: u32 = db
        .query_row("SELECT value FROM meta WHERE key='version'", [], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|_| "Unrecognized Superhuman Desktop cookie database".to_string())?
        .parse()
        .map_err(|_| "Unrecognized Superhuman Desktop cookie version".to_string())?;
    // No browser profiles, analytics cookies, or unrelated hosts are read.
    let mut query = db.prepare("SELECT name, value, encrypted_value FROM cookies WHERE host_key = ?1 AND (expires_utc = 0 OR expires_utc > (strftime('%s','now') + 11644473600) * 1000000) ORDER BY creation_utc")
        .map_err(|_| "Could not read Superhuman Desktop sessions".to_string())?;
    let cookies = query
        .query_map([COOKIE_HOST], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(|_| "Could not read Superhuman Desktop sessions".to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Could not read Superhuman Desktop sessions".to_string())?;
    let cookies: Vec<_> = cookies
        .into_iter()
        .filter(|(name, _, _)| {
            (6..=64).contains(&name.len()) && name.bytes().all(|b| b.is_ascii_digit())
        })
        .collect();
    if cookies.is_empty() {
        return Err(
            "Sign in to Gmail in Superhuman Desktop, then choose Continue with Superhuman again."
                .into(),
        );
    }
    let key = if cookies
        .iter()
        .any(|(_, _, encrypted)| !encrypted.is_empty())
    {
        Some(cookie_key()?)
    } else {
        None
    };
    cookies
        .into_iter()
        .map(|(google_id, plaintext, encrypted)| {
            let cookie = if encrypted.is_empty() {
                plaintext
            } else {
                decrypt_cookie(
                    &encrypted,
                    key.as_ref().ok_or("Superhuman session key unavailable")?,
                    version,
                )?
            };
            let session = Session { google_id, cookie };
            super::validate_session(&session)?;
            Ok(session)
        })
        .collect()
}

fn cookie_key() -> Result<Zeroizing<[u8; 16]>, String> {
    let items = ItemSearchOptions::new()
        .class(ItemClass::generic_password())
        .service("Superhuman Safe Storage")
        .load_data(true)
        .search()
        .map_err(|_| {
            "Allow access to Superhuman Safe Storage in macOS Keychain to connect, then try again."
                .to_string()
        })?;
    let password = items
        .into_iter()
        .find_map(|item| match item {
            SearchResult::Data(data) => Some(Zeroizing::new(data)),
            _ => None,
        })
        .ok_or(
            "Superhuman's Keychain entry is unavailable. Open Superhuman Desktop and try again.",
        )?;
    let mut key = Zeroizing::new([0u8; 16]);
    pbkdf2::pbkdf2_hmac::<sha1::Sha1>(&password, b"saltysalt", 1003, key.as_mut());
    Ok(key)
}

fn decrypt_cookie(encrypted: &[u8], key: &[u8; 16], version: u32) -> Result<String, String> {
    let ciphertext = encrypted
        .strip_prefix(b"v10")
        .ok_or("This Superhuman Desktop cookie encryption version is unsupported")?;
    let plaintext = Zeroizing::new(
        cbc::Decryptor::<aes::Aes128>::new(key.into(), (&[b' '; 16]).into())
            .decrypt_padded_vec_mut::<Pkcs7>(ciphertext)
            .map_err(|_| "Could not unlock the Superhuman Desktop session".to_string())?,
    );
    let value = if version >= 24 {
        let digest = Sha256::digest(COOKIE_HOST.as_bytes());
        if !plaintext.starts_with(&digest) {
            return Err("Superhuman session cookie failed its host check".into());
        }
        &plaintext[32..]
    } else {
        plaintext.as_slice()
    };
    String::from_utf8(value.to_vec())
        .map_err(|_| "Invalid Superhuman Desktop session encoding".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockEncryptMut;

    #[test]
    fn decrypts_chromium_cookie_and_checks_host_binding() {
        let key = [7u8; 16];
        for version in [23, 24] {
            let mut value = if version >= 24 {
                Sha256::digest(COOKIE_HOST.as_bytes()).to_vec()
            } else {
                Vec::new()
            };
            value.extend(b"synthetic-session");
            let mut encrypted = b"v10".to_vec();
            encrypted.extend(
                cbc::Encryptor::<aes::Aes128>::new((&key).into(), (&[b' '; 16]).into())
                    .encrypt_padded_vec_mut::<Pkcs7>(&value),
            );
            assert_eq!(
                decrypt_cookie(&encrypted, &key, version).unwrap(),
                "synthetic-session"
            );
            assert!(decrypt_cookie(&encrypted, &[8u8; 16], version).is_err());
            if version == 23 {
                assert!(decrypt_cookie(&encrypted, &key, 24).is_err());
            }
        }
        assert!(decrypt_cookie(b"v20bad", &key, 24).is_err());
    }
}
