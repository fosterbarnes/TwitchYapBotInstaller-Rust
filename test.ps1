Write-Host "=== Searching for YapBot-related WMI processes ==="

# Find PowerShell processes with WMI-related command lines
$wmiProcesses = @()
Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe'" | ForEach-Object {
    $process = $_
    if ($process.CommandLine -like "*ManagementEventWatcher*" -or 
        $process.CommandLine -like "*WMI*" -or 
        $process.CommandLine -like "*OBS*" -or
        $process.CommandLine -like "*TwitchYapBot*" -or
        $process.CommandLine -like "*YapBot*") {
        $wmiProcesses += $process
    }
}

if ($wmiProcesses.Count -gt 0) {
    Write-Host "Found $($wmiProcesses.Count) WMI-related PowerShell processes:"
    $wmiProcesses | ForEach-Object {
        Write-Host "PID: $($_.ProcessId), Command: $($_.CommandLine)"
    }
} else {
    Write-Host "No WMI-related PowerShell processes found."
}

# Check for WMI event subscribers
Write-Host "`n=== WMI Event Subscribers ==="
$subscribers = @(Get-EventSubscriber -ErrorAction SilentlyContinue)
if ($subscribers.Count -gt 0) {
    Write-Host "Found $($subscribers.Count) event subscribers:"
    $subscribers | ForEach-Object {
        Write-Host "Event: $($_.EventName), Source: $($_.SourceObject)"
    }
} else {
    Write-Host "No event subscribers found."
}

# Check for WMI event consumers
Write-Host "`n=== WMI Event Consumers ==="
$consumers = @(Get-CimInstance -Class __EventConsumer -ErrorAction SilentlyContinue)
if ($consumers.Count -gt 0) {
    Write-Host "Found $($consumers.Count) WMI event consumers:"
    $consumers | ForEach-Object {
        Write-Host "Consumer: $($_.Name), Class: $($_.__CLASS)"
    }
} else {
    Write-Host "No WMI event consumers found."
}

# Check for WMI event filters
Write-Host "`n=== WMI Event Filters ==="
$filters = @(Get-CimInstance -Class __EventFilter -ErrorAction SilentlyContinue)
if ($filters.Count -gt 0) {
    Write-Host "Found $($filters.Count) WMI event filters:"
    $filters | ForEach-Object {
        Write-Host "Filter: $($_.Name), Query: $($_.Query)"
    }
} else {
    Write-Host "No WMI event filters found."
}