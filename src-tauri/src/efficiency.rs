use novamixer_contracts::EfficiencyStatus;

#[cfg(windows)]
mod platform {
    use super::EfficiencyStatus;
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetPriorityClass, GetProcessInformation, ProcessPowerThrottling,
        SetPriorityClass, SetProcessInformation, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        PROCESS_POWER_THROTTLING_STATE,
    };

    /// Requests both properties Task Manager checks for the leaf, then reports what Windows
    /// actually holds afterwards rather than what was requested.
    pub fn set(enabled: bool) -> EfficiencyStatus {
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            // Keep the control bit when disabling: an explicit HighQoS opt-out rather than a
            // return to system heuristics.
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if enabled {
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            } else {
                0
            },
        };
        let (eco, priority) = unsafe {
            let process = GetCurrentProcess();
            let eco = SetProcessInformation(
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
            (eco, priority)
        };
        let mut status = get();
        let mut failures: Vec<String> = [
            eco.err()
                .map(|error| format!("SetProcessInformation(ProcessPowerThrottling): {error}")),
            priority
                .err()
                .map(|error| format!("SetPriorityClass: {error}")),
            status.detail.take(),
        ]
        .into_iter()
        .flatten()
        .collect();
        if failures.is_empty() && status.enabled != enabled {
            failures.push(format!(
                "Windows did not report efficiency mode as {} after the request",
                if enabled { "on" } else { "off" }
            ));
        }
        if !failures.is_empty() {
            status.detail = Some(failures.join("; "));
        }
        status
    }

    /// Queries the live process: the EcoQoS state mask and the base priority class.
    pub fn get() -> EfficiencyStatus {
        unsafe {
            let process = GetCurrentProcess();
            let priority = GetPriorityClass(process);
            if priority == 0 {
                return EfficiencyStatus {
                    supported: true,
                    enabled: false,
                    detail: Some(format!(
                        "GetPriorityClass: {}",
                        windows::core::Error::from_thread()
                    )),
                };
            }
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetPriorityClass, GetProcessInformation, ProcessPowerThrottling,
        SetPriorityClass, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        PROCESS_POWER_THROTTLING_STATE,
    };

    /// Reads the process with the calls Task Manager's leaf depends on, independently of
    /// `get()`, so a status function that reports the request cannot make the test pass.
    fn observe() -> (u32, PROCESS_POWER_THROTTLING_STATE) {
        unsafe {
            let process = GetCurrentProcess();
            let mut state = PROCESS_POWER_THROTTLING_STATE {
                Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
                ..Default::default()
            };
            GetProcessInformation(
                process,
                ProcessPowerThrottling,
                &mut state as *mut _ as *mut _,
                std::mem::size_of_val(&state) as u32,
            )
            .expect("GetProcessInformation(ProcessPowerThrottling)");
            (GetPriorityClass(process), state)
        }
    }

    #[test]
    fn efficiency_mode_sets_and_restores_both_task_manager_properties() {
        let on = set(true);
        assert_eq!(
            on,
            EfficiencyStatus {
                supported: true,
                enabled: true,
                detail: None
            }
        );
        let (priority, state) = observe();
        assert_eq!(
            priority, IDLE_PRIORITY_CLASS.0,
            "the leaf needs IDLE priority"
        );
        assert_ne!(
            state.ControlMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            0
        );
        assert_ne!(
            state.StateMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            0,
            "the leaf needs EcoQoS"
        );

        // The status follows the real process, not the last request.
        unsafe { SetPriorityClass(GetCurrentProcess(), NORMAL_PRIORITY_CLASS).unwrap() };
        assert!(
            !get().enabled,
            "EcoQoS without IDLE priority is not the leaf"
        );
        assert!(set(true).enabled);

        let off = set(false);
        assert_eq!(
            off,
            EfficiencyStatus {
                supported: true,
                enabled: false,
                detail: None
            }
        );
        let (priority, state) = observe();
        assert_eq!(priority, NORMAL_PRIORITY_CLASS.0);
        assert_ne!(
            state.ControlMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            0,
            "disabling keeps an explicit HighQoS opt-out"
        );
        assert_eq!(
            state.StateMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            0
        );
        assert!(!get().enabled);
    }
}
