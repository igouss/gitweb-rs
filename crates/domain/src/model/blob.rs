//! Blob objects: raw file content.
//!
//! gitweb decides between inline display and a download link by asking whether
//! the content looks binary (`is_binary` heuristic) and how large it is. Those
//! are the only questions the entity answers; decoding to text is the caller's
//! job via [`crate::model::encoding`].

use crate::model::binary::is_binary;

/// A git blob: an opaque byte string.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Blob {
    content: Vec<u8>,
}

impl Blob {
    /// Wraps raw blob bytes.
    #[must_use]
    pub fn new(content: Vec<u8>) -> Self {
        Self { content }
    }

    /// The raw bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.content
    }

    /// The size in bytes.
    #[must_use]
    pub fn size(&self) -> usize {
        self.content.len()
    }

    /// Whether the content looks binary (gitweb's NUL-byte heuristic).
    #[must_use]
    pub fn is_binary(&self) -> bool {
        is_binary(&self.content)
    }
}
