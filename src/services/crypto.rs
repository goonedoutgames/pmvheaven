use crate::services::db::{get_setting, set_setting};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::sync::Mutex;

static KEY: Mutex<Option<[u8; 32]>> = Mutex::new(None);

fn get_key() -> [u8; 32] {
    {
        let guard = KEY.lock().unwrap();
        if let Some(k) = *guard {
            return k;
        }
    }

    // Load/create outside KEY lock — this touches the DB mutex.
    let key = if let Ok(secret) = std::env::var("PH_SECRET") {
        if !secret.is_empty() {
            let hash = Sha256::digest(secret.as_bytes());
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&hash);
            arr
        } else {
            load_or_create_stored_key()
        }
    } else {
        load_or_create_stored_key()
    };

    let mut guard = KEY.lock().unwrap();
    if let Some(k) = *guard {
        return k;
    }
    *guard = Some(key);
    key
}

fn load_or_create_stored_key() -> [u8; 32] {
    if let Some(stored) = get_setting("crypto_key") {
            let bytes = hex_decode(&stored).unwrap_or_else(|_| {
            let mut k = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut k);
            set_setting("crypto_key", &hex_encode(&k));
            k.to_vec()
        });
        let mut arr = [0u8; 32];
        if bytes.len() == 32 {
            arr.copy_from_slice(&bytes);
            arr
        } else {
            let mut k = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut k);
            set_setting("crypto_key", &hex_encode(&k));
            k
        }
    } else {
        let mut k = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut k);
        set_setting("crypto_key", &hex_encode(&k));
        k
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

/// Ensure the AES key is loaded (may touch the DB). Call before holding the DB lock
/// around encrypt/decrypt of other data.
pub fn ensure_key() {
    let _ = get_key();
}

/// Format: `iv_b64.tag_b64.ciphertext_b64`
pub fn encrypt(plain: &str) -> String {
    let key = get_key();
    let cipher = Aes256Gcm::new_from_slice(&key).expect("key");
    let mut iv = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut iv);
    let nonce = Nonce::from_slice(&iv);
    let ciphertext = cipher
        .encrypt(nonce, plain.as_bytes())
        .expect("encrypt");
    // aes-gcm crate appends tag to ciphertext; split last 16 bytes
    let (ct, tag) = ciphertext.split_at(ciphertext.len().saturating_sub(16));
    format!("{}.{}.{}", B64.encode(iv), B64.encode(tag), B64.encode(ct))
}

pub fn decrypt(payload: &str) -> Result<String, String> {
    let parts: Vec<&str> = payload.split('.').collect();
    if parts.len() != 3 {
        return Err("Malformed ciphertext".into());
    }
    let iv = B64.decode(parts[0]).map_err(|e| e.to_string())?;
    let tag = B64.decode(parts[1]).map_err(|e| e.to_string())?;
    let ct = B64.decode(parts[2]).map_err(|e| e.to_string())?;
    let key = get_key();
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(&iv);
    let mut combined = ct;
    combined.extend_from_slice(&tag);
    let plain = cipher
        .decrypt(nonce, combined.as_ref())
        .map_err(|_| "decrypt failed".to_string())?;
    String::from_utf8(plain).map_err(|e| e.to_string())
}
