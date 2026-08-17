[CmdletBinding()]
param(
  [Parameter(Position = 0)]
  [ValidateSet('start', 'status', 'stop')]
  [string] $Action = 'start',

  [ValidateSet('Debug', 'Release')]
  [string] $Configuration = 'Debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$engine = [System.IO.Path]::GetFullPath(
  (Join-Path $repoRoot "out/build/windows-x64-dev/$Configuration/fcitx5-mock-engine.exe"))
$runtimeDirectory = Join-Path $repoRoot 'out/runtime'
$pidFile = Join-Path $runtimeDirectory 'phase1b-mock-engine.pid'

function Get-TrackedEngine {
  if (-not (Test-Path -LiteralPath $pidFile -PathType Leaf)) { return $null }
  $parsedPid = 0
  if (-not [int]::TryParse((Get-Content -LiteralPath $pidFile -Raw).Trim(), [ref] $parsedPid)) {
    return $null
  }
  $process = Get-Process -Id $parsedPid -ErrorAction SilentlyContinue
  if (-not $process) { return $null }
  $actualPath = [System.IO.Path]::GetFullPath($process.Path)
  if (-not [string]::Equals($actualPath, $engine, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Tracked PID $parsedPid belongs to a different executable: $actualPath"
  }
  return $process
}

$tracked = Get-TrackedEngine
if ($Action -eq 'status') {
  if ($tracked) {
    Write-Host "Mock engine is running (PID $($tracked.Id))."
  } else {
    Write-Host 'Mock engine is stopped.'
  }
  return
}

if ($Action -eq 'stop') {
  if ($tracked) {
    Stop-Process -Id $tracked.Id -ErrorAction Stop
    [void] $tracked.WaitForExit(2000)
    Write-Host "Mock engine stopped (PID $($tracked.Id))."
  } else {
    Write-Host 'Mock engine was not running.'
  }
  Remove-Item -LiteralPath $pidFile -Force -ErrorAction SilentlyContinue
  return
}

if ($tracked) {
  Write-Host "Mock engine is already running (PID $($tracked.Id))."
  return
}
if (-not (Test-Path -LiteralPath $engine -PathType Leaf)) {
  throw "Build the x64 mock engine first: $engine"
}
New-Item -ItemType Directory -Path $runtimeDirectory -Force | Out-Null
$process = Start-Process -FilePath $engine -PassThru -WindowStyle Hidden
Set-Content -LiteralPath $pidFile -Value $process.Id -Encoding ascii
Write-Host "Mock engine started (PID $($process.Id))."
