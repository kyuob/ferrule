use crate::{domain::http::{NetworkError, TextResponse}, infra::https::HttpsClient, usecase::fetch_text};

pub fn fetch(url: &str) -> Result<TextResponse, NetworkError> {
    fetch_text::fetch_text(&HttpsClient::new()?, url)
}
