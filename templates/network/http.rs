pub const MAX_BODY_BYTES: usize = 65_536;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkError {
    InvalidUrl,
    Unavailable,
    Timeout,
    Status,
    TooLarge,
    NotText,
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidUrl => "Enter an HTTPS URL without credentials or a fragment.",
            Self::Unavailable => "The secure connection could not be completed.",
            Self::Timeout => "The request timed out. Try again when the connection improves.",
            Self::Status => "The server returned an unsuccessful response or a redirect.",
            Self::TooLarge => "The response exceeds the 64 KiB limit.",
            Self::NotText => "The response is not valid UTF-8 text.",
        })
    }
}
impl std::error::Error for NetworkError {}
