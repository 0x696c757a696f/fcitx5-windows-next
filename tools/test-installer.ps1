[CmdletBinding()]
param(
  [string] $Version = '0.1.0',
  [string] $InstallerPath,
  [string] $ErrorLog,
  [switch] $Elevated
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
if (-not $Elevated) {
  foreach ($running in @(Get-Process fcitx5-launcher -ErrorAction SilentlyContinue)) {
    try {
      $runningControl = Join-Path (Split-Path -Parent $running.Path) 'fcitx5-control.exe'
      if (Test-Path -LiteralPath $runningControl -PathType Leaf) {
        & $runningControl --shutdown | Out-Null
      }
    } catch {}
  }
  $stopDeadline = [Environment]::TickCount64 + 10000
  do {
    $userPlane = @(Get-Process fcitx5-launcher, fcitx5-engine, fcitx5-ui `
      -ErrorAction SilentlyContinue)
    if ($userPlane.Count -eq 0) { break }
    Start-Sleep -Milliseconds 100
  } while ([Environment]::TickCount64 -lt $stopDeadline)
  foreach ($running in $userPlane) {
    try { Stop-Process -Id $running.Id -Force } catch {}
  }
  if (Get-Process fcitx5-launcher, fcitx5-engine, fcitx5-ui `
      -ErrorAction SilentlyContinue) {
    throw 'Fcitx5 user-plane processes could not be stopped before elevation.'
  }
  $evidenceRoot = Join-Path $repoRoot 'out/evidence'
  New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null
  $childError = Join-Path $evidenceRoot 'installer-smoke.stderr.log'
  Remove-Item -LiteralPath $childError -Force -ErrorAction SilentlyContinue
  $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $PSCommandPath,
                 '-Version', $Version, '-ErrorLog', $childError, '-Elevated')
  if (-not [string]::IsNullOrWhiteSpace($InstallerPath)) {
    $arguments += @('-InstallerPath', $InstallerPath)
  }
  $process = Start-Process -FilePath (Get-Process -Id $PID).Path `
    -ArgumentList $arguments -Verb RunAs -WindowStyle Hidden -PassThru
  if (-not $process.WaitForExit(300000)) {
    throw 'Elevated installer smoke did not exit within 300 seconds.'
  }
  # A persistent user-plane process must never inherit the elevated test token. Restore the
  # launcher from the registration that the child put back, after elevation has ended.
  $textServiceClsid = '{3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}'
  $registration = "Registry::HKEY_CLASSES_ROOT\CLSID\$textServiceClsid\InprocServer32"
  if (-not (Get-Process fcitx5-launcher -ErrorAction SilentlyContinue) -and
      (Test-Path -LiteralPath $registration)) {
    $registeredDll = (Get-Item -LiteralPath $registration).GetValue('')
    if (-not [string]::IsNullOrWhiteSpace($registeredDll)) {
      $registeredRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $registeredDll))
      $userLauncher = Join-Path $registeredRoot 'bin/fcitx5-launcher.exe'
      if (Test-Path -LiteralPath $userLauncher -PathType Leaf) {
        Start-Process -FilePath $userLauncher -ArgumentList '--background' | Out-Null
      }
    }
  }
  if ($process.ExitCode -ne 0) {
    $detail = if (Test-Path -LiteralPath $childError) {
      [IO.File]::ReadAllText($childError).Trim()
    } else { '' }
    throw "Elevated installer smoke failed: $($process.ExitCode). $detail"
  }
  Write-Host 'Installer install/repair/uninstall smoke passed and prior state was restored.'
  exit 0
}
trap {
  if (-not [string]::IsNullOrWhiteSpace($ErrorLog)) {
    [IO.File]::WriteAllText([IO.Path]::GetFullPath($ErrorLog), ($_ | Out-String),
      [Text.UTF8Encoding]::new($false))
  }
  exit 1
}
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
$isAdministrator = $principal.IsInRole(
  [Security.Principal.WindowsBuiltInRole]::Administrator)

$installer = if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
  Join-Path $repoRoot "out/package/artifacts/fcitx5-windows-$Version-setup.exe"
} else {
  [IO.Path]::GetFullPath($InstallerPath)
}
if (-not (Test-Path -LiteralPath $installer -PathType Leaf)) {
  throw "Installer smoke input is missing: $installer"
}
$installRoot = Join-Path $repoRoot ('out/installer-smoke-' + [guid]::NewGuid().ToString('N'))
$registerX64 = Join-Path $repoRoot 'out/build/windows-x64-dev/Release/fcitx5-register.exe'
$registerX86 = Join-Path $repoRoot 'out/build/windows-x86-dev/Release/fcitx5-register.exe'
$textServiceClsid = '{3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}'
$registrationSnapshot = @(
  [pscustomobject]@{
    Register = $registerX64
    Registry = "Registry::HKEY_CLASSES_ROOT\CLSID\$textServiceClsid\InprocServer32"
    Dll = $null
  },
  [pscustomobject]@{
    Register = $registerX86
    Registry = "Registry::HKEY_LOCAL_MACHINE\Software\Classes\WOW6432Node\CLSID\$textServiceClsid\InprocServer32"
    Dll = $null
  }
)
foreach ($item in $registrationSnapshot) {
  if (Test-Path -LiteralPath $item.Registry) {
    $value = (Get-Item -LiteralPath $item.Registry).GetValue('')
    if (-not [string]::IsNullOrWhiteSpace($value) -and
        (Test-Path -LiteralPath $value -PathType Leaf)) {
      $item.Dll = [IO.Path]::GetFullPath($value)
    }
  }
}
$runKey = 'Registry::HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run'
$runValueName = 'Fcitx5-Stable'
$startupExisted = $false
$startupValue = $null
if (Test-Path -LiteralPath $runKey) {
  $startupValue = (Get-Item -LiteralPath $runKey).GetValue($runValueName, $null)
  $startupExisted = $null -ne $startupValue
}
$launcherSnapshot = @(Get-Process fcitx5-launcher -ErrorAction SilentlyContinue |
  ForEach-Object {
    try { [IO.Path]::GetFullPath($_.Path) } catch { $null }
  } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
$restoreBootstrap = $null
if ($registrationSnapshot[0].Dll -and $registrationSnapshot[1].Dll) {
  $x64Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $registrationSnapshot[0].Dll))
  $x86Root = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $registrationSnapshot[1].Dll))
  if ($x64Root -eq $x86Root -and
      (Test-Path -LiteralPath (Join-Path $x64Root 'Start Fcitx5.exe') -PathType Leaf)) {
    $restoreBootstrap = Join-Path $x64Root 'Start Fcitx5.exe'
  }
}

function Invoke-Checked([string] $File, [string[]] $Arguments) {
  $process = Start-Process -FilePath $File -ArgumentList $Arguments -PassThru `
    -WindowStyle Hidden
  if (-not $process.WaitForExit(120000)) {
    try { Stop-Process -Id $process.Id -Force } catch {}
    throw "$File did not exit within 120 seconds."
  }
  if ($process.ExitCode -ne 0) { throw "$File failed with exit code $($process.ExitCode)." }
}

function Invoke-ElevatedChecked([string] $File, [string[]] $Arguments) {
  $parameters = @{
    FilePath = $File
    ArgumentList = $Arguments
    PassThru = $true
    WindowStyle = 'Hidden'
  }
  if (-not $isAdministrator) { $parameters.Verb = 'RunAs' }
  $process = Start-Process @parameters
  if (-not $process.WaitForExit(180000)) {
    throw "$File did not exit within 180 seconds."
  }
  if ($process.ExitCode -ne 0) {
    throw "$File failed with elevated exit code $($process.ExitCode)."
  }
}

try {
  foreach ($runningLauncher in $launcherSnapshot) {
    $runningControl = Join-Path (Split-Path -Parent $runningLauncher) 'fcitx5-control.exe'
    if (Test-Path -LiteralPath $runningControl -PathType Leaf) {
      try { Invoke-Checked $runningControl @('--shutdown') } catch {}
    }
  }
  $deadline = [Environment]::TickCount64 + 10000
  do {
    $remaining = @(Get-Process fcitx5-launcher, fcitx5-engine, fcitx5-ui `
      -ErrorAction SilentlyContinue)
    if ($remaining.Count -eq 0) { break }
    Start-Sleep -Milliseconds 100
  } while ([Environment]::TickCount64 -lt $deadline)
  foreach ($process in $remaining) {
    Stop-Process -Id $process.Id -Force
  }
  Start-Sleep -Milliseconds 250
  $stillRunning = @(Get-Process fcitx5-launcher, fcitx5-engine, fcitx5-ui `
    -ErrorAction SilentlyContinue)
  if ($stillRunning.Count -ne 0) {
    throw 'Workspace Fcitx5 processes did not stop before installer verification.'
  }

  $setupArguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOICONS',
                      '/NOCLOSEAPPLICATIONS', '/NORESTARTAPPLICATIONS',
                      "/DIR=$installRoot", "/LOG=$installRoot-install.log")
  Invoke-ElevatedChecked $installer $setupArguments
  $installedConfig = Join-Path $installRoot 'bin/fcitx5-config.exe'
  if (-not (Test-Path -LiteralPath $installedConfig -PathType Leaf)) {
    throw 'Installer did not create the expected Config artifact.'
  }
  Invoke-Checked (Join-Path $installRoot 'bin/fcitx5-register.exe') `
    @('--status', '--dll', (Join-Path $installRoot 'tsf/x64/fcitx5-tsf.dll'))
  Invoke-Checked (Join-Path $installRoot 'bin/fcitx5-register-x86.exe') `
    @('--status', '--dll', (Join-Path $installRoot 'tsf/x86/fcitx5-tsf.dll'))

  # Re-running the exact installer is the unattended repair path.
  Invoke-Checked (Join-Path $installRoot 'bin/fcitx5-control.exe') @('--shutdown')
  Invoke-ElevatedChecked $installer $setupArguments
  Invoke-Checked (Join-Path $installRoot 'bin/fcitx5-config.exe') `
    @('--ui-interaction-test')
  Invoke-ElevatedChecked (Join-Path $installRoot 'unins000.exe') `
    @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
  if (Test-Path -LiteralPath $installedConfig -PathType Leaf) {
    throw 'Uninstall left the installed Config artifact behind.'
  }
} finally {
  $resolvedInstallRoot = [IO.Path]::GetFullPath($installRoot)
  foreach ($process in @(Get-Process fcitx5-launcher, fcitx5-engine -ErrorAction SilentlyContinue)) {
    try {
      if ([IO.Path]::GetFullPath($process.Path).StartsWith(
          $resolvedInstallRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
        Stop-Process -Id $process.Id -Force
      }
    } catch {}
  }
  if ($restoreBootstrap) {
    Invoke-ElevatedChecked $restoreBootstrap @('--elevated-register')
  } else {
    foreach ($item in $registrationSnapshot) {
      if (-not [string]::IsNullOrWhiteSpace($item.Dll)) {
        Invoke-ElevatedChecked $item.Register @('--register', '--dll', $item.Dll)
      }
    }
  }
  if ($startupExisted) {
    New-Item -Path $runKey -Force | Out-Null
    New-ItemProperty -LiteralPath $runKey -Name $runValueName -Value $startupValue `
      -PropertyType String -Force | Out-Null
  } elseif (Test-Path -LiteralPath $runKey) {
    Remove-ItemProperty -LiteralPath $runKey -Name $runValueName -ErrorAction SilentlyContinue
  }
}
$installerScratchRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out'))
$scratchPrefix = $installerScratchRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) +
  [IO.Path]::DirectorySeparatorChar
if (-not $resolvedInstallRoot.StartsWith(
    $scratchPrefix, [StringComparison]::OrdinalIgnoreCase) -or
    [IO.Path]::GetFileName($resolvedInstallRoot) -notlike 'installer-smoke-*') {
  throw "Refusing to clean unexpected installer smoke path: $resolvedInstallRoot"
}
if (Test-Path -LiteralPath $resolvedInstallRoot) {
  Remove-Item -LiteralPath $resolvedInstallRoot -Recurse -Force
}
$installLog = "$resolvedInstallRoot-install.log"
if (Test-Path -LiteralPath $installLog -PathType Leaf) {
  Remove-Item -LiteralPath $installLog -Force
}
Write-Host 'Installer install/repair/uninstall smoke passed and prior registration restored.'
