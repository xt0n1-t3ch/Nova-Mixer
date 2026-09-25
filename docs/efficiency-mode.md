# Understand efficiency mode

NovaMixer efficiency mode sets the two process properties that Windows Task Manager uses for its green leaf. It keeps the audio worker responsive while the rest of the process runs at reduced priority.

## Task Manager checks two properties

Task Manager shows Efficiency mode only when a process has both Eco Quality of Service (EcoQoS) and a low base priority. Source: Microsoft, "Reduce Process Interference with Task Manager Efficiency Mode".

Turning the setting on makes two calls on the NovaMixer process:

- `SetProcessInformation(ProcessPowerThrottling)` with `ControlMask` and `StateMask` set to `PROCESS_POWER_THROTTLING_EXECUTION_SPEED`
- `SetPriorityClass(IDLE_PRIORITY_CLASS)`

Turning it off keeps `ControlMask` at `PROCESS_POWER_THROTTLING_EXECUTION_SPEED` with a `StateMask` of zero. That is an explicit high-QoS opt-out, not a return to system heuristics. It also restores `NORMAL_PRIORITY_CLASS`.

NovaMixer never enables `PROCESS_POWER_THROTTLING_IGNORE_TIMER_RESOLUTION` because the meter tick depends on timer behavior.

## The reported status is the real process state

`set_efficiency_mode` and `get_efficiency_status` query the running process after every change. They read the EcoQoS mask with `GetProcessInformation(ProcessPowerThrottling)` and the base priority with `GetPriorityClass`. `EfficiencyStatus.enabled` is true only when the process has both EcoQoS and `IDLE_PRIORITY_CLASS`. It is not the saved setting.

If Windows rejects either call, or the queried state does not match the request, `EfficiencyStatus.detail` names the failing call. `set_efficiency_mode` saves the setting only when the queried state matches the request.

## Startup reapplies the saved setting

NovaMixer stores the opt-in in `AppSettings.efficiency_mode`. During startup, it reapplies efficiency mode after the audio service starts and before the tray is set up. A failure is logged as a warning, and `get_efficiency_status` reports the real state.

## The audio worker keeps full speed

The `windows-audio` thread applies saved policy when sessions appear and drives the meter tick and scene fades. Two measures protect it:

- **EcoQoS exemption:** at thread start, `SetThreadInformation(ThreadPowerThrottling)` sets `ControlMask` to `THREAD_POWER_THROTTLING_EXECUTION_SPEED` and `StateMask` to zero. The thread keeps high QoS while the process has EcoQoS.
- **Priority boost:** in an idle-class process, a normal thread runs at base priority 4, and any busy normal-priority thread on the system would starve it. On each loop iteration, the worker checks the process priority class. While the class is `IDLE_PRIORITY_CLASS`, it sets `THREAD_PRIORITY_TIME_CRITICAL`, which is base priority 15 in that class. Otherwise, it returns to `THREAD_PRIORITY_NORMAL`. The worker blocks between short bursts, so the boost costs no sustained CPU.

Other NovaMixer threads, such as the UI thread and the async runtime, run at idle priority with EcoQoS. They only react to user input and events.

## WebView2 processes are not changed

The interface runs in Microsoft Edge WebView2 child processes (`msedgewebview2.exe`). They are separate processes. NovaMixer changes only its own process and never sets EcoQoS or the priority of the WebView2 processes. These processes are owned by the WebView2 runtime, and a slower renderer would slow the interface the user is looking at.

In Task Manager, the leaf appears on the NovaMixer entry in the Processes tab and in the Status column of `novamixer.exe` in the Details tab. When you expand the NovaMixer group, `msedgewebview2.exe` rows might show no leaf, or might show one that WebView2 applied itself. Edge-based processes sometimes lower their own priority and apply EcoQoS when hidden.

## Windows support controls availability

EcoQoS requires Windows 11 or a Windows build that exposes process power throttling. `EfficiencyStatus.supported` is false when `GetProcessInformation(ProcessPowerThrottling)` is unavailable.

## Verify a running NovaMixer

Run the read-only check from the repository root:

```powershell
pwsh -NoProfile -File scripts/check-efficiency.ps1
```

The script finds `novamixer.exe` and `novamixer-app.exe`, plus any `msedgewebview2.exe` descendants. For each process, it prints the EcoQoS state, the priority class, whether the leaf conditions hold, and the raw masks. To check specific processes, run `./scripts/check-efficiency.ps1 -ProcessId 1234, 5678` from a PowerShell prompt. With efficiency mode on, the NovaMixer row shows `on`, `Idle`, and `YES`. The script exits with code 1 when no process matches.

The Rust test `efficiency_mode_sets_and_restores_both_task_manager_properties` in `src-tauri/src/efficiency.rs` checks the same properties in-process. Run it with `cargo test -p novamixer-app efficiency`.
