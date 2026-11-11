/// Decode advertisement payloads on the Rust side of the bridge.
/// Real sensor protocols should expose typed domain values instead.
#[flutter_rust_bridge::frb(sync)]
pub fn describe_packet(bytes: Vec<u8>) -> String {
    if bytes.is_empty() {
        return "No data".into();
    }
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_retains_leading_zeroes() {
        assert_eq!(describe_packet(vec![0, 1, 255]), "00 01 FF");
    }

    #[test]
    fn empty_payload_is_explicit() {
        assert_eq!(describe_packet(vec![]), "No data");
    }
}
