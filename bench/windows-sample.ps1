param([int]$RootProcessId, [string]$RootRole = 'server', [int]$Samples = 15, [int]$IntervalMs = 2000, [int]$WarmupMs = 10000)
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class ProbeProcessTree {
    [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
    public struct Entry {
        public uint Size;
        public uint Usage;
        public uint ProcessId;
        public UIntPtr Heap;
        public uint Module;
        public uint Threads;
        public uint ParentId;
        public int Priority;
        public uint Flags;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst=260)] public string Name;
    }
    [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr CreateToolhelp32Snapshot(uint flags, uint processId);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode)] static extern bool Process32FirstW(IntPtr snapshot, ref Entry entry);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode)] static extern bool Process32NextW(IntPtr snapshot, ref Entry entry);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    public static int[] Descendants(int root) {
        var snapshot = CreateToolhelp32Snapshot(2, 0);
        if (snapshot == new IntPtr(-1)) throw new InvalidOperationException("Process snapshot failed");
        try {
            var entries = new List<Entry>();
            var entry = new Entry { Size=(uint)Marshal.SizeOf<Entry>() };
            if (Process32FirstW(snapshot, ref entry)) {
                do { entries.Add(entry); } while (Process32NextW(snapshot, ref entry));
            }
            var ids = new HashSet<int> { root };
            bool changed;
            do {
                changed = false;
                foreach(var item in entries) if(ids.Contains((int)item.ParentId)) changed |= ids.Add((int)item.ProcessId);
            } while(changed);
            var result = new int[ids.Count];
            ids.CopyTo(result);
            return result;
        } finally { CloseHandle(snapshot); }
    }
}
'@
Start-Sleep -Milliseconds $WarmupMs
$initialIds = [ProbeProcessTree]::Descendants($RootProcessId)
$previous = @{}
foreach($processId in $initialIds) { $previous[$processId] = (Get-Process -Id $processId).TotalProcessorTime.TotalSeconds }
$rows = @()
$clock = [Diagnostics.Stopwatch]::StartNew()
for($sample = 1; $sample -le $Samples; $sample++) {
    Start-Sleep -Milliseconds $IntervalMs
    $elapsed = $clock.Elapsed.TotalSeconds
    $ids = [ProbeProcessTree]::Descendants($RootProcessId)
    $privateTotal = 0L
    $workingTotal = 0L
    $cpuTotal = 0.0
    foreach($processId in $ids) {
        $process = Get-Process -Id $processId
        $cpu = $process.TotalProcessorTime.TotalSeconds
        $delta = if($previous.ContainsKey($processId)) { $cpu - $previous[$processId] } else { 0.0 }
        $previous[$processId] = $cpu
        $role = if($processId -eq $RootProcessId) { $RootRole } elseif($RootRole -eq 'tunnel' -and $process.ProcessName -notin @('cloudflared','conhost','OpenConsole','cmd','powershell','pwsh')) { 'server' } else { 'auxiliary' }
        $rows += [pscustomobject]@{ sample=$sample; role=$role; private_bytes=$process.PrivateMemorySize64; working_set_bytes=$process.WorkingSet64; cpu_one_core_percent=100*$delta/$elapsed; interval_seconds=$elapsed }
        $privateTotal += $process.PrivateMemorySize64
        $workingTotal += $process.WorkingSet64
        $cpuTotal += $delta
    }
    $rows += [pscustomobject]@{ sample=$sample; role='total'; private_bytes=$privateTotal; working_set_bytes=$workingTotal; cpu_one_core_percent=100*$cpuTotal/$elapsed; interval_seconds=$elapsed }
    $clock.Restart()
}
ConvertTo-Json -InputObject $rows -Depth 4 -Compress
