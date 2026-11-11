use crate::{domain::http::{NetworkError, TextResponse}, ports::http::HttpPort};

pub fn fetch_text(client: &impl HttpPort, url: &str) -> Result<TextResponse, NetworkError> {
    client.get_text(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fake;
    impl HttpPort for Fake {
        fn get_text(&self, _: &str) -> Result<TextResponse, NetworkError> {
            Ok(TextResponse { status: 200, body: "hello".into() })
        }
    }
    #[test]
    fn usecase_uses_only_the_port() {
        assert_eq!(fetch_text(&Fake, "https://example.com").unwrap().body, "hello");
    }
}
