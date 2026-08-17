[CmdletBinding()]
param([string] $Version = '0.1.0', [switch] $Elevated)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
if (-not $Elevated) {
  $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $PSCommandPath,
                 '-Version', $Version, '-Elevated')
  $process = Start-Process -FilePath (Get-Process -Id $PID).Path -ArgumentList $arguments `
    -Verb RunAs -Wait -PassThru
  if ($process.ExitCode -ne 0) { throw "Elevated installer smoke failed: $($process.ExitCode)" }
  Write-Host 'Installer install/repair/uninstall smoke passed and development registration restored.'
  exit 0
}

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw 'Installer smoke must run elevated.'
}

$installer = Join-Path $repoRoot "out/package/artifacts/fcitx5-windows-$Version-setup.exe"
$installRoot = Join-Path $repoRoot ('out/installer-smoke-' + [guid]::NewGuid().ToString('N'))
$restoreX64 = Join-Path $repoRoot 'out/build/windows-x64-dev/Debug/fcitx5-tsf.dll'
$restoreX86 = Join-Path $repoRoot 'out/build/windows-x86-dev/Debug/fcitx5-tsf.dll'
$registerX64 = Join-Path $repoRoot 'out/build/windows-x64-dev/Release/fcitx5-register.exe'
$registerX86 = Join-Path $repoRoot 'out/build/windows-x86-dev/Release/fcitx5-register.exe'

function Invoke-Checked([string] $File, [string[]] $Arguments) {
  $process = Start-Process -FilePath $File -ArgumentList $Arguments -Wait -PassThru `
    -WindowStyle Hidden
  if ($process.ExitCode -ne 0) { throw "$File failed with exit code $($process.ExitCode)." }
}

try {
  $setupArguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOICONS',
                      "/DIR=$installRoot", "/LOG=$installRoot-install.log")
  Invoke-Checked $installer $setupArguments
  $installedConfig = Join-Path $installRoot 'bin/fcitx5-config.exe'
  if (-not (Test-Path -LiteralPath $installedConfig -PathType Leaf)) {
    throw 'Installer did not create the expected Config artifact.'
  }
  Invoke-Checked (Join-Path $installRoot 'bin/fcitx5-register.exe') `
    @('--status', '--dll', (Join-Path $installRoot 'tsf/x64/fcitx5-tsf.dll'))
  Invoke-Checked (Join-Path $installRoot 'bin/fcitx5-register-x86.exe') `
    @('--status', '--dll', (Join-Path $installRoot 'tsf/x86/fcitx5-tsf.dll'))

  # Re-running the exact installer is the unattended repair path.
  Invoke-Checked $installer $setupArguments
  Invoke-Checked (Join-Path $installRoot 'unins000.exe') `
    @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
  if (Test-Path -LiteralPath $installedConfig -PathType Leaf) {
    throw 'Uninstall left the installed Config artifact behind.'
  }
} finally {
  if (Test-Path -LiteralPath $restoreX64 -PathType Leaf) {
    Invoke-Checked $registerX64 @('--register', '--dll', $restoreX64)
  }
  if (Test-Path -LiteralPath $restoreX86 -PathType Leaf) {
    Invoke-Checked $registerX86 @('--register', '--dll', $restoreX86)
  }
}
