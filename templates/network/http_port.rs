use crate::domain::http::{NetworkError, TextResponse};

pub trait HttpPort {
    fn get_text(&self, url: &str) -> Result<TextResponse, NetworkError>;
}
