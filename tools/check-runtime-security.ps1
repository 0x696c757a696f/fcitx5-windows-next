[CmdletBinding()]
param(
  [ValidateSet('x64', 'x86', 'arm64')]
  [string] $Architecture = 'x64',

  [ValidateSet('Debug', 'Release')]
  [string] $Configuration = 'Debug',

  [ValidateSet('Win7', 'Win10')]
  [string] $MinOs = 'Win10',

  [switch] $SourceOnly
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
if (-not $SourceOnly) {
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

  $presetKind = if ($Configuration -eq 'Release') { 'release' } else { 'dev' }
  $binaryRoot = Join-Path $repoRoot "out/build/windows-$Architecture-$presetKind/$Configuration"
  $binaries = @(
    (Join-Path $binaryRoot 'fcitx5-tsf.dll'),
    (Join-Path $binaryRoot 'fcitx5-launcher.exe'),
    (Join-Path $binaryRoot 'fcitx5-ui.exe'),
    (Join-Path $binaryRoot 'fcitx5-control.exe'),
    (Join-Path $binaryRoot 'fcitx5-config.exe'),
    (Join-Path $binaryRoot 'fcitx5-register.exe'),
    (Join-Path $binaryRoot 'fcitx5-package.exe'),
    (Join-Path $binaryRoot 'fcitx5-updater.exe'),
    (Join-Path $binaryRoot 'fcitx5-downloader.exe'),
    (Join-Path $binaryRoot 'fcitx5-deployer.exe'),
    (Join-Path $binaryRoot 'fcitx5-provider.exe')
  )
  # The native Fcitx engine is built through the MSYS2 toolchain for x64/x86
  # only; ARM64 keeps the Windows frontend surface while the engine remains
  # a planned platform.
  if ($Architecture -in @('x64', 'x86')) {
    $nativeEngine = Join-Path $repoRoot 'out/stage/fcitx5/bin/fcitx5-engine.exe'
    if (Test-Path -LiteralPath $nativeEngine -PathType Leaf) { $binaries += $nativeEngine }
  }

  $win7IncompatibleImports = @(
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
    if ($MinOs -eq 'Win7') {
      foreach ($name in $win7IncompatibleImports) {
        if ($imports -match "(?m)^\s+[0-9A-F]+\s+$([regex]::Escape($name))\s*$") {
          throw "Win7-incompatible hard import '$name' found in $binary"
        }
      }
    }
    $fileName = [IO.Path]::GetFileName($binary)
    # Rust std currently brings a WS2_32 import into Rust-linked MSVC
    # binaries even when product source does not use networking. Keep PE
    # blocking for explicit HTTP/URL stacks and enforce Winsock usage at the
    # source boundary below.
    if ($fileName -in @('fcitx5-tsf.dll', 'fcitx5-engine.exe', 'fcitx5-ui.exe') -and
        $imports -match '(?im)^\s+(WINHTTP|WININET|URLMON)\.dll\s*$') {
      throw "Network-capable library imported by input-plane binary: $binary"
    }
    if ($fileName -in @('fcitx5-package.exe', 'fcitx5-deployer.exe', 'fcitx5-updater.exe') -and
        $imports -match '(?im)^\s+(WINHTTP|WININET|URLMON)\.dll\s*$') {
      throw "Network library crossed into non-downloader package boundary: $binary"
    }
    if ($fileName -eq 'fcitx5-downloader.exe' -and
        $imports -notmatch '(?im)^\s+WINHTTP\.dll\s*$') {
      throw "Downloader is missing its explicit WinHTTP boundary: $binary"
    }
  }
}

$sourceRoots = @(
  @{ Path = Join-Path $repoRoot 'src'; Include = @('*.cpp', '*.h'); Label = 'C++ source' },
  @{ Path = Join-Path $repoRoot 'rust'; Include = @('*.rs'); Label = 'Rust source' }
)
$prohibited = @(
  '\bSetWindowsHookEx[AW]?\s*\(',
  '\bSendInput\s*\(',
  '\bWriteProcessMemory\s*\(',
  '\bCreateRemoteThread(?:Ex)?\s*\(',
  '\bVirtualAllocEx\s*\(',
  '\bPROCESS_VM_(?:READ|WRITE|OPERATION)\b'
)
$networkProhibited = @(
  '\bWinHttp[A-Za-z0-9_]*\s*\(',
  '\bInternet[A-Za-z0-9_]*\s*\(',
  '\bURLDownloadToFile[AW]?\s*\(',
  '\bWSAStartup\s*\(',
  '\bsocket\s*\(',
  '\bstd::net::',
  '\bTcpStream\b',
  '\bUdpSocket\b',
  '\bwindows::Win32::Networking\b'
)
$networkAllowedSources = @(
  'rust/package-core/src/downloader_main.rs'
)
$scannedSourceCount = 0
foreach ($root in $sourceRoots) {
  if (-not (Test-Path -LiteralPath $root.Path -PathType Container)) {
    continue
  }
  $sourceFiles = Get-ChildItem -LiteralPath $root.Path -File -Recurse -Include $root.Include
  foreach ($source in $sourceFiles) {
    $scannedSourceCount += 1
    $text = Get-Content -LiteralPath $source.FullName -Raw
    foreach ($pattern in $prohibited) {
      if ($text -match $pattern) {
        throw "Prohibited Hook/injection/game-memory capability found in $($source.FullName)"
      }
    }
    $relativeSourcePath = [IO.Path]::GetRelativePath($repoRoot, $source.FullName).Replace('\', '/')
    if ($relativeSourcePath -notin $networkAllowedSources) {
      foreach ($pattern in $networkProhibited) {
        if ($text -match $pattern) {
          throw "Network capability crossed product source boundary in $($source.FullName)"
        }
      }
    }
  }
}

if ($SourceOnly) {
  Write-Host "Runtime security source audit passed: $scannedSourceCount C++/Rust source files, no prohibited capability paths."
} else {
  Write-Host "Runtime security audit passed for ${Architecture}/${MinOs}: $($binaries.Count) PE files, $scannedSourceCount C++/Rust source files, no prohibited capability paths."
}
