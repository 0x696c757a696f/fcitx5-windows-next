[CmdletBinding()]
param(
  [switch] $Latest,
  [switch] $ReportOnly,
  [string[]] $Name
)

# Detect and (optionally) bump the pinned upstream commits for the fcitx5 core
# and addon sources recorded in `tools/bootstrap-fcitx.ps1` `$sourcePins`.
#
# The native Fcitx sources are NOT vendored into this repository; they are
# build-time checkouts. "Syncing" them means updating the `Commit = '...'`
# value in the `$sourcePins` array to the current upstream HEAD. Whether the
# pinned patch queue in `third_party/patches/` still applies against the new
# commit is verified separately by `tools/bootstrap-fcitx.ps1 -VerifyPatchesOnly`
# (which requires the pinned checkouts under `out/`); this script only updates
# the pin text and reports what changed, so the release/package gate remains the
# authority for patch compatibility.
#
#   -ReportOnly : print a JSON drift report, change nothing, exit 0.
#   -Latest      : bump every drifted pin to upstream HEAD (exit 0 if changed).
#   -Name <n>    : restrict to the named source(s).

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$bootstrap = Join-Path $repoRoot 'tools/bootstrap-fcitx.ps1'
if (-not (Test-Path -LiteralPath $bootstrap)) {
  throw "Missing bootstrap script: $bootstrap"
}
$bootstrapText = [System.IO.File]::ReadAllText($bootstrap)

# `$sourcePins` entries are single lines of the form:
#   @{ Name = '<n>'; Url = '<u>'; Commit = '<40-hex>' },
# Parse them with an anchored, line-scoped regex. Keep it simple: one line, one
# entry; the array is machine-maintained.
$entryPattern = "@\{\s*Name\s*=\s*'([^']+)'\s*;\s*Url\s*=\s*'([^']+)'\s*;\s*Commit\s*=\s*'([0-9a-f]{40})'\s*\}"
$entries = @()
foreach ($line in ($bootstrapText -split "`r?`n")) {
  $m = [regex]::Match($line, $entryPattern)
  if ($m.Success) {
    $entries += [pscustomobject]@{
      Name = $m.Groups[1].Value
      Url = $m.Groups[2].Value
      Commit = $m.Groups[3].Value
      Line = $line
    }
  }
}
if ($entries.Count -eq 0) {
  throw "Could not parse any `$sourcePins entries from $bootstrap"
}

$selected = @($entries | Where-Object {
  if (-not $Name -or $Name.Count -eq 0) { return $true }
  return $Name -contains $_.Name
})
if ($selected.Count -eq 0) {
  throw "No source pins matched the requested names: $($Name -join ', ')"
}

$report = @()
$changed = 0
foreach ($entry in $selected) {
  $remote = (& git.exe ls-remote $entry.Url HEAD 2>$null | Out-String).Trim()
  $head = if ($remote) { ($remote -split '\s+')[0] } else { '' }
  $drifted = ($head -and $head -ne $entry.Commit)
  $report += [pscustomobject]@{
    name = $entry.Name
    url = $entry.Url
    pinned = $entry.Commit
    upstream = $head
    drifted = [bool]$drifted
  }
  if ($drifted -and -not $ReportOnly) {
    $newLine = $entry.Line -replace [regex]::Escape($entry.Commit), $head
    $bootstrapText = $bootstrapText.Replace($entry.Line, $newLine)
    $changed++
  }
}

$reportJson = ($report | ConvertTo-Json -Depth 4)
$reportJson = $reportJson -replace "`r`n", "`n"
Write-Output $reportJson

if (-not $ReportOnly -and $changed -gt 0) {
  $normalized = $bootstrapText -replace "`r`n", "`n"
  if (-not $normalized.EndsWith("`n")) { $normalized += "`n" }
  [System.IO.File]::WriteAllText($bootstrap, $normalized, [System.Text.UTF8Encoding]::new($false))
  Write-Output "bumped $changed source pin(s)"
}
if ($ReportOnly -and $report.Count -gt 0 -and ($report | Where-Object drifted)) {
  exit 1
}
exit 0
