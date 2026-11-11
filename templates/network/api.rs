pub struct HttpTextResponse {
    pub status: u16,
    pub body: String,
}

/// Runs on FRB's worker pool. No network work on Flutter's UI isolate.
pub fn fetch_https_text(url: String) -> Result<HttpTextResponse, String> {
    crate::composition::network::fetch(&url)
        .map(|result| HttpTextResponse { status: result.status, body: result.body })
        .map_err(|error| error.to_string())
}
