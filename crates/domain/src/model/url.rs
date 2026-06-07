//! URL-decoding of request and file tokens (gitweb's `CGI::Util::unescape`).
//!
//! gitweb decodes the projects-list file and incoming query parameters the same
//! way: a `+` is a space, and `%XX` is the byte `0xXX`. Decoded bytes are
//! reassembled into UTF-8. An escape that is not two hex digits is left exactly
//! as written, matching CGI::Util's lenient pass-through.

/// Decodes one `application/x-www-form-urlencoded` token: `+` → space, `%XX` →
/// the byte `0xXX`, with the resulting bytes interpreted as UTF-8 (lossily, so
/// the result is always a valid `String`).
#[must_use]
pub fn unescape(token: &str) -> String {
    let bytes: &[u8] = token.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut index: usize = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                out.push(b' ');
                index += 1;
            }
            b'%' => match decode_escape(bytes, index) {
                Some(byte) => {
                    out.push(byte);
                    index += 3;
                }
                None => {
                    out.push(b'%');
                    index += 1;
                }
            },
            other => {
                out.push(other);
                index += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The byte encoded by the `%XX` starting at `percent` in `bytes`, or `None`
/// when fewer than two hex digits follow the `%`.
fn decode_escape(bytes: &[u8], percent: usize) -> Option<u8> {
    let high: u8 = hex_value(*bytes.get(percent + 1)?)?;
    let low: u8 = hex_value(*bytes.get(percent + 2)?)?;
    Some((high << 4) | low)
}

/// The numeric value of one ASCII hex digit, or `None` for anything else.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
