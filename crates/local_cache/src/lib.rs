mod crypto;
mod keys;
mod paths;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use crypto::{
    open_bytes_v1, read_encrypted_json, read_encrypted_json_strict_bounded, seal_bytes_v1,
    write_encrypted_json, write_encrypted_json_durable_bounded, DurableEncryptedWriteError,
};
pub use keys::{
    cache_key_hash, decode_cache_key, encode_cache_key, generate_cache_key,
    load_or_create_cache_key,
};
pub use paths::{account_cache_dir, default_cache_root_dir};

#[cfg(test)]
mod tests;
