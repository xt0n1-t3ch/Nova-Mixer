use novamixer_contracts::{Application, Group, IdentityKind, Scene};
use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionPolicy {
    pub volume: Option<f32>,
    pub muted: Option<bool>,
}

pub fn resolve_app_key(
    aumid: Option<&str>,
    exe_path: Option<&str>,
    exe_name: Option<&str>,
) -> String {
    if let Some(aumid) = nonempty(aumid) {
        return aumid.to_lowercase();
    }
    if let Some(path) = nonempty(exe_path) {
        return canonicalize_identity_path(path);
    }
    nonempty(exe_name).unwrap_or("unknown").to_lowercase()
}

pub fn group_for<'a>(app_key: &str, groups: &'a [Group]) -> Option<&'a Group> {
    let key = app_key.to_lowercase();
    let bare = Path::new(app_key)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(app_key)
        .to_lowercase();
    groups.iter().find(|group| {
        group.app_keys.iter().any(|app_key| {
            let candidate = app_key.to_lowercase();
            let candidate_bare = Path::new(app_key)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(app_key)
                .to_lowercase();
            candidate == key || candidate == bare || candidate_bare == key || candidate_bare == bare
        })
    })
}

pub fn policy_for_new_session(group: Option<&Group>) -> SessionPolicy {
    match group {
        Some(group) if group.auto_mute_on_launch => SessionPolicy {
            volume: None,
            muted: Some(true),
        },
        Some(group) if group.startup_volume.is_some() => SessionPolicy {
            volume: group.startup_volume.map(clamp_scalar),
            muted: None,
        },
        Some(group) => SessionPolicy {
            volume: Some(clamp_scalar(group.volume)),
            muted: None,
        },
        None => SessionPolicy {
            volume: None,
            muted: None,
        },
    }
}

pub fn clamp_scalar(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub fn clamp_percent(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 100.0)
    } else {
        0.0
    }
}

pub fn percent_to_scalar(value: f32) -> f32 {
    clamp_percent(value) / 100.0
}

// Paired definition: frontend/src/lib/volume.ts effectiveStep.
pub fn effective_step(current: f32, step: f32, smart: bool) -> f32 {
    if !smart {
        return step;
    }
    let level = clamp_scalar(current);
    if level <= 0.3 {
        step * 0.5
    } else if level >= 0.7 {
        step * 1.5
    } else {
        step
    }
}

pub fn stepped_volume(current: f32, delta: f32) -> f32 {
    clamp_scalar(((clamp_scalar(current) + delta) * 1000.0).round() / 1000.0)
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

/// Token that replaces a Squirrel.Windows version folder in a path identity.
pub const SQUIRREL_VERSION_TOKEN: &str = "app-*";

fn canonicalize_identity_path(path: &str) -> String {
    let normalized = path.replace('/', "\\");
    let mut prefix = String::new();
    let mut parts: Vec<String> = Vec::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::Prefix(value) => prefix = value.as_os_str().to_string_lossy().into_owned(),
            Component::RootDir => {}
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
        }
    }
    // Squirrel.Windows installs every update into a new `app-<version>` folder
    // (Discord, Slack, Teams classic, GitHub Desktop, Postman, ...), so the
    // concrete folder would change the identity on every update. Only directory
    // components are rewritten; the executable file name is never touched.
    if let Some((_, directories)) = parts.split_last_mut() {
        for directory in directories {
            if squirrel_version(directory).is_some() {
                *directory = SQUIRREL_VERSION_TOKEN.to_owned();
            }
        }
    }
    let separator = if prefix.is_empty() { "" } else { "\\" };
    format!("{prefix}{separator}{}", parts.join("\\")).to_lowercase()
}

/// Parses a Squirrel.Windows version folder name: `app-` followed by at least
/// two dot-separated, all-digit parts (`app-1.0.9256`), compared
/// case-insensitively. Returns the numeric parts.
fn squirrel_version(component: &str) -> Option<Vec<u64>> {
    let prefix = component.get(..4)?;
    if !prefix.eq_ignore_ascii_case("app-") {
        return None;
    }
    let parts: Vec<&str> = component[4..].split('.').collect();
    if parts.len() < 2
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    parts.iter().map(|part| part.parse().ok()).collect()
}

/// The Squirrel version encoded in a path's directories, if any.
fn path_squirrel_version(path: &str) -> Option<Vec<u64>> {
    let normalized = path.replace('/', "\\");
    let mut components: Vec<&str> = normalized.split('\\').collect();
    components.pop();
    components.into_iter().rev().find_map(squirrel_version)
}

/// Re-derives persisted path identities with the current rule and collapses
/// applications that now share one `app_key`.
///
/// Settings written before the Squirrel rule hold one entry per version folder
/// (`...\discord\app-1.0.9256\discord.exe`, `...\app-1.0.9258\...`). This
/// rewrites every `IdentityKind::Path` key, merges entries that resolve to the
/// same key into one, and rewrites group members and scene entries to the new
/// keys without duplicates.
///
/// The surviving entry is chosen deterministically: an entry whose
/// `executable_path` exists wins, then the highest Squirrel version (the most
/// recently installed, therefore most recently seen), then the earliest entry.
/// It keeps its position in the list. User choices on any merged entry survive:
/// `pinned` and `hidden` are kept when any entry had them, `custom_name`,
/// `group_id` and `icon` come from the survivor or else the first entry that has
/// one, and the remembered level (`volume`, `muted`) comes from the best-ranked
/// entry that is `remembered`.
///
/// Returns `true` when anything changed.
pub fn migrate_path_identities(
    applications: &mut Vec<Application>,
    groups: &mut [Group],
    scenes: &mut [Scene],
    exists: impl Fn(&str) -> bool,
) -> bool {
    let mut renamed: HashMap<String, String> = HashMap::new();
    let mut changed = false;
    for app in applications.iter_mut() {
        if app.identity_kind != IdentityKind::Path {
            continue;
        }
        let key = canonicalize_identity_path(&app.app_key);
        if key != app.app_key {
            renamed.insert(app.app_key.to_lowercase(), key.clone());
            app.app_key = key;
            changed = true;
        }
    }

    let mut order: Vec<String> = Vec::new();
    let mut buckets: HashMap<String, Vec<(usize, Application)>> = HashMap::new();
    for (index, app) in std::mem::take(applications).into_iter().enumerate() {
        let key = app.app_key.to_lowercase();
        let bucket = buckets.entry(key.clone()).or_default();
        if bucket.is_empty() {
            order.push(key);
        }
        bucket.push((index, app));
    }
    for key in order {
        let mut bucket = buckets.remove(&key).expect("bucket exists");
        if bucket.len() == 1 {
            applications.push(bucket.pop().expect("one entry").1);
            continue;
        }
        changed = true;
        bucket.sort_by(|(left_index, left), (right_index, right)| {
            let rank = |app: &Application| {
                (
                    app.executable_path.as_deref().is_some_and(&exists),
                    app.executable_path
                        .as_deref()
                        .and_then(path_squirrel_version)
                        .or_else(|| path_squirrel_version(&app.app_key)),
                )
            };
            rank(right)
                .cmp(&rank(left))
                .then(left_index.cmp(right_index))
        });
        let ranked: Vec<Application> = bucket.into_iter().map(|(_, app)| app).collect();
        applications.push(merge_ranked(ranked));
    }

    for group in groups.iter_mut() {
        let before = group.app_keys.clone();
        let mut seen = HashSet::new();
        group.app_keys = before
            .iter()
            .map(|key| rewrite_key(key, &renamed))
            .filter(|key| seen.insert(key.to_lowercase()))
            .collect();
        changed |= group.app_keys != before;
    }
    for scene in scenes.iter_mut() {
        let before = scene.entries.len();
        let mut seen = HashSet::new();
        for entry in &mut scene.entries {
            let key = rewrite_key(&entry.app_key, &renamed);
            if key != entry.app_key {
                entry.app_key = key;
                changed = true;
            }
        }
        scene
            .entries
            .retain(|entry| seen.insert(entry.app_key.to_lowercase()));
        changed |= scene.entries.len() != before;
    }
    changed
}

fn rewrite_key(key: &str, renamed: &HashMap<String, String>) -> String {
    if let Some(new_key) = renamed.get(&key.to_lowercase()) {
        return new_key.clone();
    }
    // A member without a matching application record is still rewritten when it
    // is an absolute Windows path, so it keeps matching the live session.
    let bytes = key.as_bytes();
    if bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let canonical = canonicalize_identity_path(key);
        if canonical != key.to_lowercase() {
            return canonical;
        }
    }
    key.to_owned()
}

fn merge_ranked(ranked: Vec<Application>) -> Application {
    let mut survivor = ranked[0].clone();
    let others = &ranked[1..];
    survivor.pinned = ranked.iter().any(|app| app.pinned);
    survivor.hidden = ranked.iter().any(|app| app.hidden);
    if survivor.custom_name.is_none() {
        survivor.custom_name = others.iter().find_map(|app| app.custom_name.clone());
    }
    if let Some(name) = &survivor.custom_name {
        survivor.display_name = name.clone();
    }
    if survivor.group_id.is_none() {
        survivor.group_id = others.iter().find_map(|app| app.group_id.clone());
    }
    if survivor.icon.is_none() {
        survivor.icon = others.iter().find_map(|app| app.icon.clone());
    }
    if survivor.executable_name.is_none() {
        survivor.executable_name = others.iter().find_map(|app| app.executable_name.clone());
    }
    if let Some(policy) = ranked.iter().find(|app| app.remembered) {
        survivor.remembered = true;
        survivor.volume = policy.volume;
        survivor.muted = policy.muted;
    }
    survivor.sort_order = ranked.iter().map(|app| app.sort_order).min().unwrap_or(0);
    survivor
}

#[cfg(test)]
mod tests {
    use super::*;
    use novamixer_contracts::Group;

    fn group(app_key: &str) -> Group {
        Group {
            id: "group".into(),
            name: "Main".into(),
            is_default: true,
            volume: 0.625,
            app_keys: vec![app_key.into()],
            startup_volume: None,
            auto_mute_on_launch: false,
            hotkeys_enabled: true,
        }
    }

    #[test]
    fn identity_prefers_aumid_then_path_then_name() {
        assert_eq!(
            resolve_app_key(
                Some("SpotifyAB.Spotify"),
                Some("C:\\Spotify.exe"),
                Some("Spotify.exe")
            ),
            "spotifyab.spotify"
        );
        assert_eq!(
            resolve_app_key(None, Some("C:/Apps/../Apps/Spotify.exe"), Some("other.exe")),
            "c:\\apps\\spotify.exe"
        );
        assert_eq!(
            resolve_app_key(None, None, Some("Spotify.exe")),
            "spotify.exe"
        );
    }

    #[test]
    fn squirrel_version_folders_share_one_identity() {
        let old = resolve_app_key(
            None,
            Some("C:\\Users\\xt0n1\\AppData\\Local\\Discord\\app-1.0.9256\\Discord.exe"),
            Some("Discord.exe"),
        );
        let new = resolve_app_key(
            None,
            Some("c:\\users\\xt0n1\\appdata\\local\\discord\\APP-1.0.9259\\discord.exe"),
            Some("Discord.exe"),
        );
        assert_eq!(old, new);
        assert_eq!(
            old,
            "c:\\users\\xt0n1\\appdata\\local\\discord\\app-*\\discord.exe"
        );
        assert_eq!(
            resolve_app_key(None, Some("C:/Slack/app-4.41.105/slack.exe"), None),
            "c:\\slack\\app-*\\slack.exe"
        );
    }

    #[test]
    fn non_squirrel_folders_keep_their_identity() {
        for path in [
            "C:\\Program Files\\App\\tool.exe",
            "C:\\Program Files\\apps\\tool.exe",
            "C:\\Program Files\\app-data\\tool.exe",
            "C:\\Program Files\\myapp-1.0\\tool.exe",
            "C:\\Program Files\\app-1\\tool.exe",
            "C:\\Program Files\\app-1.0-beta\\tool.exe",
            "C:\\Program Files\\app-1..2\\tool.exe",
            "C:\\Program Files\\app-v1.2\\tool.exe",
            "C:\\Tools\\app-1.2.3",
        ] {
            assert_eq!(
                resolve_app_key(None, Some(path), None),
                path.to_lowercase(),
                "{path} must not be rewritten"
            );
        }
    }

    fn discord(version: &str) -> Application {
        let path = format!("c:\\users\\x\\appdata\\local\\discord\\app-{version}\\discord.exe");
        let mut app = Application::offline(path.clone(), IdentityKind::Path, "Discord".into());
        app.executable_name = Some("Discord.exe".into());
        app.executable_path = Some(path);
        app
    }

    #[test]
    fn migration_collapses_versioned_entries_and_rewrites_groups() {
        let mut old = discord("1.0.9256");
        old.volume = 0.3;
        old.pinned = true;
        old.custom_name = Some("Chat".into());
        let mut newer = discord("1.0.9258");
        newer.remembered = false;
        newer.volume = 0.9;
        newer.sort_order = 1;
        let mut applications = vec![old.clone(), newer.clone()];
        let mut groups = vec![group(&old.app_key)];
        groups[0].app_keys.push(newer.app_key.clone());
        let live = newer.executable_path.clone().unwrap();
        assert!(migrate_path_identities(
            &mut applications,
            &mut groups,
            &mut [],
            |path| path == live
        ));
        assert_eq!(applications.len(), 1);
        let app = &applications[0];
        let key = "c:\\users\\x\\appdata\\local\\discord\\app-*\\discord.exe";
        assert_eq!(app.app_key, key);
        assert_eq!(app.executable_path.as_deref(), Some(live.as_str()));
        assert!(app.pinned && app.remembered);
        assert_eq!(app.custom_name.as_deref(), Some("Chat"));
        assert_eq!(app.volume, 0.3);
        assert_eq!(groups[0].app_keys, vec![key.to_owned()]);
        assert!(!migrate_path_identities(
            &mut applications,
            &mut groups,
            &mut [],
            |_| false
        ));
    }

    #[test]
    fn migration_prefers_newest_version_when_no_path_exists() {
        let mut applications = vec![discord("1.0.9256"), discord("1.0.10000")];
        migrate_path_identities(&mut applications, &mut [], &mut [], |_| false);
        assert_eq!(applications.len(), 1);
        assert!(applications[0]
            .executable_path
            .as_deref()
            .is_some_and(|path| path.contains("app-1.0.10000")));
    }

    #[test]
    fn migrated_bare_executable_matches_full_path() {
        let groups = vec![group("spotify.exe")];
        assert_eq!(
            group_for("C:\\Users\\x\\Spotify.exe", &groups).map(|g| g.id.as_str()),
            Some("group")
        );
    }

    #[test]
    fn policy_precedence_is_mute_startup_group_none() {
        let mut value = group("spotify.exe");
        assert_eq!(policy_for_new_session(Some(&value)).volume, Some(0.625));
        value.startup_volume = Some(0.4);
        assert_eq!(policy_for_new_session(Some(&value)).volume, Some(0.4));
        value.auto_mute_on_launch = true;
        assert_eq!(
            policy_for_new_session(Some(&value)),
            SessionPolicy {
                volume: None,
                muted: Some(true)
            }
        );
        assert_eq!(
            policy_for_new_session(None),
            SessionPolicy {
                volume: None,
                muted: None
            }
        );
    }

    #[test]
    fn smart_step_matches_frontend_thresholds() {
        assert_eq!(effective_step(0.3, 0.1, true), 0.05);
        assert_eq!(effective_step(0.5, 0.1, true), 0.1);
        assert!((effective_step(0.7, 0.1, true) - 0.15).abs() < f32::EPSILON);
        assert_eq!(effective_step(0.1, 0.1, false), 0.1);
    }
}
