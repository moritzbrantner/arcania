use std::time::{SystemTime, UNIX_EPOCH};

use rand::RngCore;
use rand::rngs::OsRng;

pub(super) fn readable_match_id(attempt: u32) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(1);
    let suffix = if attempt == 0 {
        String::new()
    } else {
        format!("-{}", to_base36(u64::from(attempt)))
    };
    format!("arc-{}{}", to_base36(millis), suffix)
}

#[cfg(any(test, debug_assertions))]
pub(super) fn scenario_match_id(scenario_id: &str, attempt: u32) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(1);
    let suffix = if attempt == 0 {
        String::new()
    } else {
        format!("-{}", to_base36(u64::from(attempt)))
    };
    format!("dev-{}-{}{}", scenario_id, to_base36(millis), suffix)
}

pub(super) fn random_seat_token() -> String {
    random_hex_token(24)
}

pub(super) fn random_hex_token(byte_count: usize) -> String {
    let mut bytes = [0_u8; 24];
    let mut dynamic_bytes;
    let bytes = if byte_count == bytes.len() {
        OsRng.fill_bytes(&mut bytes);
        bytes.as_slice()
    } else {
        dynamic_bytes = vec![0_u8; byte_count];
        OsRng.fill_bytes(&mut dynamic_bytes);
        dynamic_bytes.as_slice()
    };
    let mut token = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        token.push_str(&format!("{byte:02x}"));
    }
    token
}

pub(super) fn to_base36(mut value: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if value == 0 {
        return "0".to_string();
    }

    let mut out = Vec::new();
    while value > 0 {
        out.push(DIGITS[(value % 36) as usize]);
        value /= 36;
    }
    out.reverse();
    String::from_utf8(out).expect("base36 should be valid ASCII")
}
