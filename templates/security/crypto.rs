use crate::ports::crypto::CryptoPort;
use chacha20poly1305::{
    aead::{Aead, KeyInit, Payload},
    XChaCha20Poly1305, XNonce,
};

pub const MAX_NOTE_BYTES: usize = 65_536;
const HEADER: &[u8] = b"FRN1";
const AAD: &[u8] = b"ferrule:private-note:v1";
const OVERHEAD: usize = 4 + 24 + 16;
const FAILED: &str = "The private note could not be authenticated.";

pub struct RustCrypto;
impl CryptoPort for RustCrypto {
    fn seal(&self, key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, &'static str> {
        if plaintext.len() > MAX_NOTE_BYTES {
            return Err("The private note is too long.");
        }
        let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| FAILED)?;
        let mut nonce = [0u8; 24];
        getrandom::fill(&mut nonce).map_err(|_| "Secure randomness is unavailable.")?;
        let encrypted = cipher
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: plaintext,
                    aad: AAD,
                },
            )
            .map_err(|_| FAILED)?;
        let mut result = Vec::with_capacity(OVERHEAD + plaintext.len());
        result.extend_from_slice(HEADER);
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&encrypted);
        Ok(result)
    }
    fn open(&self, key: &[u8], envelope: &[u8]) -> Result<Vec<u8>, &'static str> {
        if envelope.len() < OVERHEAD
            || envelope.len() > MAX_NOTE_BYTES + OVERHEAD
            || &envelope[..4] != HEADER
        {
            return Err(FAILED);
        }
        let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| FAILED)?;
        let nonce: [u8; 24] = envelope[4..28].try_into().map_err(|_| FAILED)?;
        cipher
            .decrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &envelope[28..],
                    aad: AAD,
                },
            )
            .map_err(|_| FAILED)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_is_randomized_and_rejects_wrong_keys_and_every_changed_byte() {
        let key = [42; 32];
        let text = "비공개 메모 🔐".as_bytes();
        let sealed = RustCrypto.seal(&key, text).unwrap();
        assert_eq!(RustCrypto.open(&key, &sealed).unwrap(), text);
        assert_ne!(sealed, RustCrypto.seal(&key, text).unwrap());
        assert!(RustCrypto.open(&[43; 32], &sealed).is_err());
        for i in 0..sealed.len() {
            let mut changed = sealed.clone();
            changed[i] ^= 1;
            assert!(RustCrypto.open(&key, &changed).is_err());
        }
        for len in 0..sealed.len() {
            assert!(RustCrypto.open(&key, &sealed[..len]).is_err());
        }
    }
    #[test]
    fn empty_note_limits_and_invalid_key_lengths() {
        let key = [3; 32];
        let empty = RustCrypto.seal(&key, b"").unwrap();
        assert!(RustCrypto.open(&key, &empty).unwrap().is_empty());
        assert!(RustCrypto.seal(&key, &vec![0; MAX_NOTE_BYTES + 1]).is_err());
        assert!(RustCrypto.seal(&[0; 31], b"x").is_err());
        assert!(RustCrypto.open(&[], &empty).is_err());
    }
}
