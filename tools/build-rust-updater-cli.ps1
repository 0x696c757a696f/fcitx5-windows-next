[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $CargoExecutable,
  [Parameter(Mandatory)] [string] $CargoTarget,
  [Parameter(Mandatory)] [string] $OutputDirectory,
  [Parameter(Mandatory)] [string] $Version
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
$repoPrefix = $repoRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) +
  [IO.Path]::DirectorySeparatorChar
if (-not $outputRoot.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase)) {
  throw "Refusing to write Rust updater CLI outside repository: $outputRoot"
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

$env:Path = (($env:Path -split [IO.Path]::PathSeparator) |
  Where-Object {
    -not $_.EndsWith('\Git\usr\bin', [StringComparison]::OrdinalIgnoreCase)
  }) -join [IO.Path]::PathSeparator
$env:FCITX_WINDOWS_VERSION = $Version
if ([string]::IsNullOrWhiteSpace($env:RUSTC_WRAPPER)) {
  $repoSccache = Join-Path $repoRoot `
    'out/toolchains/fast/sccache-0.17.0/sccache-v0.17.0-x86_64-pc-windows-msvc/sccache.exe'
  if (Test-Path -LiteralPath $repoSccache -PathType Leaf) {
    $env:RUSTC_WRAPPER = [IO.Path]::GetFullPath($repoSccache)
  }
}

& $CargoExecutable build --locked --manifest-path (Join-Path $repoRoot 'Cargo.toml') `
  -p fcitx5-package-core --bin fcitx5-updater --target $CargoTarget
if ($LASTEXITCODE -ne 0) {
  throw 'Rust fcitx5-updater CLI build failed.'
}

$targetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  Join-Path $repoRoot 'target'
} else {
  [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
}
$rustExe = Join-Path $targetRoot (Join-Path $CargoTarget 'debug/fcitx5-updater.exe')
if (-not (Test-Path -LiteralPath $rustExe -PathType Leaf)) {
  throw "Missing Rust updater CLI binary: $rustExe"
}

$outputExe = Join-Path $outputRoot 'fcitx5-updater.exe'
Copy-Item -LiteralPath $rustExe -Destination $outputExe -Force

$mt = $null
$command = Get-Command 'mt.exe' -ErrorAction SilentlyContinue
if ($null -ne $command) {
  $mt = $command.Source
}
if ([string]::IsNullOrWhiteSpace($mt)) {
  $localMt = Get-ChildItem `
    -LiteralPath (Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin') `
    -Recurse -Filter 'mt.exe' -File -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName.EndsWith('\x64\mt.exe', [StringComparison]::OrdinalIgnoreCase) } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
  if ($null -ne $localMt) {
    $mt = $localMt.FullName
  }
}
if ([string]::IsNullOrWhiteSpace($mt)) {
  throw 'Unable to locate mt.exe for Rust updater asInvoker manifest embedding.'
}

$manifestPath = Join-Path $outputRoot 'fcitx5-updater.asInvoker.manifest'
$manifest = @'
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
</assembly>
'@
[IO.File]::WriteAllText($manifestPath, $manifest, [Text.UTF8Encoding]::new($false))
& $mt -manifest $manifestPath "-outputresource:$outputExe;#1"
if ($LASTEXITCODE -ne 0) {
  throw 'Rust fcitx5-updater manifest embedding failed.'
}
