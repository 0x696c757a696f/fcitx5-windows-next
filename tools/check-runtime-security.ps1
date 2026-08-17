[CmdletBinding()]
param(
  [ValidateSet('x64', 'x86')]
  [string] $Architecture = 'x64',

  [ValidateSet('Debug', 'Release')]
  [string] $Configuration = 'Debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
  throw 'Visual Studio vswhere.exe is required for the PE import audit.'
}
$installation = (& $vswhere -latest -products * `
  -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 `
  -property installationPath | Select-Object -First 1)
if (-not $installation) { throw 'Visual Studio C++ tools were not found.' }
$dumpbin = Get-ChildItem -LiteralPath (Join-Path $installation 'VC/Tools/MSVC') `
  -Filter dumpbin.exe -File -Recurse |
  Where-Object { $_.FullName -match 'Hostx64\\x64\\dumpbin\.exe$' } |
  Sort-Object FullName -Descending |
  Select-Object -First 1 -ExpandProperty FullName
if (-not $dumpbin) { throw 'dumpbin.exe was not found in the pinned Visual Studio toolset.' }

$binaryRoot = Join-Path $repoRoot "out/build/windows-$Architecture-dev/$Configuration"
$binaries = @(
  (Join-Path $binaryRoot 'fcitx5-tsf.dll'),
  (Join-Path $binaryRoot 'fcitx5-launcher.exe'),
  (Join-Path $binaryRoot 'fcitx5-ui.exe')
)
if ($Architecture -eq 'x64') {
  $nativeEngine = Join-Path $repoRoot 'out/stage/fcitx5/bin/fcitx5-engine.exe'
  if (Test-Path -LiteralPath $nativeEngine -PathType Leaf) { $binaries += $nativeEngine }
}

$postWin7Imports = @(
  'AddDllDirectory',
  'AdjustWindowRectExForDpi',
  'GetCurrentPackageFamilyName',
  'GetDpiForSystem',
  'GetDpiForWindow',
  'GetProcessInformation',
  'GetSystemMetricsForDpi',
  'GetSystemTimePreciseAsFileTime',
  'GetThreadDescription',
  'IsWow64Process2',
  'SetDefaultDllDirectories',
  'SetProcessDpiAwarenessContext',
  'SetThreadDescription'
)
foreach ($binary in $binaries) {
  if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
    throw "Required binary is missing for PE audit: $binary"
  }
  $imports = (& $dumpbin /nologo /imports $binary) -join "`n"
  if ($LASTEXITCODE -ne 0) { throw "dumpbin failed for $binary" }
  foreach ($name in $postWin7Imports) {
    if ($imports -match "(?m)^\s+[0-9A-F]+\s+$([regex]::Escape($name))\s*$") {
      throw "Win7-incompatible hard import '$name' found in $binary"
    }
  }
  if ([IO.Path]::GetFileName($binary) -eq 'fcitx5-tsf.dll' -and
      $imports -match '(?im)^\s+(WINHTTP|WININET|WS2_32|URLMON)\.dll\s*$') {
    throw "Network-capable library imported by thin TSF DLL: $binary"
  }
}

$sourceFiles = Get-ChildItem -LiteralPath (Join-Path $repoRoot 'src') -File -Recurse `
  -Include *.cpp,*.h
$prohibited = @(
  '\bSetWindowsHookEx[AW]?\s*\(',
  '\bSendInput\s*\(',
  '\bWriteProcessMemory\s*\(',
  '\bCreateRemoteThread(?:Ex)?\s*\(',
  '\bVirtualAllocEx\s*\(',
  '\bPROCESS_VM_(?:READ|WRITE|OPERATION)\b'
)
foreach ($source in $sourceFiles) {
  $text = Get-Content -LiteralPath $source.FullName -Raw
  foreach ($pattern in $prohibited) {
    if ($text -match $pattern) {
      throw "Prohibited Hook/injection/game-memory capability found in $($source.FullName)"
    }
  }
}

Write-Host "Runtime security audit passed for ${Architecture}: $($binaries.Count) PE files, no prohibited capability paths."
