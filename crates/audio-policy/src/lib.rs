use novamixer_contracts::Group;
use std::path::{Component, Path};

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
    let separator = if prefix.is_empty() { "" } else { "\\" };
    format!("{prefix}{separator}{}", parts.join("\\")).to_lowercase()
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
