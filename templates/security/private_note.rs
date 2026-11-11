use crate::ports::crypto::CryptoPort;

pub struct PrivateNoteService<C: CryptoPort>(pub C);
impl<C: CryptoPort> PrivateNoteService<C> {
    pub fn seal(&self, key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, &'static str> {
        self.0.seal(key, plaintext)
    }
    pub fn open(&self, key: &[u8], envelope: &[u8]) -> Result<Vec<u8>, &'static str> {
        self.0.open(key, envelope)
    }
}
