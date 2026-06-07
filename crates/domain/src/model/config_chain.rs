//! Global gitweb config source selection: the pure half of
//! `evaluate_gitweb_config`.
//!
//! gitweb loads up to three global config files. Each slot resolves from its
//! environment override or a compile-time default (`$ENV{X} || "@DEFAULT@"`).
//! `GITWEB_CONFIG_COMMON` is always applied first; then the first of
//! `GITWEB_CONFIG` / `GITWEB_CONFIG_SYSTEM` whose file exists is applied —
//! `GITWEB_CONFIG` wins, and `GITWEB_CONFIG_SYSTEM` is consulted only when the
//! per-instance file is absent. Both are de-duplicated against the common path
//! so the same file is never read twice; per gitweb, blanking a duplicate
//! per-instance path lets the system config load in its place.
//!
//! Reading and executing the files (and applying a per-project layer on top of
//! the result) is the web boundary's job. This rule only decides which files to
//! load and in what order — weakest first, so a later file overrides an earlier
//! one — given which candidate paths exist on disk.

/// One configuration-file slot: gitweb's `$ENV{X} || "@DEFAULT@"`. A non-empty
/// environment override wins; otherwise the compile-time default; an empty
/// result means the slot names no file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigSlot {
    env: Option<String>,
    default: Option<String>,
}

impl ConfigSlot {
    /// A slot from its environment override and compile-time default. An empty
    /// string is treated as unset, mirroring Perl's falsy `""`.
    #[must_use]
    pub fn new(env: Option<String>, default: Option<String>) -> Self {
        Self { env, default }
    }

    /// The resolved path, or `None` when neither source supplies a non-empty
    /// one (`$ENV{X} || "@DEFAULT@"`, where `""` is falsy).
    fn resolve(&self) -> Option<&str> {
        non_empty(self.env.as_deref()).or_else(|| non_empty(self.default.as_deref()))
    }
}

/// gitweb's three global config slots, in precedence order. `COMMON` is always
/// applied first; then `GITWEB_CONFIG` (`primary`) or, only when its file is
/// absent, `GITWEB_CONFIG_SYSTEM` (`system`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigChain {
    common: ConfigSlot,
    primary: ConfigSlot,
    system: ConfigSlot,
}

impl ConfigChain {
    /// Builds the chain from its three resolved slots.
    #[must_use]
    pub fn new(common: ConfigSlot, primary: ConfigSlot, system: ConfigSlot) -> Self {
        Self {
            common,
            primary,
            system,
        }
    }

    /// The ordered list of config files to load, weakest first, given the
    /// `exists` predicate over candidate paths. Mirrors `evaluate_gitweb_config`.
    #[must_use]
    pub fn load_order(&self, exists: impl Fn(&str) -> bool) -> Vec<String> {
        let common: Option<&str> = self.common.resolve();
        // De-duplicate the per-instance and system paths against the common path
        // so the same file is never read twice (gitweb blanks the duplicate).
        let primary: Option<&str> = self.primary.resolve().filter(|p: &&str| Some(*p) != common);
        let system: Option<&str> = self.system.resolve().filter(|s: &&str| Some(*s) != common);

        let mut order: Vec<String> = Vec::new();
        // The common file is always applied first when it exists.
        if let Some(path) = common.filter(|p: &&str| exists(p)) {
            order.push(path.to_owned());
        }
        // Then the first existing of per-instance / system — the per-instance
        // file wins, and the system file fills in only when it is absent.
        if let Some(path) = primary.filter(|p: &&str| exists(p)) {
            order.push(path.to_owned());
        } else if let Some(path) = system.filter(|s: &&str| exists(s)) {
            order.push(path.to_owned());
        }
        order
    }
}

/// `Some` for a present, non-empty string; `None` for absent or empty.
fn non_empty(value: Option<&str>) -> Option<&str> {
    match value {
        Some(text) if !text.is_empty() => Some(text),
        _ => None,
    }
}
