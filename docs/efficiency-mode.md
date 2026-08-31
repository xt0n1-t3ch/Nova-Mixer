# Understand efficiency mode

NovaMixer efficiency mode requests the two process properties that Windows Task Manager uses for its green leaf. It also keeps the audio worker responsive while the rest of the process runs at reduced priority.

## Task Manager checks two properties

Task Manager requires Eco Quality of Service (EcoQoS) and a low base priority. NovaMixer enables execution-speed power throttling with `SetProcessInformation` and sets `IDLE_PRIORITY_CLASS` with `SetPriorityClass`.

Disabling the setting clears execution-speed throttling and restores normal priority. NovaMixer never enables `PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION` because the meter tick depends on timer behavior.

## The audio worker remains responsive

The audio thread applies saved policy when sessions appear and runs the meter tick. NovaMixer exempts that thread from execution-speed throttling with `SetThreadInformation` while process-level EcoQoS remains active.

## Windows support controls availability

EcoQoS requires Windows 11 or a Windows build that exposes process power throttling. `EfficiencyStatus.supported` is false when the API is unavailable. `EfficiencyStatus.detail` reports an API or priority failure instead of treating it as success.

NovaMixer stores the opt-in setting in `AppSettings.efficiency_mode` and reapplies it during startup.
