pub trait CryptoPort {
    fn seal(&self, key: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, &'static str>;
    fn open(&self, key: &[u8], envelope: &[u8]) -> Result<Vec<u8>, &'static str>;
}
