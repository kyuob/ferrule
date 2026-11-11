use crate::composition::security::private_notes;
use zeroize::Zeroizing;

pub fn generate_vault_key() -> Result<Vec<u8>, String> {
    let mut key = Zeroizing::new(vec![0; 32]);
    getrandom::fill(&mut key).map_err(|_| "Secure randomness is unavailable.".to_string())?;
    Ok(key.to_vec())
}
pub fn seal_private_note(key: Vec<u8>, plaintext: String) -> Result<Vec<u8>, String> {
    let key = Zeroizing::new(key);
    let plaintext = Zeroizing::new(plaintext);
    private_notes()
        .seal(&key, plaintext.as_bytes())
        .map_err(str::to_owned)
}
pub fn open_private_note(key: Vec<u8>, envelope: Vec<u8>) -> Result<String, String> {
    let key = Zeroizing::new(key);
    let bytes = Zeroizing::new(
        private_notes()
            .open(&key, &envelope)
            .map_err(str::to_owned)?,
    );
    String::from_utf8(bytes.to_vec()).map_err(|_| "The private note could not be read.".to_string())
}
