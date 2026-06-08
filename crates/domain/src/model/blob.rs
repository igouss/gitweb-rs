//! Blob objects: raw file content.
//!
//! gitweb decides between inline display and a download link by asking whether
//! the content looks binary (`is_binary` heuristic) and how large it is. Those
//! are the only questions the entity answers; decoding to text is the caller's
//! job via [`crate::model::encoding`].

use crate::model::binary::is_binary;

/// How gitweb's `git_blob` decides to present a blob.
///
/// gitweb shows a blob inline as text, inline as an `<img>`, or — for any other
/// binary content — not at all, offering only the raw download. Which one is the
/// outcome of [`Blob::display_kind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobDisplay {
    /// Rendered inline, line by line (gitweb's `text/*` branch, and any content
    /// without a NUL — the default for source).
    Text,
    /// Shown inline as an `<img>` (gitweb's `image/*` branch).
    Image,
    /// Not inlined; only a raw download is offered (gitweb falls back to
    /// `blob_plain`).
    Binary,
}

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

    /// How `git_blob` would present this blob, given its `file_name`.
    ///
    /// Mirroring gitweb's built-in (no `/etc/mime.types`) path: content without a
    /// NUL is text and is shown inline whatever its name; binary content is shown
    /// inline as an image when its name carries an image extension, and is
    /// otherwise download-only.
    #[must_use]
    pub fn display_kind(&self, file_name: Option<&str>) -> BlobDisplay {
        if !self.is_binary() {
            return BlobDisplay::Text;
        }
        match file_name {
            Some(name) if has_image_extension(name) => BlobDisplay::Image,
            _ => BlobDisplay::Binary,
        }
    }
}

/// Whether `file_name` carries an extension gitweb's built-in `blob_mimetype`
/// fallback maps to an inline image: `.png`, `.gif`, `.jpg`, or `.jpeg`,
/// case-insensitive (gitweb's `m/\.png$/i`, `m/\.gif$/i`, `m/\.jpe?g$/i`).
fn has_image_extension(file_name: &str) -> bool {
    let extension: Option<String> = file_name
        .rsplit_once('.')
        .map(|(_, ext): (&str, &str)| ext.to_ascii_lowercase());
    matches!(extension.as_deref(), Some("png" | "gif" | "jpg" | "jpeg"))
}
