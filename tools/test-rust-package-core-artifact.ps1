[CmdletBinding()]
param(
  [string] $CargoExecutable = 'cargo',
  [string] $CargoTarget = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$outRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out/package/rust-package-core'))
$packageRootName = 'Fcitx5RustPackageCore'

function Remove-SmokeTree {
  param([Parameter(Mandatory)] [string] $Path)
  $resolved = [IO.Path]::GetFullPath($Path)
  $prefix = $outRoot.TrimEnd('\') + '\'
  if ($resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolved)) {
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}

function Get-CargoTargetRoot {
  if (-not [string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    return [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
  }
  return [IO.Path]::GetFullPath((Join-Path $repoRoot 'target'))
}

function Test-NoPrivateKeyMaterial {
  param([Parameter(Mandatory)] [string] $Root)
  $securityRoot = Join-Path $Root 'security'
  $keyringPath = Join-Path $securityRoot 'trusted-keys.json'
  if (-not (Test-Path -LiteralPath $keyringPath -PathType Leaf)) {
    throw 'Rust package-core artifact is missing trusted-keys.json.'
  }
  $keyring = Get-Content -LiteralPath $keyringPath -Raw -Encoding UTF8 | ConvertFrom-Json
  if ($keyring.format_version -ne 2) {
    throw 'Rust package-core artifact must carry v2 public trusted keys.'
  }
  $rawSecurityJson = Get-ChildItem -LiteralPath $securityRoot -File -Recurse |
    Where-Object Extension -eq '.json' |
    ForEach-Object { Get-Content -LiteralPath $_.FullName -Raw -Encoding UTF8 }
  foreach ($marker in @('private_key', 'secret_key', 'seed_base64',
                        'private_key_base64', 'secret_key_base64')) {
    foreach ($content in $rawSecurityJson) {
      if ($content.IndexOf($marker, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
        throw "Private signing material marker found in Rust package-core artifact: $marker"
      }
    }
  }
}

New-Item -ItemType Directory -Force -Path $outRoot | Out-Null
if ([string]::IsNullOrWhiteSpace($CargoTarget)) {
  & $CargoExecutable build --locked --manifest-path (Join-Path $repoRoot 'Cargo.toml') `
    -p fcitx5-package-core --bin fcitx5-package-core
} else {
  & $CargoExecutable build --locked --manifest-path (Join-Path $repoRoot 'Cargo.toml') `
    -p fcitx5-package-core --bin fcitx5-package-core --target $CargoTarget
}
if ($LASTEXITCODE -ne 0) { throw 'Rust package-core build failed.' }

$targetRoot = Get-CargoTargetRoot
if ([string]::IsNullOrWhiteSpace($CargoTarget)) {
  $rustExe = Join-Path $targetRoot 'debug/fcitx5-package-core.exe'
} else {
  $rustExe = Join-Path $targetRoot (Join-Path $CargoTarget 'debug/fcitx5-package-core.exe')
}
if (-not (Test-Path -LiteralPath $rustExe -PathType Leaf)) {
  throw "Missing Rust package-core binary: $rustExe"
}

$stage = Join-Path $outRoot ('stage-' + [guid]::NewGuid().ToString('N'))
$smoke = Join-Path $outRoot ('smoke-' + [guid]::NewGuid().ToString('N'))
$artifact = Join-Path $outRoot 'fcitx5-package-core-smoke.zip'
$packageRoot = Join-Path $stage $packageRootName
try {
  New-Item -ItemType Directory -Force -Path (Join-Path $packageRoot 'bin'),
    (Join-Path $packageRoot 'security') | Out-Null
  Copy-Item -LiteralPath $rustExe -Destination (Join-Path $packageRoot 'bin/fcitx5-package-core.exe')
  Copy-Item -LiteralPath (Join-Path $repoRoot 'security/trusted-keys.template.json') `
    -Destination (Join-Path $packageRoot 'security/trusted-keys.json')
  $files = Get-ChildItem -LiteralPath $packageRoot -File -Recurse | Sort-Object FullName |
    ForEach-Object {
      [ordered]@{
        path = [IO.Path]::GetRelativePath($packageRoot, $_.FullName).Replace('\', '/')
        size = $_.Length
        sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
      }
    }
  $manifest = [ordered]@{
    format_version = 1
    artifact = 'fcitx5-package-core-rust-smoke'
    source_commit = (git -C $repoRoot rev-parse HEAD).Trim()
    files = @($files)
  }
  [IO.File]::WriteAllText((Join-Path $packageRoot 'manifest.json'),
    (($manifest | ConvertTo-Json -Depth 5) + "`n"), [Text.UTF8Encoding]::new($false))
  Compress-Archive -Path $packageRoot -DestinationPath $artifact -CompressionLevel Optimal -Force

  Expand-Archive -LiteralPath $artifact -DestinationPath $smoke
  $extractedRoot = Join-Path $smoke $packageRootName
  Test-NoPrivateKeyMaterial -Root $extractedRoot
  $stageManifest = Get-Content -LiteralPath (Join-Path $extractedRoot 'manifest.json') -Raw |
    ConvertFrom-Json
  foreach ($file in $stageManifest.files) {
    $path = Join-Path $extractedRoot ([string]$file.path).Replace('/', '\')
    if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
        (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne
          $file.sha256) {
      throw "Rust package-core staged-file verification failed: $($file.path)"
    }
  }
  & (Join-Path $extractedRoot 'bin/fcitx5-package-core.exe') --self-check --audit-self-pe `
    --trusted-keys (Join-Path $extractedRoot 'security/trusted-keys.json')
  if ($LASTEXITCODE -ne 0) { throw 'Rust package-core packaged artifact smoke failed.' }
} finally {
  Remove-SmokeTree -Path $stage
  Remove-SmokeTree -Path $smoke
}

Write-Host "Rust package-core packaged artifact smoke passed: $artifact"
