<#
.SYNOPSIS
Report the two properties behind Task Manager's Efficiency mode leaf for NovaMixer.

.DESCRIPTION
For every NovaMixer process (image novamixer.exe / novamixer-app.exe, or the PIDs given),
and every msedgewebview2.exe descendant, prints the EcoQoS state from
GetProcessInformation(ProcessPowerThrottling) and the base priority from GetPriorityClass.
Leaf = EcoQoS on AND priority class Idle. Read-only: it never changes a process.

.EXAMPLE
pwsh -NoProfile -File scripts/check-efficiency.ps1
pwsh -NoProfile -File scripts/check-efficiency.ps1 -ProcessId 1234
#>
[CmdletBinding()]
param(
    [int[]]$ProcessId,
    [string[]]$Name = @('novamixer', 'novamixer-app')
)
$ErrorActionPreference = 'Stop'

if (-not ('NovaMixer.Efficiency' -as [type])) {
    Add-Type -Namespace NovaMixer -Name Efficiency -MemberDefinition @'
[StructLayout(LayoutKind.Sequential)]
public struct PROCESS_POWER_THROTTLING_STATE { public uint Version; public uint ControlMask; public uint StateMask; }

[DllImport("kernel32.dll", SetLastError = true)]
public static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool CloseHandle(IntPtr handle);
[DllImport("kernel32.dll", SetLastError = true)]
public static extern uint GetPriorityClass(IntPtr process);
[DllImport("kernel32.dll", SetLastError = true)]
public static extern bool GetProcessInformation(IntPtr process, int infoClass, ref PROCESS_POWER_THROTTLING_STATE info, int size);
'@
}

$ProcessPowerThrottling = 4
$ExecutionSpeed = 0x1
$QueryLimitedInformation = 0x1000
$classes = @{
    0x40 = 'Idle'; 0x4000 = 'BelowNormal'; 0x20 = 'Normal'
    0x8000 = 'AboveNormal'; 0x80 = 'High'; 0x100 = 'Realtime'
}

function Get-Efficiency([int]$ProcessIdValue) {
    $handle = [NovaMixer.Efficiency]::OpenProcess($QueryLimitedInformation, $false, $ProcessIdValue)
    if ($handle -eq [IntPtr]::Zero) {
        return @{ Error = "OpenProcess failed ($([Runtime.InteropServices.Marshal]::GetLastWin32Error()))" }
    }
    try {
        $priority = [NovaMixer.Efficiency]::GetPriorityClass($handle)
        $state = New-Object NovaMixer.Efficiency+PROCESS_POWER_THROTTLING_STATE
        $state.Version = 1
        $size = [Runtime.InteropServices.Marshal]::SizeOf($state)
        if (-not [NovaMixer.Efficiency]::GetProcessInformation($handle, $ProcessPowerThrottling, [ref]$state, $size)) {
            return @{ Error = "GetProcessInformation failed ($([Runtime.InteropServices.Marshal]::GetLastWin32Error()))" }
        }
        $ecoqos = ($state.StateMask -band $ExecutionSpeed) -ne 0
        $explicit = ($state.ControlMask -band $ExecutionSpeed) -ne 0
        $className = if ($classes.ContainsKey([int]$priority)) { $classes[[int]$priority] } else { ('0x{0:X}' -f $priority) }
        return @{
            EcoQoS   = if ($ecoqos) { 'on' } elseif ($explicit) { 'off' } else { 'off (system-managed)' }
            Priority = $className
            Leaf     = $ecoqos -and $priority -eq 0x40
            Masks    = ('control=0x{0:X} state=0x{1:X}' -f $state.ControlMask, $state.StateMask)
        }
    } finally {
        [void][NovaMixer.Efficiency]::CloseHandle($handle)
    }
}

$all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name
$roots = if ($ProcessId) {
    $all | Where-Object { $ProcessId -contains $_.ProcessId }
} else {
    $names = $Name | ForEach-Object { "$_.exe" }
    $all | Where-Object { $names -contains $_.Name }
}
if (-not $roots) {
    Write-Warning "No matching process found (names: $($Name -join ', '); pids: $($ProcessId -join ', '))."
    exit 1
}

$rows = foreach ($root in $roots) {
    $queue = [Collections.Generic.Queue[object]]::new()
    $queue.Enqueue(@($root, 0))
    $seen = [Collections.Generic.HashSet[int]]::new()
    while ($queue.Count) {
        # Index the pair explicitly: multiple assignment from a dequeued
        # [object[]] does not reliably split it into process and depth.
        $pair = $queue.Dequeue()
        $item = $pair[0]
        $depth = [int]$pair[1]
        # A reused parent PID can form a cycle; visit each process once.
        if (-not $seen.Add([int]$item.ProcessId)) { continue }
        if ($depth -eq 0 -or $item.Name -eq 'msedgewebview2.exe') {
            $info = Get-Efficiency $item.ProcessId
            [pscustomobject]@{
                Process  = ('  ' * [Math]::Min($depth, 1)) + $item.Name
                PID      = $item.ProcessId
                EcoQoS   = if ($info.Error) { '?' } else { $info.EcoQoS }
                Priority = if ($info.Error) { '?' } else { $info.Priority }
                Leaf     = if ($info.Error) { $info.Error } else { if ($info.Leaf) { 'YES' } else { 'no' } }
                Masks    = $info.Masks
            }
        }
        foreach ($child in $all | Where-Object { $_.ParentProcessId -eq $item.ProcessId }) {
            $queue.Enqueue(@($child, $depth + 1))
        }
    }
}
$rows | Format-Table -AutoSize
