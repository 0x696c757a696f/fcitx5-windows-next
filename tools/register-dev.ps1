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

$sourceX64Dll = Join-Path $repoRoot "out/build/windows-x64-dev/$Configuration/fcitx5-tsf.dll"
$sourceX86Dll = Join-Path $repoRoot "out/build/windows-x86-dev/$Configuration/fcitx5-tsf.dll"
$x64Regsvr = Join-Path $env:SystemRoot 'System32/regsvr32.exe'
$x86Regsvr = Join-Path $env:SystemRoot 'SysWOW64/regsvr32.exe'
$clsid = '{3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}'
$classRootPath = "Software\Classes\CLSID\$clsid"
$classKeyPath = "$classRootPath\InprocServer32"

foreach ($path in @($x64Regsvr, $x86Regsvr)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Required registration tool is missing: $path"
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

function Assert-PathUnderRoot {
  param(
    [Parameter(Mandatory)] [string] $Path,
    [Parameter(Mandatory)] [string[]] $AllowedRoots
  )

  $resolvedPath = [System.IO.Path]::GetFullPath($Path)
  foreach ($root in $AllowedRoots) {
    $resolvedRoot = [System.IO.Path]::GetFullPath($root).TrimEnd('\') + '\'
    if ($resolvedPath.StartsWith($resolvedRoot, [StringComparison]::OrdinalIgnoreCase)) {
      return
    }
  }
  throw "Refusing to unregister a COM server outside this repository's development paths: $Path"
}

function Remove-StaleComRegistration {
  param(
    [Parameter(Mandatory)] [string] $Name,
    [Parameter(Mandatory)] [Microsoft.Win32.RegistryView] $View,
    [Parameter(Mandatory)] [string] $Dll,
    [Parameter(Mandatory)] [string[]] $AllowedRoots
  )

  Assert-PathUnderRoot -Path $Dll -AllowedRoots $AllowedRoots
  if (Test-Path -LiteralPath $Dll -PathType Leaf) {
    throw "$Name registered DLL exists and must self-unregister: $Dll"
  }

  $baseKey = [Microsoft.Win32.RegistryKey]::OpenBaseKey(
    [Microsoft.Win32.RegistryHive]::LocalMachine,
    $View
  )
  try {
    $classes = $baseKey.OpenSubKey('Software\Classes\CLSID', $true)
    if (-not $classes) {
      throw "$Name Classes\\CLSID registry root is unavailable."
    }
    try {
      $classes.DeleteSubKeyTree($clsid, $false)
    } finally {
      $classes.Dispose()
    }
  } finally {
    $baseKey.Dispose()
  }
  Write-Host "$Name registered DLL is missing; removing stale COM registration: $Dll"
}

function Assert-ComRegistration {
  param(
    [Parameter(Mandatory)] [string] $ExpectedX64,
    [Parameter(Mandatory)] [string] $ExpectedX86
  )

  foreach ($registration in @(
      [pscustomobject]@{ Name = 'x64'; View = [Microsoft.Win32.RegistryView]::Registry64; Expected = $ExpectedX64 }
      [pscustomobject]@{ Name = 'x86'; View = [Microsoft.Win32.RegistryView]::Registry32; Expected = $ExpectedX86 }
    )) {
    $actual = Get-ComServerPath -View $registration.View
    if (-not [string]::Equals(
        $actual,
        $registration.Expected,
        [StringComparison]::OrdinalIgnoreCase
      )) {
      throw "$($registration.Name) COM registration mismatch. Expected '$($registration.Expected)', found '$actual'."
    }
  }
}

if ($Action -eq 'register') {
  foreach ($path in @($sourceX64Dll, $sourceX86Dll)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
      throw "Required registration artifact is missing: $path"
    }
  }

  $deploymentId = '{0:yyyyMMdd-HHmmss}-{1}' -f (Get-Date).ToUniversalTime(), ([guid]::NewGuid().ToString('N'))
  $deploymentRoot = Join-Path $repoRoot "out/dev-registration/$deploymentId"
  $x64Dll = Join-Path $deploymentRoot 'x64/fcitx5-tsf.dll'
  $x86Dll = Join-Path $deploymentRoot 'x86/fcitx5-tsf.dll'
  New-Item -ItemType Directory -Path (Split-Path -Parent $x64Dll), (Split-Path -Parent $x86Dll) | Out-Null
  Copy-Item -LiteralPath $sourceX64Dll -Destination $x64Dll
  Copy-Item -LiteralPath $sourceX86Dll -Destination $x86Dll

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
    Assert-ComRegistration -ExpectedX64 $x64Dll -ExpectedX86 $x86Dll
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
  Write-Host "Developer TSF registration completed from isolated deployment '$deploymentRoot'."
} else {
  $allowedRoots = @(
    (Join-Path $repoRoot 'out/dev-registration'),
    (Join-Path $repoRoot 'out/build/windows-x64-dev'),
    (Join-Path $repoRoot 'out/build/windows-x86-dev'),
    (Join-Path $repoRoot 'out/package')
  )
  $targets = @(
    [pscustomobject]@{
      Name = 'x64'
      View = [Microsoft.Win32.RegistryView]::Registry64
      Executable = $x64Regsvr
      Dll = Get-ComServerPath -View ([Microsoft.Win32.RegistryView]::Registry64
      )
    }
    [pscustomobject]@{
      Name = 'x86'
      View = [Microsoft.Win32.RegistryView]::Registry32
      Executable = $x86Regsvr
      Dll = Get-ComServerPath -View ([Microsoft.Win32.RegistryView]::Registry32
      )
    }
  )
  foreach ($target in $targets) {
    if (-not $target.Dll) { continue }
    Assert-PathUnderRoot -Path $target.Dll -AllowedRoots $allowedRoots
    if (-not (Test-Path -LiteralPath $target.Dll -PathType Leaf)) {
      Remove-StaleComRegistration -Name $target.Name -View $target.View `
        -Dll $target.Dll -AllowedRoots $allowedRoots
      $remaining = Get-ComServerPath -View $target.View
      if ($remaining) {
        throw "$($target.Name) stale COM registration remains at '$remaining'."
      }
      continue
    }
    Invoke-Regsvr -Executable $target.Executable -Dll $target.Dll -Unregister $true
    $remaining = Get-ComServerPath -View $target.View
    if ($remaining) {
      throw "$($target.Name) COM registration remains at '$remaining'."
    }
  }
  Write-Host 'Developer TSF unregistration completed for x64 and x86.'
}
