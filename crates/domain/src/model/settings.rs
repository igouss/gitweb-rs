//! Resolved global gitweb settings and the value-precedence rule.
//!
//! [`config_chain`](crate::model::config_chain) decides WHICH global config
//! files load and in what order (weakest first). This module owns the VALUES:
//! [`Settings`], the effective configuration the rest of the app reads, and the
//! pure rule that resolves it across that ordered list of sources — built-in
//! defaults first, then each loaded file overlaid on top (and, later, the
//! per-project layer strongest of all).
//!
//! gitweb's config files are executable Perl that re-assign `our` variables and
//! entries of the `%feature` hash; because each later `do $file` re-runs those
//! assignments, the strongest source that sets a value wins. We do not run
//! Perl, so a source is a partial [`SettingsLayer`] and [`Settings::resolve`]
//! overlays them. Settings compose by three kinds, all covered by the rule:
//!   - a **scalar**'s value replaces (`$x = ...`);
//!   - a **list** replaces wholesale, never appends (`@x = (...)`);
//!   - a **feature**'s `default` and `override` flag overlay INDEPENDENTLY, so
//!     one source may flip a feature's override while another sets its default
//!     (gitweb writes `$feature{x}{default}` and `$feature{x}{override}` as
//!     separate assignments).
//!
//! Only the settings with real consumers are modelled here, plus the full
//! `%feature` set (its mechanism is the heart of gitweb config). gitweb's purely
//! cosmetic scalars (`$logo_url`, `$site_footer`, …) are deliberately left out
//! until a view needs one — adding such a field is a one-line scalar that this
//! rule already resolves.

use std::collections::BTreeMap;

/// A gitweb feature toggle: one entry of the `%feature` hash. `default` is the
/// site-wide value (an array of options — often a single `"0"`/`"1"`, or a list
/// such as the snapshot formats); `overridable` says whether a per-project
/// `gitweb.<key>` may override it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feature {
    default: Vec<String>,
    overridable: bool,
}

impl Feature {
    /// A feature from its default option list and overridability.
    #[must_use]
    pub fn new(default: Vec<String>, overridable: bool) -> Self {
        Self {
            default,
            overridable,
        }
    }

    /// The site-wide default options (gitweb's `$feature{x}{default}` array).
    #[must_use]
    pub fn default_options(&self) -> &[String] {
        &self.default
    }

    /// Whether a per-project `gitweb.<key>` may override this feature.
    #[must_use]
    pub fn is_overridable(&self) -> bool {
        self.overridable
    }

    /// Whether this feature is on, read as a boolean the way gitweb's
    /// `gitweb_check_feature` / `feature_bool` read it: the first default option
    /// in Perl-truthy terms — present, non-empty, and not `"0"`. A feature with no
    /// options at all (a list feature read in boolean context, e.g. `actions`) is
    /// off.
    #[must_use]
    pub fn enabled(&self) -> bool {
        match self.default.first() {
            Some(first) => !first.is_empty() && first != "0",
            None => false,
        }
    }
}

/// The names of gitweb's `%feature` entries. The string form (`as_key`) is the
/// exact gitweb key, so config files and per-project `gitweb.<key>` lookups map
/// straight onto it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeatureName {
    Blame,
    Snapshot,
    Search,
    Grep,
    Pickaxe,
    ShowSizes,
    Pathinfo,
    Forks,
    Actions,
    Ctags,
    Patches,
    Avatar,
    Timed,
    JavascriptActions,
    JavascriptTimezone,
    Highlight,
    RemoteHeads,
    ExtraBranchRefs,
    EmailPrivacy,
}

impl FeatureName {
    /// Every feature gitweb defines, in declaration order.
    pub const ALL: [FeatureName; 19] = [
        FeatureName::Blame,
        FeatureName::Snapshot,
        FeatureName::Search,
        FeatureName::Grep,
        FeatureName::Pickaxe,
        FeatureName::ShowSizes,
        FeatureName::Pathinfo,
        FeatureName::Forks,
        FeatureName::Actions,
        FeatureName::Ctags,
        FeatureName::Patches,
        FeatureName::Avatar,
        FeatureName::Timed,
        FeatureName::JavascriptActions,
        FeatureName::JavascriptTimezone,
        FeatureName::Highlight,
        FeatureName::RemoteHeads,
        FeatureName::ExtraBranchRefs,
        FeatureName::EmailPrivacy,
    ];

    /// The exact gitweb `%feature` key for this name.
    #[must_use]
    pub fn as_key(self) -> &'static str {
        match self {
            Self::Blame => "blame",
            Self::Snapshot => "snapshot",
            Self::Search => "search",
            Self::Grep => "grep",
            Self::Pickaxe => "pickaxe",
            Self::ShowSizes => "show-sizes",
            Self::Pathinfo => "pathinfo",
            Self::Forks => "forks",
            Self::Actions => "actions",
            Self::Ctags => "ctags",
            Self::Patches => "patches",
            Self::Avatar => "avatar",
            Self::Timed => "timed",
            Self::JavascriptActions => "javascript-actions",
            Self::JavascriptTimezone => "javascript-timezone",
            Self::Highlight => "highlight",
            Self::RemoteHeads => "remote_heads",
            Self::ExtraBranchRefs => "extra-branch-refs",
            Self::EmailPrivacy => "email-privacy",
        }
    }

    /// The feature whose key is `key`, or `None` for an unknown key.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|name: &Self| name.as_key() == key)
    }

    /// gitweb's per-feature override rule — which `feature_*` sub reads the
    /// repository's `gitweb.<key>` value when the feature is overridable. `None`
    /// is a feature gitweb does not make per-project overridable (it has no
    /// `sub`): search, pathinfo, forks, actions, ctags, timed, javascript-*.
    #[must_use]
    fn override_rule(self) -> Option<OverrideRule> {
        match self {
            Self::Blame
            | Self::Grep
            | Self::Pickaxe
            | Self::ShowSizes
            | Self::Highlight
            | Self::RemoteHeads => Some(OverrideRule::Bool),
            Self::Snapshot => Some(OverrideRule::SnapshotList),
            Self::Patches => Some(OverrideRule::Int),
            Self::Avatar => Some(OverrideRule::Value),
            Self::ExtraBranchRefs => Some(OverrideRule::WsList),
            _ => None,
        }
    }

    /// gitweb's built-in default for this feature (its `%feature` entry).
    #[must_use]
    fn builtin(self) -> Feature {
        match self {
            Self::Blame => Feature::new(opts(&["0"]), false),
            Self::Snapshot => Feature::new(opts(&["tgz"]), false),
            Self::Search => Feature::new(opts(&["1"]), false),
            Self::Grep => Feature::new(opts(&["1"]), false),
            Self::Pickaxe => Feature::new(opts(&["1"]), false),
            Self::ShowSizes => Feature::new(opts(&["1"]), false),
            Self::Pathinfo => Feature::new(opts(&["0"]), false),
            Self::Forks => Feature::new(opts(&["0"]), false),
            Self::Actions => Feature::new(opts(&[]), false),
            Self::Ctags => Feature::new(opts(&["0"]), false),
            Self::Patches => Feature::new(opts(&["16"]), false),
            Self::Avatar => Feature::new(opts(&[""]), false),
            Self::Timed => Feature::new(opts(&["0"]), false),
            Self::JavascriptActions => Feature::new(opts(&["0"]), false),
            Self::JavascriptTimezone => {
                Feature::new(opts(&["local", "gitweb_tz", "datetime"]), false)
            }
            Self::Highlight => Feature::new(opts(&["0"]), false),
            Self::RemoteHeads => Feature::new(opts(&["0"]), false),
            Self::ExtraBranchRefs => Feature::new(opts(&[]), false),
            // The one feature gitweb ships overridable by default.
            Self::EmailPrivacy => Feature::new(opts(&["0"]), true),
        }
    }
}

/// One configuration source's partial contribution: the settings it sets, and
/// nothing it leaves untouched. A loaded config file maps to one of these; the
/// rule overlays them weakest-first. `None`/absent means "this source is silent
/// here, keep what the weaker sources resolved".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsLayer {
    pub projectroot: Option<String>,
    pub site_name: Option<String>,
    pub logo: Option<String>,
    pub favicon: Option<String>,
    pub default_projects_order: Option<String>,
    pub projects_list_description_width: Option<usize>,
    pub omit_age_column: Option<bool>,
    pub omit_owner: Option<bool>,
    pub fallback_encoding: Option<String>,
    pub prevent_xss: Option<bool>,
    pub stylesheets: Option<Vec<String>>,
    pub git_base_url_list: Option<Vec<String>>,
    /// Only the features this source touches; each field overlays independently.
    pub features: BTreeMap<FeatureName, FeatureLayer>,
}

/// A source's partial contribution to one feature: it may set the default, the
/// override flag, both, or neither — independently of the other.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureLayer {
    pub default: Option<Vec<String>>,
    pub overridable: Option<bool>,
}

/// One repository's `gitweb.<key>` config values for the overridable features —
/// the reads gitweb's `git_get_project_config` makes per project, collected once
/// by the adapter. A feature present here is one the repository set; the inner
/// `Option<String>` is its raw value, `None` for a valueless key (`[gitweb]
/// blame`), which a boolean feature reads as on. The domain owns the value rules;
/// the adapter hands over the raw strings only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectOverrides {
    values: BTreeMap<FeatureName, Option<String>>,
}

impl ProjectOverrides {
    /// An empty set — the repository sets no `gitweb.*` config.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records the repository's value for `feature`'s key (`None` = a valueless
    /// key, e.g. `[gitweb] blame` with no `=`).
    pub fn set(&mut self, feature: FeatureName, value: Option<String>) {
        self.values.insert(feature, value);
    }

    /// The repository's value for `feature`: outer `None` means the key is absent
    /// (keep the site default); inner `None` means a valueless key.
    #[must_use]
    fn get(&self, feature: FeatureName) -> Option<Option<&str>> {
        self.values
            .get(&feature)
            .map(|value: &Option<String>| value.as_deref())
    }
}

/// The resolved, effective global configuration the rest of the app reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    projectroot: String,
    site_name: String,
    logo: String,
    favicon: String,
    default_projects_order: String,
    projects_list_description_width: usize,
    omit_age_column: bool,
    omit_owner: bool,
    fallback_encoding: String,
    prevent_xss: bool,
    stylesheets: Vec<String>,
    git_base_url_list: Vec<String>,
    features: BTreeMap<FeatureName, Feature>,
}

impl Settings {
    /// gitweb's compiled-in defaults: the values of its `our` declarations and
    /// the `%feature` hash before any config file is read.
    #[must_use]
    pub fn builtin() -> Self {
        let features: BTreeMap<FeatureName, Feature> = FeatureName::ALL
            .into_iter()
            .map(|name: FeatureName| (name, name.builtin()))
            .collect();
        Self {
            projectroot: String::new(),
            site_name: "Untitled Git".to_owned(),
            logo: "static/git-logo.png".to_owned(),
            favicon: "static/git-favicon.png".to_owned(),
            default_projects_order: "project".to_owned(),
            projects_list_description_width: 25,
            omit_age_column: false,
            omit_owner: false,
            fallback_encoding: "latin1".to_owned(),
            prevent_xss: false,
            stylesheets: vec!["static/gitweb.css".to_owned()],
            git_base_url_list: Vec::new(),
            features,
        }
    }

    /// Resolves the effective settings: the built-in defaults overlaid by each
    /// source in turn, weakest first, so the strongest source that sets a value
    /// wins. The per-project layer, when present, is applied last by its caller.
    #[must_use]
    pub fn resolve(layers: &[SettingsLayer]) -> Self {
        let mut resolved: Self = Self::builtin();
        for layer in layers {
            resolved.apply(layer);
        }
        resolved
    }

    /// Overlays one source on top of the current values: each set scalar/list
    /// replaces, each feature's `default` and `override` flag overlay
    /// independently.
    fn apply(&mut self, layer: &SettingsLayer) {
        replace(&mut self.projectroot, layer.projectroot.as_ref());
        replace(&mut self.site_name, layer.site_name.as_ref());
        replace(&mut self.logo, layer.logo.as_ref());
        replace(&mut self.favicon, layer.favicon.as_ref());
        replace(
            &mut self.default_projects_order,
            layer.default_projects_order.as_ref(),
        );
        replace(
            &mut self.projects_list_description_width,
            layer.projects_list_description_width.as_ref(),
        );
        replace(&mut self.omit_age_column, layer.omit_age_column.as_ref());
        replace(&mut self.omit_owner, layer.omit_owner.as_ref());
        replace(
            &mut self.fallback_encoding,
            layer.fallback_encoding.as_ref(),
        );
        replace(&mut self.prevent_xss, layer.prevent_xss.as_ref());
        replace(&mut self.stylesheets, layer.stylesheets.as_ref());
        replace(
            &mut self.git_base_url_list,
            layer.git_base_url_list.as_ref(),
        );
        for (name, overlay) in &layer.features {
            let feature: &mut Feature =
                self.features.entry(*name).or_insert_with(|| name.builtin());
            replace(&mut feature.default, overlay.default.as_ref());
            replace(&mut feature.overridable, overlay.overridable.as_ref());
        }
    }

    /// Absolute filesystem path prepended to every project path (`$projectroot`).
    #[must_use]
    pub fn projectroot(&self) -> &str {
        &self.projectroot
    }

    /// Site/organization name shown in page titles (`$site_name`).
    #[must_use]
    pub fn site_name(&self) -> &str {
        &self.site_name
    }

    /// The site logo URL (`$logo`), gitweb's `static/git-logo.png` by default —
    /// the RSS feed's `<image>` and the Atom feed's `<logo>`.
    #[must_use]
    pub fn logo(&self) -> &str {
        &self.logo
    }

    /// The site favicon URL (`$favicon`), gitweb's `static/git-favicon.png` by
    /// default — the Atom feed's `<icon>`, and the RSS `<image>` fallback when no
    /// logo is set.
    #[must_use]
    pub fn favicon(&self) -> &str {
        &self.favicon
    }

    /// Default ordering of the projects list (`$default_projects_order`).
    #[must_use]
    pub fn default_projects_order(&self) -> &str {
        &self.default_projects_order
    }

    /// Width, in characters, of the projects-list description column.
    #[must_use]
    pub fn projects_list_description_width(&self) -> usize {
        self.projects_list_description_width
    }

    /// Whether to omit the age column from the projects list (`$omit_age_column`).
    #[must_use]
    pub fn omit_age_column(&self) -> bool {
        self.omit_age_column
    }

    /// Whether to omit owner information from project pages (`$omit_owner`).
    #[must_use]
    pub fn omit_owner(&self) -> bool {
        self.omit_owner
    }

    /// Encoding assumed for bytes that are not valid UTF-8 (`$fallback_encoding`).
    #[must_use]
    pub fn fallback_encoding(&self) -> &str {
        &self.fallback_encoding
    }

    /// Whether to disable repository-owner script injection (`$prevent_xss`).
    #[must_use]
    pub fn prevent_xss(&self) -> bool {
        self.prevent_xss
    }

    /// Stylesheet URIs linked from every page (`@stylesheets`).
    #[must_use]
    pub fn stylesheets(&self) -> &[String] {
        &self.stylesheets
    }

    /// Base URLs for clone links (`@git_base_url_list`).
    #[must_use]
    pub fn git_base_url_list(&self) -> &[String] {
        &self.git_base_url_list
    }

    /// The resolved feature toggle for `name`. Every feature is always present:
    /// [`builtin`](Self::builtin) seeds the whole `%feature` set.
    #[must_use]
    pub fn feature(&self, name: FeatureName) -> &Feature {
        self.features
            .get(&name)
            .expect("builtin populates every feature")
    }

    /// Re-resolves the feature toggles against one repository's `gitweb.<key>`
    /// config — gitweb's per-project `gitweb_get_feature`. An overridable feature
    /// (the site enabled its `override`) whose key the repository sets takes the
    /// repository's value, read by the feature's rule; a non-overridable feature,
    /// a key the repository never sets, and every scalar are left untouched. The
    /// per-project layer is the strongest of all, applied after global resolution.
    #[must_use]
    pub fn for_project(&self, overrides: &ProjectOverrides) -> Settings {
        let mut resolved: Settings = self.clone();
        for &name in &FeatureName::ALL {
            let feature: &Feature = &self.features[&name];
            if !feature.is_overridable() {
                continue; // gitweb: `!$override` returns @defaults, repo unread
            }
            let Some(rule) = name.override_rule() else {
                continue; // no `feature_*` sub: not overridable upstream
            };
            let Some(raw) = overrides.get(name) else {
                continue; // `git_get_project_config` found nothing: keep default
            };
            resolved
                .features
                .insert(name, apply_override(feature, rule, raw));
        }
        resolved
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::builtin()
    }
}

/// Overlays one source's value onto a target: a set source replaces, an absent
/// one leaves the weaker-resolved value in place. This is the scalar/list
/// compose kind (a list `Vec<String>` replaces wholesale, never appends).
fn replace<T: Clone>(target: &mut T, source: Option<&T>) {
    if let Some(value) = source {
        *target = value.clone();
    }
}

/// Builds an owned option list from string slices (e.g. a feature's default).
fn opts(items: &[&str]) -> Vec<String> {
    items.iter().map(|item: &&str| (*item).to_owned()).collect()
}

/// gitweb's per-feature override rule: which `feature_*` sub interprets the
/// repository's raw `gitweb.<key>` value into the effective options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverrideRule {
    /// gitweb's `feature_bool` over `config_to_bool` (`gitweb.blame` &c.).
    Bool,
    /// gitweb's `feature_snapshot`: a `,`/whitespace list of formats, `none` for
    /// none; a falsy value keeps the site default (`gitweb.snapshot`).
    SnapshotList,
    /// gitweb's `feature_patches` over `config_to_int`: an integer with optional
    /// git k/m/g unit; an absent key keeps the site default (`gitweb.patches`).
    Int,
    /// gitweb's `feature_avatar`: the value taken verbatim as a single option; an
    /// absent key keeps the site default (`gitweb.avatar`).
    Value,
    /// gitweb's `feature_extra_branch_refs`: the value split on runs of
    /// whitespace; a falsy value keeps the site default (`gitweb.extrabranchrefs`).
    WsList,
}

/// Re-reads one overridable feature's options from the repository's raw
/// `gitweb.<key>` value (`raw`), per its [`OverrideRule`]. The overridability
/// flag is carried through unchanged — only the default options are recomputed.
fn apply_override(feature: &Feature, rule: OverrideRule, raw: Option<&str>) -> Feature {
    let default: Vec<String> = match rule {
        OverrideRule::Bool => {
            let on: &str = if config_to_bool(raw) { "1" } else { "0" };
            vec![on.to_owned()]
        }
        OverrideRule::SnapshotList => match raw {
            // gitweb's `if ($val)` gate: a truthy value is read, "none" clears,
            // anything else splits; a falsy value falls through to the default.
            Some(value) if perl_truthy(value) => snapshot_formats(value),
            _ => feature.default_options().to_vec(),
        },
        OverrideRule::Int => match raw {
            // gitweb's `if (@val)` gate: a present key is read as an int; an
            // absent (or valueless) key falls through to the default.
            Some(value) => vec![config_to_int(value)],
            None => feature.default_options().to_vec(),
        },
        OverrideRule::Value => match raw {
            // gitweb's `@val ? @val : @_`: a present key is taken verbatim; an
            // absent (or valueless) key falls through to the default.
            Some(value) => vec![value.to_owned()],
            None => feature.default_options().to_vec(),
        },
        OverrideRule::WsList => match raw {
            // gitweb's `if ($values)` gate: a truthy value splits on whitespace;
            // a falsy value falls through to the default.
            Some(value) if perl_truthy(value) => {
                value.split_whitespace().map(str::to_owned).collect()
            }
            _ => feature.default_options().to_vec(),
        },
    };
    Feature::new(default, feature.is_overridable())
}

/// gitweb's `feature_snapshot` value rule: `none` yields no formats; otherwise
/// the value splits on commas and runs of whitespace (gitweb's
/// `split /\s*[,\s]\s*/`). Empty fields are dropped — an empty format name is
/// meaningless, the lone divergence from Perl's leading-empty-field on
/// pathological input like `,tgz`.
fn snapshot_formats(value: &str) -> Vec<String> {
    if value == "none" {
        return Vec::new();
    }
    value
        .split(|ch: char| ch == ',' || ch.is_whitespace())
        .filter(|field: &&str| !field.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Perl string truthiness: every string is true except the empty string and the
/// single character `0` (gitweb's bare `if ($val)` over a config scalar).
fn perl_truthy(value: &str) -> bool {
    !value.is_empty() && value != "0"
}

/// gitweb's `config_to_int`: a trimmed integer, applying git's `k`/`m`/`g`
/// (1024/1048576/1073741824) unit suffix when the value is `[0-9]*[kmg]`
/// (an empty number part counts as zero); any other value is returned as-is.
fn config_to_int(value: &str) -> String {
    let value: &str = value.trim();
    if let Some(unit) = value.chars().last() {
        let multiplier: Option<i64> = match unit.to_ascii_lowercase() {
            'k' => Some(1024),
            'm' => Some(1_048_576),
            'g' => Some(1_073_741_824),
            _ => None,
        };
        if let Some(multiplier) = multiplier {
            let number: &str = &value[..value.len() - unit.len_utf8()];
            if number.bytes().all(|byte: u8| byte.is_ascii_digit()) {
                let parsed: i64 = number.parse().unwrap_or(0);
                return (parsed * multiplier).to_string();
            }
        }
    }
    value.to_owned()
}

/// gitweb's `config_to_bool`: how a `--bool` config value reads. A valueless key
/// (`None`, e.g. `[gitweb] blame`) is on; otherwise, after trimming whitespace, a
/// run of digits that is not just zero, or `true`/`yes` (any case), is on, and
/// everything else — `0`, `false`/`no`/`off`, the empty string, any other word —
/// is off.
fn config_to_bool(value: Option<&str>) -> bool {
    let Some(value) = value else {
        return true;
    };
    let value: &str = value.trim();
    let is_truthy_number: bool =
        !value.is_empty() && value.bytes().all(|byte: u8| byte.is_ascii_digit()) && value != "0";
    is_truthy_number || value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("yes")
}
