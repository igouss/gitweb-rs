//! How gitweb's `git_blob_plain` serves a raw blob.
//!
//! `git_blob_plain` streams a blob's bytes verbatim under two headers it derives
//! from the content and the file name: a Content-Type (gitweb's
//! `blob_contenttype` over the built-in `blob_mimetype` fallback) and a
//! Content-Disposition (its "save as" name plus the XSS sandbox decision). This
//! module owns those pure rules; the use case reads the blob and the bytes are
//! the caller's to stream.

use crate::model::blob::image_media_type;

/// The media type a blob is served as — gitweb's built-in `blob_mimetype`
/// fallback (no `/etc/mime.types`): text content is `text/plain`; binary content
/// is `image/png`, `image/gif` or `image/jpeg` when the name carries that
/// extension, and `application/octet-stream` otherwise, including when no name is
/// given. Content wins over name: text is `text/plain` whatever it is called.
#[must_use]
fn media_type(file_name: Option<&str>, is_binary: bool) -> &'static str {
    if !is_binary {
        return "text/plain";
    }
    file_name
        .and_then(image_media_type)
        .unwrap_or("application/octet-stream")
}

/// The Content-Type and Content-Disposition gitweb's `git_blob_plain` sends with
/// a raw blob's verbatim bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlainHeaders {
    content_type: String,
    content_disposition: String,
}

impl PlainHeaders {
    /// Derives the headers from the blob's content (`is_binary`), its `file_name`
    /// (gitweb's `$file_name`), the resolved `blob_id` (the "save as" default,
    /// gitweb's `$hash`), the site's `charset` (gitweb's
    /// `$default_text_plain_charset`, appended only to `text/plain`), and whether
    /// XSS prevention is on (gitweb's `$prevent_xss`).
    #[must_use]
    pub fn resolve(
        file_name: Option<&str>,
        is_binary: bool,
        blob_id: &str,
        charset: Option<&str>,
        prevent_xss: bool,
    ) -> Self {
        let media: &'static str = media_type(file_name, is_binary);
        let content_type: String = with_charset(media, charset);
        let save_as: String = save_as(file_name, blob_id, media);
        let disposition: &str = if prevent_xss && !is_inline_safe(media) {
            "attachment"
        } else {
            "inline"
        };
        Self {
            content_type,
            content_disposition: format!(r#"{disposition}; filename="{save_as}""#),
        }
    }

    /// The `Content-Type` value (e.g. `text/plain`, `image/png`).
    #[must_use]
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// The `Content-Disposition` value (e.g. `inline; filename="x.txt"`).
    #[must_use]
    pub fn content_disposition(&self) -> &str {
        &self.content_disposition
    }
}

/// gitweb's `blob_contenttype` charset rule: a charset is appended only to
/// `text/plain`, and only when the site configures one (`$default_text_plain_charset`).
fn with_charset(media_type: &str, charset: Option<&str>) -> String {
    match charset {
        Some(charset) if media_type == "text/plain" => format!("{media_type}; charset={charset}"),
        _ => media_type.to_owned(),
    }
}

/// gitweb's "save as" name: the file name when given; otherwise the blob id,
/// with a `.txt` suffix for a text type (gitweb's `$save_as .= '.txt'`).
fn save_as(file_name: Option<&str>, blob_id: &str, media_type: &str) -> String {
    match file_name {
        Some(name) => name.to_owned(),
        None if media_type.starts_with("text/") => format!("{blob_id}.txt"),
        None => blob_id.to_owned(),
    }
}

/// gitweb's XSS-sandbox safe-type test: a type served inline even under
/// `$prevent_xss` is `text/<lowercase-alpha>` or one of the inline image types
/// `image/gif`, `image/png`, `image/jpeg` (gitweb's
/// `m!^(?:text/[a-z]+|image/(?:gif|png|jpeg))(?:[ ;]|$)!`).
fn is_inline_safe(media_type: &str) -> bool {
    matches!(media_type, "image/gif" | "image/png" | "image/jpeg")
        || media_type.strip_prefix("text/").is_some_and(|sub: &str| {
            !sub.is_empty() && sub.bytes().all(|b: u8| b.is_ascii_lowercase())
        })
}
