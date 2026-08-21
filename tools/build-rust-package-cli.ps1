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
  throw "Refusing to write Rust package CLI outside repository: $outputRoot"
}

New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

& $CargoExecutable build --locked --manifest-path (Join-Path $repoRoot 'Cargo.toml') `
  -p fcitx5-package-core --bin fcitx5-package-core --target $CargoTarget
if ($LASTEXITCODE -ne 0) {
  throw 'Rust fcitx5-package CLI build failed.'
}

$targetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  Join-Path $repoRoot 'target'
} else {
  [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
}
$rustExe = Join-Path $targetRoot (Join-Path $CargoTarget 'debug/fcitx5-package-core.exe')
if (-not (Test-Path -LiteralPath $rustExe -PathType Leaf)) {
  throw "Missing Rust package CLI binary: $rustExe"
}

Copy-Item -LiteralPath $rustExe -Destination (Join-Path $outputRoot 'fcitx5-package.exe') -Force
