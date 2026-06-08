//! A captured gitweb CGI response, split into its header block and raw body.
//!
//! `gitweb.perl` writes an HTTP/CGI response: header lines, each ended by CRLF, a
//! blank CRLF line, then the body — and for a byte-stable endpoint the body is
//! emitted with `binmode STDOUT, ':raw'`, so it may hold NUL bytes or non-UTF-8
//! content. [`Golden`] keeps the body as raw bytes and never decodes it, so the
//! comparison stays exact.

use std::path::PathBuf;

/// A parsed golden: the captured header lines and the raw response body.
#[derive(Debug, Clone)]
pub struct Golden {
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// The CRLF blank line separating the CGI header block from the body.
const SEPARATOR: &[u8] = b"\r\n\r\n";

impl Golden {
    /// Parses a raw captured CGI response into its headers and body.
    ///
    /// The header block ends at the first blank CRLF line; everything after it is
    /// the body, kept as raw bytes so NUL and non-UTF-8 content survive intact. A
    /// response with no blank line (which gitweb never emits) is treated as a
    /// body with no headers.
    #[must_use]
    pub fn parse(raw: &[u8]) -> Self {
        let Some(end): Option<usize> = raw
            .windows(SEPARATOR.len())
            .position(|window: &[u8]| window == SEPARATOR)
        else {
            return Self {
                headers: Vec::new(),
                body: raw.to_vec(),
            };
        };
        Self {
            headers: parse_headers(&raw[..end]),
            body: raw[end + SEPARATOR.len()..].to_vec(),
        }
    }

    /// Loads and parses the committed golden at `relative` under `goldens/`.
    ///
    /// # Panics
    /// Panics if the file is missing — a feature naming a golden that was never
    /// captured is a broken test, so it should fail loudly rather than skip.
    #[must_use]
    pub fn load(relative: &str) -> Self {
        let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("goldens")
            .join(relative);
        let raw: Vec<u8> = std::fs::read(&path)
            .unwrap_or_else(|err: std::io::Error| panic!("read golden {}: {err}", path.display()));
        Self::parse(&raw)
    }

    /// The raw response body bytes.
    #[must_use]
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// The value of header `name`, matched case-insensitively, or `None`.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _): &&(String, String)| key.eq_ignore_ascii_case(name))
            .map(|(_, value): &(String, String)| value.as_str())
    }
}

/// Splits a CGI header block (`Name: value` lines joined by CRLF, ASCII) into
/// name/value pairs, trimming surrounding whitespace. Lines without a colon are
/// dropped; the header block is ASCII, so a lossy decode never loses anything.
fn parse_headers(block: &[u8]) -> Vec<(String, String)> {
    String::from_utf8_lossy(block)
        .split("\r\n")
        .filter_map(|line: &str| {
            let (name, value): (&str, &str) = line.split_once(':')?;
            Some((name.trim().to_owned(), value.trim().to_owned()))
        })
        .collect()
}
