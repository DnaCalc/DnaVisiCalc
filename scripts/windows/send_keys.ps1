param(
    [string]$WindowTitle,
    [int]$ProcessId = 0,
    [Parameter(Mandatory = $true)]
    [string[]]$Keys,
    [int]$DelayMs = 120,
    [int]$ActivationTimeoutMs = 6000
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$shell = New-Object -ComObject WScript.Shell

$activateTarget = if ($ProcessId -gt 0) { $ProcessId } else { $WindowTitle }
if (($ProcessId -le 0) -and [string]::IsNullOrWhiteSpace($WindowTitle)) {
    throw "Provide either -ProcessId or -WindowTitle."
}

$activated = $false
$deadline = (Get-Date).AddMilliseconds($ActivationTimeoutMs)
while ((Get-Date) -lt $deadline) {
    if ($shell.AppActivate($activateTarget)) {
        $activated = $true
        break
    }
    Start-Sleep -Milliseconds 100
}

if (-not $activated) {
    if ($ProcessId -gt 0) {
        throw "Could not activate process id '$ProcessId'."
    }
    throw "Could not activate window titled '$WindowTitle'."
}

Start-Sleep -Milliseconds 200
foreach ($key in $Keys) {
    $shell.SendKeys($key)
    Start-Sleep -Milliseconds $DelayMs
}
