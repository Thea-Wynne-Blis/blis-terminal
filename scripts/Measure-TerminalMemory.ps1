[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateRange(1, [int]::MaxValue)]
    [int]$ProcessId,

    [ValidateRange(1, 60)]
    [int]$Samples = 5
)

for ($sample = 1; $sample -le $Samples; $sample++) {
    $processesByParent = Get-CimInstance Win32_Process -ErrorAction Stop |
        Group-Object -Property ParentProcessId -AsHashTable
    $root = Get-Process -Id $ProcessId -ErrorAction Stop
    $pending = [System.Collections.Generic.Queue[System.Diagnostics.Process]]::new()
    $pending.Enqueue($root)
    $descendants = while ($pending.Count -gt 0) {
        $parent = $pending.Dequeue()
        foreach ($entry in $processesByParent[[uint32]$parent.Id]) {
            $child = Get-Process -Id $entry.ProcessId -ErrorAction SilentlyContinue
            if ($child -and $child.StartTime -ge $parent.StartTime) {
                $pending.Enqueue($child)
                $child
            }
        }
    }
    $webview = @($descendants | Where-Object ProcessName -eq 'msedgewebview2')
    $tree = @($root) + @($descendants | Where-Object { $_ })

    [pscustomobject]@{
        Sample = $sample
        RootWorkingSetMiB = [math]::Round($root.WorkingSet64 / 1MB, 2)
        RootPrivateMiB = [math]::Round($root.PrivateMemorySize64 / 1MB, 2)
        WebView2Count = $webview.Count
        WebView2WorkingSetMiB = [math]::Round(($webview | Measure-Object WorkingSet64 -Sum).Sum / 1MB, 2)
        WebView2PrivateMiB = [math]::Round(($webview | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB, 2)
        TreeWorkingSetMiB = [math]::Round(($tree | Measure-Object WorkingSet64 -Sum).Sum / 1MB, 2)
        TreePrivateMiB = [math]::Round(($tree | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB, 2)
    }
    if ($sample -lt $Samples) { Start-Sleep -Seconds 1 }
}
