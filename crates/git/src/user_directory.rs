//! Resolving the operating-system owner of a repository directory to a display
//! name — gitweb's `get_file_owner` (stat the directory, `getpwuid` its owning
//! uid, take the GECOS comment field). This is the last-resort owner source,
//! consulted only when neither the projects-list file nor the `gitweb.owner`
//! config names an owner.
//!
//! The uid → name step lives behind the [`UserDirectory`] seam so a conformance
//! spec can pin a name without depending on the host's passwd database (the real
//! mapping is non-deterministic across machines). The production
//! [`SystemUserDirectory`] reads `/etc/passwd`, which keeps the adapter free of
//! the `unsafe` FFI `getpwuid` would need and of a needless dependency.

use std::fs;

/// Maps a numeric user id to the owner display name gitweb would show — already
/// reduced to the GECOS comment's first segment. `None` when the host has no
/// passwd entry for the uid (gitweb's `!defined $gcos` → `undef`).
///
/// This is an adapter-internal seam, not a domain port: only the gix
/// [`ProjectStore`](crate::GixProjectStore) consumes it, to make the filesystem
/// owner fallback pinnable in conformance tests.
pub trait UserDirectory: std::fmt::Debug + Send + Sync {
    /// The display name of the user owning `uid`, or `None` when no passwd entry
    /// names it.
    fn display_name(&self, uid: u32) -> Option<String>;
}

/// The production [`UserDirectory`]: resolves a uid against the system passwd
/// database (`/etc/passwd`).
///
/// NSS sources other than the local file (LDAP, NIS, …) are out of reach without
/// the `unsafe` FFI of `getpwuid`; deployments that need them can inject their
/// own [`UserDirectory`] into the store. An unreadable or absent `/etc/passwd`
/// simply yields no owner, the way gitweb's `getpwuid` miss returns `undef`.
#[derive(Debug, Default)]
pub struct SystemUserDirectory;

impl UserDirectory for SystemUserDirectory {
    fn display_name(&self, uid: u32) -> Option<String> {
        let passwd: String = fs::read_to_string("/etc/passwd").ok()?;
        passwd
            .lines()
            .find_map(|line: &str| gecos_for_uid(line, uid))
            .map(owner_from_gecos)
    }
}

/// The GECOS comment field of one `/etc/passwd` line when it names `uid`, else
/// `None`. A passwd line is `name:passwd:uid:gid:gecos:dir:shell`; the uid is
/// field index 2 and the GECOS comment is field index 4. A line that is short or
/// whose uid field is not numeric simply does not match.
fn gecos_for_uid(line: &str, uid: u32) -> Option<&str> {
    let mut fields: std::str::Split<'_, char> = line.split(':');
    let line_uid: u32 = fields.nth(2)?.parse().ok()?;
    if line_uid != uid {
        return None;
    }
    // The iterator now starts at field 3; the GECOS comment is one further on.
    fields.nth(1)
}

/// gitweb's `$owner =~ s/[,;].*$//`: the GECOS comment up to its first `,` or
/// `;`, dropping the room/phone/other sub-fields and leaving the user's name. An
/// empty GECOS yields an empty owner, exactly as gitweb's `to_utf8("")` does.
fn owner_from_gecos(gecos: &str) -> String {
    gecos.split([',', ';']).next().unwrap_or("").to_owned()
}

#[cfg(test)]
mod tests {
    use super::{gecos_for_uid, owner_from_gecos};

    // owner_from_gecos: zero / one / many sub-field separators.

    #[test]
    fn gecos_without_separators_is_the_whole_comment() {
        assert_eq!(owner_from_gecos("Ada Lovelace"), "Ada Lovelace");
    }

    #[test]
    fn gecos_keeps_only_the_name_before_a_single_comma() {
        assert_eq!(owner_from_gecos("Ada Lovelace,Room 1,,"), "Ada Lovelace");
    }

    #[test]
    fn gecos_stops_at_a_semicolon_too() {
        assert_eq!(owner_from_gecos("Ada Lovelace;extra"), "Ada Lovelace");
    }

    #[test]
    fn gecos_stops_at_the_first_of_several_separators() {
        assert_eq!(owner_from_gecos("Ada,Lovelace;Babbage"), "Ada");
    }

    #[test]
    fn an_empty_gecos_yields_an_empty_owner() {
        assert_eq!(owner_from_gecos(""), "");
    }

    // gecos_for_uid: matching, non-matching, malformed.

    #[test]
    fn a_matching_uid_yields_its_gecos_field() {
        assert_eq!(
            gecos_for_uid("ada:x:1000:1000:Ada Lovelace:/home/ada:/bin/sh", 1000),
            Some("Ada Lovelace"),
        );
    }

    #[test]
    fn a_non_matching_uid_does_not_match() {
        assert_eq!(
            gecos_for_uid("ada:x:1000:1000:Ada Lovelace:/home/ada:/bin/sh", 42),
            None,
        );
    }

    #[test]
    fn a_line_with_no_numeric_uid_does_not_match() {
        assert_eq!(gecos_for_uid("# a comment, not an entry", 1000), None);
    }
}
