//! Type registration owner for the frontend/backend IPC contract.

#[allow(dead_code)]
pub fn contract_type_names() -> &'static [&'static str] {
    &[
        "AudioSession",
        "SessionState",
        "MasterState",
        "MixerSnapshot",
        "PeakBatch",
        "SessionPeak",
        "AppBinding",
        "Group",
        "HotkeyBinding",
        "HotkeyAction",
        "UiPrefs",
        "AppSettings",
        "ApiError",
    ]
}
