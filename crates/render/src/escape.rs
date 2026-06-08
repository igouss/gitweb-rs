//! HTML and URL escaping, mirroring gitweb's `esc_*` family.
//!
//! All of these take an already-decoded `&str`. Turning raw git bytes into a
//! `String` (gitweb's `to_utf8`) is the adapter's job and happens before a
//! view-model ever reaches the render layer; here we only own the
//! HTML/URL-safety rules.

/// Escapes text for safe inclusion in HTML, in either text or attribute
/// context (gitweb `esc_html`). The five HTML metacharacters become entities
/// and control characters — other than tab, which stays literal — are made
/// visible via [`quot_cec`].
#[must_use]
pub fn esc_html(text: &str) -> String {
    escape_html(text, Whitespace::Collapse, TabHandling::Literal)
}

/// Like [`esc_html`], but every space becomes a non-breaking space so that
/// significant whitespace (e.g. in a diff line) survives HTML collapsing
/// (gitweb `esc_html(..., -nbsp => 1)`).
#[must_use]
pub fn esc_html_nbsp(text: &str) -> String {
    escape_html(text, Whitespace::NonBreaking, TabHandling::Literal)
}

/// Escapes a filesystem path for HTML (gitweb `esc_path`). Identical to
/// [`esc_html`] except that tab, too, is rendered as a visible control escape.
#[must_use]
pub fn esc_path(text: &str) -> String {
    escape_html(text, Whitespace::Collapse, TabHandling::Visible)
}

/// Escapes text for an HTML attribute value (gitweb `esc_attr`, which is just
/// `esc_html`).
#[must_use]
pub fn esc_attr(text: &str) -> String {
    esc_html(text)
}

/// Percent-encodes a whole URL, keeping URL-structural characters literal and
/// turning spaces into `+` (gitweb `esc_url`).
#[must_use]
pub fn esc_url(text: &str) -> String {
    percent_encode(text, is_url_safe, SpaceEncoding::Plus)
}

/// Percent-encodes a single URL component / query parameter, turning spaces
/// into `+` (gitweb `esc_param`).
#[must_use]
pub fn esc_param(text: &str) -> String {
    percent_encode(text, is_param_safe, SpaceEncoding::Plus)
}

/// Percent-encodes a `PATH_INFO` fragment: spaces and `+` stay literal but `?`
/// is escaped (gitweb `esc_path_info`).
#[must_use]
pub fn esc_path_info(text: &str) -> String {
    percent_encode(text, is_path_info_safe, SpaceEncoding::Literal)
}

/// Quotes a `project_index` field — a project path or owner — gitweb's inline
/// `s/([^a-zA-Z0-9_.\-\/ ])/%XX/; s/ /+/`: percent-encode every byte outside the
/// keep-set with upper-case hex, *keeping the slash* so the project path stays
/// readable, then turn spaces into `+` (gitweb's `git_project_index`).
#[must_use]
pub fn esc_index_field(text: &str) -> String {
    percent_encode(text, is_index_safe, SpaceEncoding::Plus)
}

// ---- HTML escaping ----------------------------------------------------------

/// Whether spaces are kept as ordinary collapsing whitespace or pinned with
/// non-breaking spaces.
#[derive(Clone, Copy)]
enum Whitespace {
    Collapse,
    NonBreaking,
}

/// Whether a tab is left as a literal tab (HTML text) or made visible like any
/// other control character (filesystem paths).
#[derive(Clone, Copy)]
enum TabHandling {
    Literal,
    Visible,
}

fn escape_html(text: &str, whitespace: Whitespace, tab: TabHandling) -> String {
    let mut out: String = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            ' ' => out.push_str(space_for(whitespace)),
            '\t' if matches!(tab, TabHandling::Literal) => out.push('\t'),
            other if other.is_control() => out.push_str(&quot_cec(other)),
            other => out.push(other),
        }
    }
    out
}

fn space_for(whitespace: Whitespace) -> &'static str {
    match whitespace {
        Whitespace::Collapse => " ",
        Whitespace::NonBreaking => "&nbsp;",
    }
}

/// Renders a control character as a visible escape inside a `cntrl` span
/// (gitweb `quot_cec`). Recognised C0 controls use their conventional letter
/// escape; anything else falls back to space-padded lower-case hex, matching
/// gitweb's `sprintf('\%2x', ord)`.
fn quot_cec(ctrl: char) -> String {
    let escape: String = match ctrl {
        '\t' => "\\t".to_owned(),
        '\n' => "\\n".to_owned(),
        '\r' => "\\r".to_owned(),
        '\u{0c}' => "\\f".to_owned(),
        '\u{08}' => "\\b".to_owned(),
        '\u{07}' => "\\a".to_owned(),
        '\u{1b}' => "\\e".to_owned(),
        '\u{0b}' => "\\v".to_owned(),
        '\u{00}' => "\\0".to_owned(),
        other => format!("\\{:2x}", other as u32),
    };
    format!("<span class=\"cntrl\">{escape}</span>")
}

// ---- URL escaping -----------------------------------------------------------

/// Whether a space is rendered as `+` or kept literal.
#[derive(Clone, Copy)]
enum SpaceEncoding {
    Plus,
    Literal,
}

fn percent_encode(text: &str, is_safe: fn(char) -> bool, space: SpaceEncoding) -> String {
    let mut out: String = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            ' ' if matches!(space, SpaceEncoding::Plus) => out.push('+'),
            other if is_safe(other) => out.push(other),
            other => push_percent_encoded(&mut out, other),
        }
    }
    out
}

fn push_percent_encoded(out: &mut String, ch: char) {
    let mut buf: [u8; 4] = [0; 4];
    for &byte in ch.encode_utf8(&mut buf).as_bytes() {
        out.push_str(&format!("%{byte:02X}"));
    }
}

// NB: space is handled by the `SpaceEncoding` arm before these predicates are
// consulted, so it is deliberately absent here even though gitweb's keep-set
// lists it.

fn is_url_safe(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '-' | '_' | '.' | '~' | '(' | ')' | ';' | '/' | '?' | ':' | '@' | '&' | '='
        )
}

fn is_param_safe(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~' | '(' | ')' | '/' | ':' | '@')
}

fn is_path_info_safe(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            '-' | '_' | '.' | '~' | '(' | ')' | ';' | '/' | ':' | '@' | '&' | '=' | ' ' | '+'
        )
}

// gitweb's project_index keep-set is the bare `[a-zA-Z0-9_.\-\/ ]`: only the
// underscore, dot, hyphen and slash join the alphanumerics; the space is handled
// by `SpaceEncoding::Plus` before this predicate is consulted.
fn is_index_safe(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '-' | '/')
}
