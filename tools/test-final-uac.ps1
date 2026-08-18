[CmdletBinding()]
param(
  [string] $Version = '0.1.0',
  [switch] $Elevated,
  [string] $ErrorLog
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$stagePointer = Join-Path $repoRoot 'out/package/current-stage.txt'
$installer = Join-Path $repoRoot "out/package/artifacts/fcitx5-windows-$Version-setup.exe"
$evidenceRoot = Join-Path $repoRoot 'out/evidence'

function Get-CurrentStageRoot {
  if (-not (Test-Path -LiteralPath $stagePointer -PathType Leaf)) {
    throw 'No tested package stage is selected. Run the package gate first.'
  }
  $selected = [IO.Path]::GetFullPath(([IO.File]::ReadAllText($stagePointer).Trim()))
  $root = if (Test-Path -LiteralPath (Join-Path $selected 'Start Fcitx5.exe')) {
    $selected
  } else {
    Join-Path $selected 'Fcitx5'
  }
  if (-not (Test-Path -LiteralPath (Join-Path $root 'Start Fcitx5.exe') -PathType Leaf)) {
    throw "The selected package stage has no bootstrap entry point: $root"
  }
  return $root
}

if ($Elevated) {
  trap {
    if (-not [string]::IsNullOrWhiteSpace($ErrorLog)) {
      [IO.File]::WriteAllText([IO.Path]::GetFullPath($ErrorLog), ($_ | Out-String),
        [Text.UTF8Encoding]::new($false))
    }
    exit 1
  }
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = [Security.Principal.WindowsPrincipal]::new($identity)
  if (-not $principal.IsInRole(
      [Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'The final administrator verification did not receive an elevated token.'
  }
  if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
    throw "The tested installer is missing: $installer"
  }
  $stageRoot = Get-CurrentStageRoot
  & (Join-Path $PSScriptRoot 'test-installer.ps1') -Version $Version `
    -InstallerPath $installer -ErrorLog $ErrorLog -Elevated
  $bootstrap = Join-Path $stageRoot 'Start Fcitx5.exe'
  $registration = Start-Process -FilePath $bootstrap -ArgumentList '--elevated-register' `
    -WindowStyle Hidden -Wait -PassThru
  if ($registration.ExitCode -ne 0) {
    throw "Exact-stage dual-architecture TSF registration failed: $($registration.ExitCode)."
  }
  Write-Host 'Elevated installer lifecycle and exact-stage TSF registration passed.'
  exit 0
}

New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null
$childError = Join-Path $evidenceRoot 'final-uac.stderr.log'
Remove-Item -LiteralPath $childError -Force -ErrorAction SilentlyContinue

foreach ($running in @(Get-Process fcitx5-launcher -ErrorAction SilentlyContinue)) {
  try {
    $control = Join-Path (Split-Path -Parent $running.Path) 'fcitx5-control.exe'
    if (Test-Path -LiteralPath $control -PathType Leaf) {
      & $control --shutdown | Out-Null
    }
  } catch {}
}
$deadline = [Environment]::TickCount64 + 10000
do {
  $userPlane = @(Get-Process fcitx5-launcher, fcitx5-engine, fcitx5-ui `
    -ErrorAction SilentlyContinue)
  if ($userPlane.Count -eq 0) { break }
  Start-Sleep -Milliseconds 100
} while ([Environment]::TickCount64 -lt $deadline)
if ($userPlane.Count -ne 0) {
  throw 'Fcitx5 user-plane processes did not stop before the final UAC verification.'
}

$arguments = @('-NoLogo', '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
               $PSCommandPath, '-Version', $Version, '-Elevated', '-ErrorLog', $childError)
$administrator = Start-Process -FilePath 'D:\Program Files\PowerShell\7\pwsh.exe' `
  -ArgumentList $arguments -Verb RunAs -WindowStyle Hidden -PassThru
if (-not $administrator.WaitForExit(600000)) {
  throw 'The final elevated verification exceeded its ten-minute deadline.'
}
if ($administrator.ExitCode -ne 0) {
  $detail = if (Test-Path -LiteralPath $childError -PathType Leaf) {
    [IO.File]::ReadAllText($childError).Trim()
  } else { '' }
  throw "Final elevated verification failed: $($administrator.ExitCode). $detail"
}

& (Join-Path $PSScriptRoot 'test-desktop.ps1') -Configuration Release
Write-Host 'One-UAC installer, TSF, tray, Config, and real typing verification passed.'
