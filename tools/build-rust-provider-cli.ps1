[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $CargoExecutable,
  [Parameter(Mandatory)] [string] $CargoTarget,
  [Parameter(Mandatory)] [string] $OutputDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
$repoPrefix = $repoRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) +
  [IO.Path]::DirectorySeparatorChar
if (-not $outputRoot.StartsWith($repoPrefix, [StringComparison]::OrdinalIgnoreCase)) {
  throw "Refusing to write Rust provider CLI outside repository: $outputRoot"
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

$env:Path = (($env:Path -split [IO.Path]::PathSeparator) |
  Where-Object {
    -not $_.EndsWith('\Git\usr\bin', [StringComparison]::OrdinalIgnoreCase)
  }) -join [IO.Path]::PathSeparator

& $CargoExecutable build --locked --manifest-path (Join-Path $repoRoot 'Cargo.toml') `
  -p fcitx5-package-core --bin fcitx5-provider --target $CargoTarget
if ($LASTEXITCODE -ne 0) {
  throw 'Rust fcitx5-provider CLI build failed.'
}

$targetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  Join-Path $repoRoot 'target'
} else {
  [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
}
$rustExe = Join-Path $targetRoot (Join-Path $CargoTarget 'debug/fcitx5-provider.exe')
if (-not (Test-Path -LiteralPath $rustExe -PathType Leaf)) {
  throw "Missing Rust provider CLI binary: $rustExe"
}

Copy-Item -LiteralPath $rustExe -Destination (Join-Path $outputRoot 'fcitx5-provider.exe') -Force
