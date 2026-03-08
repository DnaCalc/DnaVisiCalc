param(
    [Parameter(Mandatory = $true)]
    [string]$TraceLogPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputPrefix
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (-not (Test-Path -Path $TraceLogPath)) {
    throw "Trace log not found: $TraceLogPath"
}

$events = @()
foreach ($line in Get-Content -Path $TraceLogPath) {
    if ($line -match '^fec_f3e\s+(?:trace_version=[^\s]+\s+)?seq=(\d+)\s+event=([^\s]+)') {
        $events += [pscustomobject]@{
            Seq = [int64]$matches[1]
            Event = $matches[2]
        }
    }
}

$events = $events | Sort-Object -Property Seq
$outDir = Split-Path -Parent $OutputPrefix
if ($outDir -and -not (Test-Path -Path $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$counts = @{}
foreach ($item in $events) {
    if (-not $counts.ContainsKey($item.Event)) {
        $counts[$item.Event] = 0
    }
    $counts[$item.Event]++
}

$countRows = $counts.GetEnumerator() | Sort-Object -Property Value -Descending
$countPath = "$OutputPrefix.event_counts.tsv"
"event`tcount" | Set-Content -Path $countPath -Encoding UTF8
foreach ($row in $countRows) {
    "$($row.Key)`t$($row.Value)" | Add-Content -Path $countPath -Encoding UTF8
}

$edgeCounts = @{}
for ($i = 1; $i -lt $events.Count; $i++) {
    $from = $events[$i - 1].Event
    $to = $events[$i].Event
    $key = "$from|$to"
    if (-not $edgeCounts.ContainsKey($key)) {
        $edgeCounts[$key] = 0
    }
    $edgeCounts[$key]++
}

$edgeRows = foreach ($entry in $edgeCounts.GetEnumerator() | Sort-Object -Property Name) {
    $parts = $entry.Key.Split("|", 2)
    [pscustomobject]@{
        from = $parts[0]
        to = $parts[1]
        count = $entry.Value
    }
}

$edgePath = "$OutputPrefix.callgraph.edges.csv"
$edgeRows | Export-Csv -Path $edgePath -NoTypeInformation -Encoding UTF8

$dotPath = "$OutputPrefix.callgraph.dot"
"digraph fec_f3e {" | Set-Content -Path $dotPath -Encoding UTF8
foreach ($edge in $edgeRows) {
    "  `"$($edge.from)`" -> `"$($edge.to)`" [label=`"$($edge.count)`"];" | Add-Content -Path $dotPath -Encoding UTF8
}
"}" | Add-Content -Path $dotPath -Encoding UTF8

Write-Host "Derived artifacts:"
Write-Host " - $countPath"
Write-Host " - $edgePath"
Write-Host " - $dotPath"
