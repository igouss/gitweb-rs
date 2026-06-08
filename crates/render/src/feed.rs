//! RSS and Atom serialization — gitweb's `git_feed` body.
//!
//! Feeds are *format-stable* (verified by golden differential conformance, not
//! just behaviourally), so this reproduces gitweb's XML to the byte: the channel
//! / feed header, then one item / entry per commit, each carrying the message as
//! a `<pre>` block and the changed files as a `<ul>` of `blobdiff` / `blame` /
//! `history` links.
//!
//! The [`FeedView`] view-model arrives with every link already built (absolute,
//! gitweb-form — the web boundary's [`href_full`] output) and every text field
//! raw; this layer applies gitweb's per-site escaping (`esc_html` for text and
//! most attributes, `esc_url` for the standalone URL elements, `esc_path` for the
//! listed file name) exactly where `git_feed` applies it — including the spots
//! gitweb leaves a URL unescaped (RSS's `<guid>`, Atom's `<link rel="self">`).
//!
//! `href_full` (the web crate's URL builder) produces those links; here we escape.

use gitweb_domain::model::timestamp::Timestamp;

use crate::escape::{esc_html, esc_path, esc_url};

/// One changed file in a feed entry: the file it names and the already-built
/// links the entry lists for it (`blame` / `history` are present only when the
/// feature and the feed shape call for them, gitweb's per-link conditions).
#[derive(Debug, Clone)]
pub struct FeedFileView {
    /// The to-side path the entry lists (gitweb's `$file`).
    pub to_path: String,
    /// The `blobdiff` link (always present), as raw `href_full` output.
    pub blobdiff_url: String,
    /// The `blame` link, when the `blame` feature is on.
    pub blame_url: Option<String>,
    /// The `history` link, omitted only for the very file a file-history feed is
    /// about (gitweb's `!defined $file_name || $file_name ne $file`).
    pub history_url: Option<String>,
}

/// One feed entry: a commit and the files it changed.
#[derive(Debug, Clone)]
pub struct FeedEntryView {
    /// The commit subject (RSS `<title>`/`<description>`, Atom `<title>`).
    pub title: String,
    /// The whole author ident `name <email>` — RSS's `<author>`.
    pub author_full: String,
    /// The author's display name — Atom's `<author><name>`.
    pub author_name: String,
    /// The author's email, if any — Atom's `<author><email>`.
    pub author_email: Option<String>,
    /// The committer's display name — Atom's `<contributor><name>`.
    pub committer_name: String,
    /// The committer's email, if any — Atom's `<contributor><email>`.
    pub committer_email: Option<String>,
    /// The committer time — RSS `<pubDate>` / Atom `<updated>`,`<published>`.
    pub timestamp: Timestamp,
    /// The `commitdiff` link for this commit (raw `href_full` output).
    pub commitdiff_url: String,
    /// The message body lines for the `<pre>` block (raw).
    pub comment: Vec<String>,
    /// The changed files, in diff order.
    pub files: Vec<FeedFileView>,
}

/// The whole feed: channel/feed metadata plus the entries.
#[derive(Debug, Clone)]
pub struct FeedView {
    /// The feed title (raw) — gitweb's `$title`.
    pub title: String,
    /// The feed description (raw) — RSS `<description>` / Atom `<subtitle>`.
    pub description: String,
    /// The project owner (raw) — RSS `<managingEditor>` / Atom feed author.
    pub owner: String,
    /// The generator version composite (`$version/$git_version`), emitted raw.
    pub generator: String,
    /// The site logo URL (`""` when unset) — RSS `<image>` / Atom `<logo>`.
    pub logo: String,
    /// The site favicon URL (`""` when unset) — Atom `<icon>` / RSS image fallback.
    pub favicon: String,
    /// The HTML alternate URL (raw `href_full`) — the `summary`/`log`/`history`
    /// page this feed mirrors.
    pub alt_url: String,
    /// The feed's own id URL (raw `href_full` of the project) — Atom `<id>`.
    pub id_url: String,
    /// The request's own URL (raw) — Atom `<link rel="self">`.
    pub self_url: String,
    /// The base URL for relative links in entry content (raw) — Atom `xml:base`.
    pub xml_base: String,
    /// The feed's own media type (no charset) — Atom self-link `type`.
    pub content_type: String,
    /// The newest commit's timestamp, or `None` for an empty feed.
    pub latest: Option<Timestamp>,
    /// The entries, newest first.
    pub entries: Vec<FeedEntryView>,
}

/// Atom's dummy `<updated>` for a feed with no commits (gitweb keeps the feed
/// valid "until commits trickle in").
const ATOM_EPOCH: &str = "1970-01-01T00:00:00Z";

/// Serializes the feed as RSS 2.0 — gitweb's `git_feed('rss')` body.
#[must_use]
pub fn rss(view: &FeedView) -> String {
    let mut out: String = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.push_str(
        "<rss version=\"2.0\" xmlns:content=\"http://purl.org/rss/1.0/modules/content/\">\n\
         <channel>\n",
    );
    line(
        &mut out,
        &format!("<title>{}</title>", esc_html(&view.title)),
    );
    line(
        &mut out,
        &format!("<link>{}</link>", esc_html(&view.alt_url)),
    );
    line(
        &mut out,
        &format!("<description>{}</description>", esc_html(&view.description)),
    );
    line(&mut out, "<language>en</language>");
    line(
        &mut out,
        &format!("<managingEditor>{}</managingEditor>", esc_html(&view.owner)),
    );
    // gitweb prefers the logo to the favicon for the single RSS <image>.
    let image: &str = if view.logo.is_empty() {
        &view.favicon
    } else {
        &view.logo
    };
    if !image.is_empty() {
        line(&mut out, "<image>");
        line(&mut out, &format!("<url>{}</url>", esc_url(image)));
        line(
            &mut out,
            &format!("<title>{}</title>", esc_html(&view.title)),
        );
        line(
            &mut out,
            &format!("<link>{}</link>", esc_html(&view.alt_url)),
        );
        line(&mut out, "</image>");
    }
    if let Some(latest) = &view.latest {
        line(
            &mut out,
            &format!("<pubDate>{}</pubDate>", latest.rfc2822()),
        );
        line(
            &mut out,
            &format!("<lastBuildDate>{}</lastBuildDate>", latest.rfc2822()),
        );
    }
    line(
        &mut out,
        &format!("<generator>gitweb v.{}</generator>", view.generator),
    );
    for entry in &view.entries {
        out.push_str(&rss_item(entry));
    }
    out.push_str("</channel>\n</rss>\n");
    out
}

/// One RSS `<item>`.
fn rss_item(entry: &FeedEntryView) -> String {
    let mut out: String = String::new();
    line(&mut out, "<item>");
    line(
        &mut out,
        &format!("<title>{}</title>", esc_html(&entry.title)),
    );
    line(
        &mut out,
        &format!("<author>{}</author>", esc_html(&entry.author_full)),
    );
    line(
        &mut out,
        &format!("<pubDate>{}</pubDate>", entry.timestamp.rfc2822()),
    );
    // gitweb emits the guid and the link without escaping the URL itself.
    line(
        &mut out,
        &format!("<guid isPermaLink=\"true\">{}</guid>", entry.commitdiff_url),
    );
    line(
        &mut out,
        &format!("<link>{}</link>", esc_html(&entry.commitdiff_url)),
    );
    line(
        &mut out,
        &format!("<description>{}</description>", esc_html(&entry.title)),
    );
    out.push_str("<content:encoded><![CDATA[\n");
    out.push_str(&entry_body(entry));
    out.push_str("</ul>]]>\n");
    out.push_str("</content:encoded>\n");
    out.push_str("</item>\n");
    out
}

/// Serializes the feed as Atom 1.0 — gitweb's `git_feed('atom')` body.
#[must_use]
pub fn atom(view: &FeedView) -> String {
    let mut out: String = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n");
    out.push_str("<feed xmlns=\"http://www.w3.org/2005/Atom\">\n");
    line(
        &mut out,
        &format!("<title>{}</title>", esc_html(&view.title)),
    );
    line(
        &mut out,
        &format!("<subtitle>{}</subtitle>", esc_html(&view.description)),
    );
    line(
        &mut out,
        &format!(
            "<link rel=\"alternate\" type=\"text/html\" href=\"{}\" />",
            esc_html(&view.alt_url)
        ),
    );
    // gitweb inserts self_url() raw.
    line(
        &mut out,
        &format!(
            "<link rel=\"self\" type=\"{}\" href=\"{}\" />",
            view.content_type, view.self_url
        ),
    );
    line(&mut out, &format!("<id>{}</id>", esc_url(&view.id_url)));
    line(
        &mut out,
        &format!("<author><name>{}</name></author>", esc_html(&view.owner)),
    );
    if !view.favicon.is_empty() {
        line(
            &mut out,
            &format!("<icon>{}</icon>", esc_url(&view.favicon)),
        );
    }
    if !view.logo.is_empty() {
        line(&mut out, &format!("<logo>{}</logo>", esc_url(&view.logo)));
    }
    let updated: String = view
        .latest
        .as_ref()
        .map_or_else(|| ATOM_EPOCH.to_owned(), Timestamp::iso8601);
    line(&mut out, &format!("<updated>{updated}</updated>"));
    line(
        &mut out,
        &format!("<generator version='{}'>gitweb</generator>", view.generator),
    );
    for entry in &view.entries {
        out.push_str(&atom_entry(entry, &view.xml_base));
    }
    out.push_str("</feed>\n");
    out
}

/// One Atom `<entry>`. `xml_base` is the feed-level base for the entry content's
/// `xml:base` (gitweb's `$my_url`).
fn atom_entry(entry: &FeedEntryView, xml_base: &str) -> String {
    let mut out: String = String::new();
    out.push_str("<entry>\n");
    line(
        &mut out,
        &format!("<title type=\"html\">{}</title>", esc_html(&entry.title)),
    );
    line(
        &mut out,
        &format!("<updated>{}</updated>", entry.timestamp.iso8601()),
    );
    out.push_str("<author>\n");
    line(
        &mut out,
        &format!("  <name>{}</name>", esc_html(&entry.author_name)),
    );
    if let Some(email) = &entry.author_email {
        line(&mut out, &format!("  <email>{}</email>", esc_html(email)));
    }
    out.push_str("</author>\n");
    out.push_str("<contributor>\n");
    line(
        &mut out,
        &format!("  <name>{}</name>", esc_html(&entry.committer_name)),
    );
    if let Some(email) = &entry.committer_email {
        line(&mut out, &format!("  <email>{}</email>", esc_html(email)));
    }
    out.push_str("</contributor>\n");
    line(
        &mut out,
        &format!("<published>{}</published>", entry.timestamp.iso8601()),
    );
    line(
        &mut out,
        &format!(
            "<link rel=\"alternate\" type=\"text/html\" href=\"{}\" />",
            esc_html(&entry.commitdiff_url)
        ),
    );
    line(
        &mut out,
        &format!("<id>{}</id>", esc_html(&entry.commitdiff_url)),
    );
    line(
        &mut out,
        &format!(
            "<content type=\"xhtml\" xml:base=\"{}\">",
            esc_url(xml_base)
        ),
    );
    out.push_str("<div xmlns=\"http://www.w3.org/1999/xhtml\">\n");
    out.push_str(&entry_body(entry));
    out.push_str("</ul>\n</div>\n");
    out.push_str("</content>\n");
    out.push_str("</entry>\n");
    out
}

/// The shared entry body: the message as a `<pre>` block, then the changed files
/// opened as a `<ul>` — up to but not including the closing `</ul>`, which the
/// RSS and Atom paths close differently. Mirrors gitweb's shared loop.
fn entry_body(entry: &FeedEntryView) -> String {
    let mut out: String = String::from("<pre>\n");
    for line_text in &entry.comment {
        out.push_str(&esc_html(line_text));
        out.push('\n');
    }
    out.push_str("</pre><ul>\n");
    for file in &entry.files {
        out.push_str(&file_item(file));
    }
    out
}

/// One changed-file `<li>`: the `blobdiff` `D` link, the optional `blame` `B`
/// and `history` `H` links, then the file name. gitweb HTML-escapes each link's
/// `href`; the file name is `esc_path`-escaped.
fn file_item(file: &FeedFileView) -> String {
    let mut out: String = String::from("<li>[");
    out.push_str(&anchor(&file.blobdiff_url, "diff", "D"));
    if let Some(url) = &file.blame_url {
        out.push_str(&anchor(url, "blame", "B"));
    }
    if let Some(url) = &file.history_url {
        out.push_str(&anchor(url, "history", "H"));
    }
    out.push_str("] ");
    out.push_str(&esc_path(&file.to_path));
    out.push_str("</li>\n");
    out
}

/// gitweb's `$cgi->a({-href => …, -title => …}, $text)`: an anchor whose href is
/// HTML-escaped, with a fixed title and link text.
fn anchor(url: &str, title: &str, text: &str) -> String {
    format!("<a href=\"{}\" title=\"{title}\">{text}</a>", esc_html(url))
}

/// Appends `text` and a newline to `out` (gitweb's per-line `print "...\n"`).
fn line(out: &mut String, text: &str) {
    out.push_str(text);
    out.push('\n');
}
