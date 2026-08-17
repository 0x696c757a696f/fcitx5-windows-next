[CmdletBinding()]
param(
  [Parameter(Position = 0)]
  [ValidateSet('register', 'unregister')]
  [string] $Action = 'register',

  [ValidateSet('Debug', 'Release')]
  [string] $Configuration = 'Debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw 'TSF profile registration requires an elevated PowerShell session. Re-run this script as Administrator.'
}
$x64Dll = Join-Path $repoRoot "out/build/windows-x64-dev/$Configuration/fcitx5-tsf.dll"
$x86Dll = Join-Path $repoRoot "out/build/windows-x86-dev/$Configuration/fcitx5-tsf.dll"
$x64Regsvr = Join-Path $env:SystemRoot 'System32/regsvr32.exe'
$x86Regsvr = Join-Path $env:SystemRoot 'SysWOW64/regsvr32.exe'
$clsid = '{3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}'
$classKeyPath = "Software\Classes\CLSID\$clsid\InprocServer32"

foreach ($path in @($x64Dll, $x86Dll, $x64Regsvr, $x86Regsvr)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Required registration artifact is missing: $path"
  }
}

function Invoke-Regsvr {
  param(
    [Parameter(Mandatory)] [string] $Executable,
    [Parameter(Mandatory)] [string] $Dll,
    [Parameter(Mandatory)] [bool] $Unregister
  )

  $arguments = if ($Unregister) { @('/s', '/u', $Dll) } else { @('/s', $Dll) }
  $process = Start-Process -FilePath $Executable -ArgumentList $arguments -Wait -PassThru -WindowStyle Hidden
  if ($process.ExitCode -ne 0) {
    throw "regsvr32 failed for $Dll with exit code $($process.ExitCode)"
  }
}

function Get-ComServerPath {
  param([Parameter(Mandatory)] [Microsoft.Win32.RegistryView] $View)

  $baseKey = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
    [Microsoft.Win32.RegistryHive]::LocalMachine,
    $View
  )
  try {
    $key = $baseKey.OpenSubKey($classKeyPath)
    if (-not $key) { return $null }
    try {
      return [string] $key.GetValue($null, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    } finally {
      $key.Dispose()
    }
  } finally {
    $baseKey.Dispose()
  }
}

function Assert-ComRegistration {
  param([Parameter(Mandatory)] [bool] $Present)

  $registrations = @(
    [pscustomobject]@{ Name = 'x64'; View = [Microsoft.Win32.RegistryView]::Registry64; Expected = $x64Dll }
    [pscustomobject]@{ Name = 'x86'; View = [Microsoft.Win32.RegistryView]::Registry32; Expected = $x86Dll }
  )
  foreach ($registration in $registrations) {
    $actual = Get-ComServerPath -View $registration.View
    if ($Present -and -not [string]::Equals(
        $actual,
        $registration.Expected,
        [StringComparison]::OrdinalIgnoreCase
      )) {
      throw "$($registration.Name) COM registration mismatch. Expected '$($registration.Expected)', found '$actual'."
    }
    if (-not $Present -and $actual) {
      throw "$($registration.Name) COM registration remains at '$actual'."
    }
  }
}

if ($Action -eq 'register') {
  $targets = @(
    [pscustomobject]@{ Executable = $x64Regsvr; Dll = $x64Dll }
    [pscustomobject]@{ Executable = $x86Regsvr; Dll = $x86Dll }
  )
  $completed = [Collections.Generic.List[object]]::new()
  try {
    foreach ($target in $targets) {
      Invoke-Regsvr -Executable $target.Executable -Dll $target.Dll -Unregister $false
      $completed.Add($target)
    }
    Assert-ComRegistration -Present $true
  } catch {
    for ($index = $completed.Count - 1; $index -ge 0; --$index) {
      try {
        Invoke-Regsvr -Executable $completed[$index].Executable -Dll $completed[$index].Dll -Unregister $true
      } catch {
        Write-Warning "Registration rollback failed for $($completed[$index].Dll): $_"
      }
    }
    throw
  }
} else {
  $unregisterErrors = [Collections.Generic.List[string]]::new()
  foreach ($target in @(
      [pscustomobject]@{ Executable = $x64Regsvr; Dll = $x64Dll }
      [pscustomobject]@{ Executable = $x86Regsvr; Dll = $x86Dll }
    )) {
    try {
      Invoke-Regsvr -Executable $target.Executable -Dll $target.Dll -Unregister $true
    } catch {
      $unregisterErrors.Add($_.Exception.Message)
    }
  }
  Assert-ComRegistration -Present $false
  foreach ($message in $unregisterErrors) {
    Write-Warning $message
  }
}

Write-Host "Developer TSF $Action completed for x64 and x86."
