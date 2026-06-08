//! Git identity lines (signatures).
//!
//! Mirrors the ident parsing in gitweb's `parse_commit_text` / `parse_tag`. An
//! ident line is `"<name> <<email>> <epoch> <tz>"`; the caller has already
//! stripped the leading role keyword (`author` / `committer` / `tagger`) and
//! decoded the bytes (see [`crate::model::encoding`]).

/// A parsed git identity: who, optionally their email, and when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    name: String,
    email: Option<String>,
    epoch: i64,
    timezone: String,
}

impl Signature {
    /// Parses an ident line, returning `None` when no trailing
    /// `"<epoch> <tz>"` is present (gitweb's author/committer regex would
    /// simply fail to match such a line).
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        let (ident, epoch, timezone): (&str, i64, String) = split_ident_time(line)?;
        let (name, email): (String, Option<String>) = split_name_email(ident);
        Some(Self {
            name,
            email,
            epoch,
            timezone,
        })
    }

    /// The author/committer display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The email address between the angle brackets, if any. An explicitly
    /// empty `<>` parses to `Some("")`, distinct from an absent email.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// The whole ident in gitweb's `$co{'author'}` form — `"name <email>"`, or
    /// just the name when no email is present. The mailbox header of
    /// `commitdiff_plain` and the feeds' `<author>` both print this.
    #[must_use]
    pub fn full_ident(&self) -> String {
        match &self.email {
            Some(email) => format!("{} <{email}>", self.name),
            None => self.name.clone(),
        }
    }

    /// The author/committer time as a Unix epoch.
    #[must_use]
    pub fn epoch(&self) -> i64 {
        self.epoch
    }

    /// The raw timezone offset token (e.g. `"+0200"`).
    #[must_use]
    pub fn timezone(&self) -> &str {
        &self.timezone
    }
}

/// Peels the trailing `" <epoch> <tz>"` off an ident line.
///
/// gitweb matches `^(.*) ([0-9]+) (.*)$` with a greedy leading group, so the
/// epoch is the *rightmost* digit run that is both preceded and followed by a
/// space — this is what lets a name like "Agent 007" survive intact.
fn split_ident_time(line: &str) -> Option<(&str, i64, String)> {
    let bytes: &[u8] = line.as_bytes();
    let mut split: Option<(usize, usize)> = None;
    let mut i: usize = 0;
    while i < bytes.len() {
        if bytes[i] == b' ' {
            let mut j: usize = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let has_digits: bool = j > i + 1;
            let trailing_space: bool = j < bytes.len() && bytes[j] == b' ';
            if has_digits && trailing_space {
                split = Some((i, j)); // keep the rightmost match
            }
        }
        i += 1;
    }
    let (space_before_epoch, space_after_epoch): (usize, usize) = split?;
    let ident: &str = &line[..space_before_epoch];
    let epoch: i64 = line[space_before_epoch + 1..space_after_epoch]
        .parse()
        .ok()?;
    let timezone: String = line[space_after_epoch + 1..].to_owned();
    Some((ident, epoch, timezone))
}

/// Splits an ident into name and optional email.
///
/// gitweb matches `^([^<]+) <([^>]*)>`: the email lives between the first
/// `<` (which must be preceded by a space and at least one name character)
/// and the next `>`. Without that shape the whole ident is the name.
fn split_name_email(ident: &str) -> (String, Option<String>) {
    let bytes: &[u8] = ident.as_bytes();
    let open: usize = match ident.find('<') {
        Some(index) => index,
        None => return (ident.to_owned(), None),
    };
    let has_space_before: bool = open >= 2 && bytes[open - 1] == b' ';
    let close_offset: Option<usize> = ident[open + 1..].find('>');
    match (has_space_before, close_offset) {
        (true, Some(offset)) => {
            let name: &str = &ident[..open - 1];
            let email: &str = &ident[open + 1..open + 1 + offset];
            (name.to_owned(), Some(email.to_owned()))
        }
        _ => (ident.to_owned(), None),
    }
}
