#[derive(Debug)]
pub struct EfficiencyProof {
    pub priority_class: u32,
    pub ecoqos_state_mask: u32,
    pub low_priority: bool,
    pub ecoqos: bool,
}

#[cfg(windows)]
pub fn apply(enabled: bool) -> Result<EfficiencyProof, String> {
    use windows::Win32::System::Threading::{
        GetCurrentProcess, GetPriorityClass, GetProcessInformation, ProcessPowerThrottling,
        SetPriorityClass, SetProcessInformation, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
        PROCESS_POWER_THROTTLING_CURRENT_VERSION, PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
        PROCESS_POWER_THROTTLING_STATE,
    };
    unsafe {
        let process = GetCurrentProcess();
        let state = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ControlMask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            StateMask: if enabled {
                PROCESS_POWER_THROTTLING_EXECUTION_SPEED
            } else {
                0
            },
        };
        SetProcessInformation(
            process,
            ProcessPowerThrottling,
            &state as *const _ as *const _,
            std::mem::size_of_val(&state) as u32,
        )
        .map_err(|error| format!("SetProcessInformation failed: {error}"))?;
        SetPriorityClass(
            process,
            if enabled {
                IDLE_PRIORITY_CLASS
            } else {
                NORMAL_PRIORITY_CLASS
            },
        )
        .map_err(|error| format!("SetPriorityClass failed: {error}"))?;
        let priority_class = GetPriorityClass(process);
        let mut actual = PROCESS_POWER_THROTTLING_STATE {
            Version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            ..Default::default()
        };
        GetProcessInformation(
            process,
            ProcessPowerThrottling,
            &mut actual as *mut _ as *mut _,
            std::mem::size_of_val(&actual) as u32,
        )
        .map_err(|error| format!("GetProcessInformation failed: {error}"))?;
        Ok(EfficiencyProof {
            priority_class,
            ecoqos_state_mask: actual.StateMask,
            low_priority: priority_class == IDLE_PRIORITY_CLASS.0,
            ecoqos: actual.StateMask & PROCESS_POWER_THROTTLING_EXECUTION_SPEED != 0,
        })
    }
}

#[cfg(not(windows))]
pub fn apply(_: bool) -> Result<EfficiencyProof, String> {
    Err("Efficiency mode requires Windows 11".into())
}
