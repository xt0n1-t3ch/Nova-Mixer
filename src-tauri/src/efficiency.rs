use novamixer_contracts::EfficiencyStatus;

#[cfg(windows)]
mod platform {
    use super::EfficiencyStatus;
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetPriorityClass, GetProcessInformation, ProcessPowerThrottling,
        SetPriorityClass, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        PROCESS_POWER_THROTTLING_STATE,
    };

    pub fn set(enabled: bool) -> EfficiencyStatus {
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if enabled {
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            } else {
                0
            },
        };
        unsafe {
            let process = GetCurrentProcess();
            let eco = windows::Win32::System::Threading::SetProcessInformation(
                process,
                ProcessPowerThrottling,
                &state as *const _ as *const _,
                std::mem::size_of_val(&state) as u32,
            );
            let priority = SetPriorityClass(
                process,
                if enabled {
                    IDLE_PRIORITY_CLASS
                } else {
                    NORMAL_PRIORITY_CLASS
                },
            );
            match (eco, priority) {
                (Ok(()), Ok(())) => EfficiencyStatus {
                    supported: true,
                    enabled,
                    detail: None,
                },
                (a, b) => EfficiencyStatus {
                    supported: a.is_ok(),
                    enabled: false,
                    detail: Some(format!(
                        "EcoQoS: {}; priority: {}",
                        a.err().map_or_else(|| "ok".into(), |e| e.to_string()),
                        b.err().map_or_else(|| "ok".into(), |e| e.to_string())
                    )),
                },
            }
        }
    }
    pub fn get() -> EfficiencyStatus {
        unsafe {
            let process = GetCurrentProcess();
            let priority = GetPriorityClass(process);
            let mut state = PROCESS_POWER_THROTTLING_STATE {
                Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
                ..Default::default()
            };
            match GetProcessInformation(
                process,
                ProcessPowerThrottling,
                &mut state as *mut _ as *mut _,
                std::mem::size_of_val(&state) as u32,
            ) {
                Ok(()) => EfficiencyStatus {
                    supported: true,
                    enabled: priority == IDLE_PRIORITY_CLASS.0
                        && state.StateMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED != 0,
                    detail: None,
                },
                Err(error) => EfficiencyStatus {
                    supported: false,
                    enabled: false,
                    detail: Some(format!("Cannot query EcoQoS: {error}")),
                },
            }
        }
    }
}
#[cfg(not(windows))]
mod platform {
    use super::EfficiencyStatus;
    pub fn set(_: bool) -> EfficiencyStatus {
        EfficiencyStatus {
            supported: false,
            enabled: false,
            detail: Some("Efficiency mode requires Windows 11".into()),
        }
    }
    pub fn get() -> EfficiencyStatus {
        set(false)
    }
}
pub use platform::{get, set};
