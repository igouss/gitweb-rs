//! Git file modes: type classification and symbolic permission strings.
//!
//! Mirrors gitweb's `mode_str`, `file_type`, and `file_type_long`. A git tree
//! entry carries an octal mode (e.g. `100644`); from it we derive the entry's
//! kind and the `ls -l`-style permission string shown in the tree view.

// Standard `st_mode` type bits (POSIX `S_IF*`), plus git's gitlink type.
// gitweb pulls these from Fcntl `:mode`; we name them inline to keep the
// domain dependency-free.
const S_IFMT: u32 = 0o170000; // type-field mask
const S_IFGITLINK: u32 = 0o160000; // submodule / commit reference
const S_IFLNK: u32 = 0o120000; // symbolic link
const S_IFREG: u32 = 0o100000; // regular file
const S_IFDIR: u32 = 0o040000; // directory
const S_IXUSR: u32 = 0o000100; // owner-execute bit

/// What a git tree entry is, derived from its mode bits.
///
/// git records only the executable bit for regular files, so this is the
/// finest distinction the mode actually carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// Submodule / gitlink (mode `160000`).
    Submodule,
    /// Directory / tree (mode `040000`).
    Directory,
    /// Symbolic link (mode `120000`).
    Symlink,
    /// Regular file with the owner-execute bit set.
    Executable,
    /// Regular file without the execute bit.
    Regular,
    /// Anything else (e.g. gitweb's `S_IFINVALID`).
    Unknown,
}

/// A git tree-entry mode, parsed from its octal text form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileMode {
    bits: u32,
}

impl FileMode {
    /// Parses an octal mode string such as `"100644"`.
    ///
    /// Returns `None` when the text is not a non-empty run of octal digits,
    /// mirroring gitweb's `^[0-7]+$` guard (the render boundary echoes such
    /// input verbatim rather than classifying it).
    #[must_use]
    pub fn from_octal(text: &str) -> Option<Self> {
        if text.is_empty()
            || !text
                .bytes()
                .all(|byte: u8| byte.is_ascii_digit() && byte <= b'7')
        {
            return None;
        }
        u32::from_str_radix(text, 8)
            .ok()
            .map(|bits: u32| Self { bits })
    }

    /// Classifies the mode into a [`FileKind`].
    ///
    /// gitlink is checked before the standard type bits because its value
    /// (`160000`) overlaps `S_IFLNK | S_IFDIR` and would otherwise misclassify.
    #[must_use]
    pub fn kind(self) -> FileKind {
        let type_bits: u32 = self.bits & S_IFMT;
        if type_bits == S_IFGITLINK {
            FileKind::Submodule
        } else if type_bits == S_IFDIR {
            FileKind::Directory
        } else if type_bits == S_IFLNK {
            FileKind::Symlink
        } else if type_bits == S_IFREG {
            if self.bits & S_IXUSR != 0 {
                FileKind::Executable
            } else {
                FileKind::Regular
            }
        } else {
            FileKind::Unknown
        }
    }

    /// gitweb's `file_type`: short type label. Executable and plain regular
    /// files both read as `"file"`.
    #[must_use]
    pub fn short_type(self) -> &'static str {
        match self.kind() {
            FileKind::Submodule => "submodule",
            FileKind::Directory => "directory",
            FileKind::Symlink => "symlink",
            FileKind::Executable | FileKind::Regular => "file",
            FileKind::Unknown => "unknown",
        }
    }

    /// gitweb's `file_type_long`: long type label, distinguishing
    /// `"executable"` from `"file"`.
    #[must_use]
    pub fn long_type(self) -> &'static str {
        match self.kind() {
            FileKind::Submodule => "submodule",
            FileKind::Directory => "directory",
            FileKind::Symlink => "symlink",
            FileKind::Executable => "executable",
            FileKind::Regular => "file",
            FileKind::Unknown => "unknown",
        }
    }

    /// gitweb's `mode_str`: the 10-character symbolic permission string.
    ///
    /// git only honours the executable bit, so regular files render as one of
    /// two fixed strings rather than reflecting the full Unix permission bits.
    #[must_use]
    pub fn permission_string(self) -> &'static str {
        match self.kind() {
            FileKind::Submodule => "m---------",
            FileKind::Directory => "drwxr-xr-x",
            FileKind::Symlink => "lrwxrwxrwx",
            FileKind::Executable => "-rwxr-xr-x",
            FileKind::Regular => "-rw-r--r--",
            FileKind::Unknown => "----------",
        }
    }
}
