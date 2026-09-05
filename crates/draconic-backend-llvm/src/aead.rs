//! AES-256-GCM AEAD for L10.02 (`aeadEncrypt` / `aeadDecrypt`).
//!
//! Designed algorithm: AES-256-GCM with a 32-byte key, 12-byte nonce, empty
//! AAD, and ciphertext || 16-byte tag.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};

pub(crate) const KEY_LEN: usize = 32;
pub(crate) const NONCE_LEN: usize = 12;
pub(crate) const TAG_LEN: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AeadError {
    KeyLen,
    NonceLen,
    CiphertextLen,
    Auth,
}

pub(crate) fn encrypt(key: &[u8], nonce: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, AeadError> {
    let cipher = cipher_for(key, nonce)?;
    cipher
        .encrypt(Nonce::from_slice(nonce), plaintext)
        .map_err(|_| AeadError::Auth)
}

pub(crate) fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, AeadError> {
    if ciphertext.len() < TAG_LEN {
        return Err(AeadError::CiphertextLen);
    }
    let cipher = cipher_for(key, nonce)?;
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| AeadError::Auth)
}

fn cipher_for(key: &[u8], nonce: &[u8]) -> Result<Aes256Gcm, AeadError> {
    if key.len() != KEY_LEN {
        return Err(AeadError::KeyLen);
    }
    if nonce.len() != NONCE_LEN {
        return Err(AeadError::NonceLen);
    }
    Aes256Gcm::new_from_slice(key).map_err(|_| AeadError::KeyLen)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex;

    /// NIST SP 800-38D Appendix B Test Case 13 (AES-256, empty PT).
    const KEY13: [u8; 32] = [0; 32];
    const NONCE13: [u8; 12] = [0; 12];
    const TAG13: &str = "530f8afbc74536b9a963b4f1c4cb738b";

    /// NIST SP 800-38D Appendix B Test Case 14 (AES-256, 16-byte PT).
    const PT14: [u8; 16] = [0; 16];
    const CT14: &str = "cea7403d4d606b6e074ec5d3baf39d18d0d1c8a799996bf0265b98b5d48ab919";

    #[test]
    fn nist_case13_empty_plaintext() {
        let ct = encrypt(&KEY13, &NONCE13, &[]).expect("encrypt");
        assert_eq!(hex::encode(&ct), TAG13);
        assert_eq!(decrypt(&KEY13, &NONCE13, &ct).expect("decrypt"), b"");
    }

    #[test]
    fn nist_case14_one_block() {
        let ct = encrypt(&KEY13, &NONCE13, &PT14).expect("encrypt");
        assert_eq!(hex::encode(&ct), CT14);
        assert_eq!(decrypt(&KEY13, &NONCE13, &ct).expect("decrypt"), PT14);
    }

    #[test]
    fn roundtrip_hello() {
        let key = hex::decode("feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308")
            .expect("key");
        let nonce = hex::decode("cafebabefacedbaddecaf888").expect("nonce");
        let ct = encrypt(&key, &nonce, b"hello").expect("encrypt");
        assert_eq!(ct.len(), b"hello".len() + TAG_LEN);
        assert_eq!(decrypt(&key, &nonce, &ct).expect("decrypt"), b"hello");
    }

    #[test]
    fn rejects_wrong_lengths_and_tamper() {
        assert_eq!(encrypt(&[0; 16], &[0; 12], b""), Err(AeadError::KeyLen));
        assert_eq!(encrypt(&[0; 32], &[0; 8], b""), Err(AeadError::NonceLen));
        assert_eq!(
            decrypt(&[0; 32], &[0; 12], &[0; 8]),
            Err(AeadError::CiphertextLen)
        );
        assert_eq!(decrypt(&[0; 32], &[0; 12], &[0; 16]), Err(AeadError::Auth));
    }
}
