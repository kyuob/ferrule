use crate::{domain::http::{NetworkError, TextResponse, MAX_BODY_BYTES}, ports::http::HttpPort};
use reqwest::{blocking::{Client, ClientBuilder}, redirect::Policy, Url};
use std::{io::Read, time::Duration};

pub struct HttpsClient(Client);

fn connection_error(error: reqwest::Error) -> NetworkError {
    // Never expose request URLs, query strings or native diagnostic details.
    if error.is_timeout() { NetworkError::Timeout } else { NetworkError::Unavailable }
}

fn validated_url(input: &str) -> Result<Url, NetworkError> {
    if input.len() > 2048 || input.chars().any(char::is_control) {
        return Err(NetworkError::InvalidUrl);
    }
    let url = Url::parse(input).map_err(|_| NetworkError::InvalidUrl)?;
    if url.scheme() != "https" || url.host_str().is_none() || !url.username().is_empty()
        || url.password().is_some() || url.fragment().is_some() {
        return Err(NetworkError::InvalidUrl);
    }
    Ok(url)
}

fn text_body(reader: impl Read, length: Option<u64>) -> Result<String, NetworkError> {
    if length.is_some_and(|n| n > MAX_BODY_BYTES as u64) {
        return Err(NetworkError::TooLarge);
    }
    // Enforce the cap even when Content-Length is absent or incorrect.
    let mut bytes = Vec::new();
    reader.take(MAX_BODY_BYTES as u64 + 1).read_to_end(&mut bytes)
        .map_err(|error| if error.kind() == std::io::ErrorKind::TimedOut
            || error.get_ref().and_then(|inner| inner.downcast_ref::<reqwest::Error>())
                .is_some_and(reqwest::Error::is_timeout) {
            NetworkError::Timeout
        } else { NetworkError::Unavailable })?;
    if bytes.len() > MAX_BODY_BYTES { return Err(NetworkError::TooLarge); }
    String::from_utf8(bytes).map_err(|_| NetworkError::NotText)
}

impl HttpsClient {
    fn builder() -> ClientBuilder {
        // reqwest 0.12's rustls-tls feature uses bundled WebPKI roots and ring.
        // No native TLS, system proxy, cookies, decompression or optional HTTP/2.
        Client::builder().use_rustls_tls().https_only(true).no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
    }
    pub fn new() -> Result<Self, NetworkError> {
        Self::builder().build().map(Self).map_err(connection_error)
    }
}

impl HttpPort for HttpsClient {
    fn get_text(&self, input: &str) -> Result<TextResponse, NetworkError> {
        let url = validated_url(input)?;
        let response = self.0.get(url).timeout(Duration::from_secs(10))
            .header("Accept", "text/plain, application/json, text/html")
            .send().map_err(connection_error)?;
        let status = response.status();
        if !status.is_success() { return Err(NetworkError::Status); }
        let length = response.content_length();
        Ok(TextResponse { status: status.as_u16(), body: text_body(response, length)? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_insecure_or_credential_urls_before_network() {
        for input in ["http://example.com", "file:///etc/passwd", "https://a:b@example.com", "https://a@example.com", "https://example.com/#fragment", "https://example.com/\n"] {
            assert_eq!(validated_url(input), Err(NetworkError::InvalidUrl));
        }
        assert!(validated_url(&format!("https://example.com/{}", "a".repeat(2048))).is_err());
        assert!(validated_url("https://example.com/path?q=1").is_ok());
    }
    #[test]
    fn bounds_streaming_and_declared_bodies_and_rejects_non_utf8() {
        assert_eq!(text_body(&b"hello"[..], None).unwrap(), "hello");
        assert_eq!(text_body(&b""[..], Some(65_537)), Err(NetworkError::TooLarge));
        assert_eq!(text_body(&vec![b'x'; 65_537][..], None), Err(NetworkError::TooLarge));
        assert_eq!(text_body(&vec![b'x'; 65_536][..], None).unwrap().len(), 65_536);
        assert_eq!(text_body(&[0xff][..], None), Err(NetworkError::NotText));
    }
    #[test]
    fn client_constructs_without_a_global_crypto_provider() {
        assert!(HttpsClient::new().is_ok());
    }
    /// Executed explicitly by network-smoke.sh against its private HTTPS fixture.
    #[test]
    #[ignore = "requires the local TLS fixture started by network-smoke.sh"]
    fn local_tls_transport_contract() {
        let origin = std::env::var("FERRULE_TLS_ORIGIN").unwrap();
        let certificate = std::fs::read(std::env::var("FERRULE_TLS_CERT").unwrap()).unwrap();
        let trusted = HttpsClient(HttpsClient::builder()
            .add_root_certificate(reqwest::Certificate::from_pem(&certificate).unwrap())
            .build().unwrap());
        let untrusted = HttpsClient::new().unwrap();
        assert_eq!(untrusted.get_text(&format!("{origin}/ok")), Err(NetworkError::Unavailable));
        let response = trusted.get_text(&format!("{origin}/ok")).unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "hello TLS");
        for path in ["redirect", "error"] {
            assert_eq!(trusted.get_text(&format!("{origin}/{path}")), Err(NetworkError::Status));
        }
        for path in ["large", "stream"] {
            assert_eq!(trusted.get_text(&format!("{origin}/{path}")), Err(NetworkError::TooLarge));
        }
        assert_eq!(trusted.get_text(&format!("{origin}/binary")), Err(NetworkError::NotText));
        for path in ["slow", "slow_body", "drip"] {
            let start = std::time::Instant::now();
            assert_eq!(trusted.get_text(&format!("{origin}/{path}")), Err(NetworkError::Timeout));
            assert!(start.elapsed() < Duration::from_secs(13));
        }
        let wrong_host = origin.replace("localhost", "127.0.0.1");
        assert_eq!(trusted.get_text(&format!("{wrong_host}/ok")), Err(NetworkError::Unavailable));
    }
}
