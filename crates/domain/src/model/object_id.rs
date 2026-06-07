//! Git object identifiers (content hashes).
//!
//! Mirrors gitweb's `$oid_regex`: an object id is a hex string of either 40
//! (SHA-1) or 64 (SHA-256) characters. The UI shows an abbreviation of the
//! leading characters — seven by default, matching gitweb's `substr($h, 0, 7)`.

/// Number of hex characters in a SHA-1 object id.
const SHA1_LEN: usize = 40;
/// Number of hex characters in a SHA-256 object id.
const SHA256_LEN: usize = 64;
/// Default abbreviation length (gitweb's `substr($hash, 0, 7)`).
const DEFAULT_ABBREV: usize = 7;

/// A validated git object id, preserving the original hex text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectId {
    hex: String,
}

impl ObjectId {
    /// Parses a hex object id, returning `None` unless it is exactly 40 or 64
    /// hexadecimal characters.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let is_oid_length: bool = text.len() == SHA1_LEN || text.len() == SHA256_LEN;
        if is_oid_length && text.bytes().all(|byte: u8| byte.is_ascii_hexdigit()) {
            Some(Self {
                hex: text.to_owned(),
            })
        } else {
            None
        }
    }

    /// The full hex string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.hex
    }

    /// Whether this is the all-zero null id git shows for the missing side of an
    /// addition or deletion (gitweb's `'0' x 40` / `'0' x 64`).
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.hex.bytes().all(|byte: u8| byte == b'0')
    }

    /// The all-zero null id of this id's hash length — the id git shows for the
    /// absent side of an addition or deletion.
    #[must_use]
    pub fn null_like(&self) -> Self {
        Self {
            hex: "0".repeat(self.hex.len()),
        }
    }

    /// The default seven-character abbreviation.
    #[must_use]
    pub fn abbreviated(&self) -> &str {
        self.abbreviated_to(DEFAULT_ABBREV)
    }

    /// The abbreviation to `len` leading characters, clamped to the full id.
    ///
    /// Byte-indexing is safe here: a validated object id is pure ASCII hex, so
    /// every character is one byte wide.
    #[must_use]
    pub fn abbreviated_to(&self, len: usize) -> &str {
        let end: usize = len.min(self.hex.len());
        &self.hex[..end]
    }
}
