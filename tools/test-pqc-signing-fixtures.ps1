[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $BuildDirectory,
  [string] $Configuration = 'Debug'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$build = [IO.Path]::GetFullPath($BuildDirectory)
$bin = Join-Path $build $Configuration
$signer = Join-Path $bin 'fcitx5-pqc-fixture-signer.exe'
$package = Join-Path $bin 'fcitx5-package.exe'
foreach ($path in @($signer, $package)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Required test binary is missing: $path"
  }
}

function Invoke-Required {
  param([Parameter(Mandatory)] [scriptblock] $Command)
  & $Command
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed with exit code $LASTEXITCODE."
  }
}

function Invoke-ExpectedFailure {
  param([Parameter(Mandatory)] [scriptblock] $Command)
  & $Command
  if ($LASTEXITCODE -eq 0) {
    throw 'Command unexpectedly succeeded.'
  }
}

function Assert-NoPrivateKeyMaterial {
  param([Parameter(Mandatory)] [string] $Path)
  $text = Get-Content -LiteralPath $Path -Raw -Encoding UTF8
  foreach ($marker in @('private_key', 'secret_key', 'seed_base64', 'private_key_base64',
                        'secret_key_base64')) {
    if ($text.IndexOf($marker, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
      throw "Private signing material marker found in ${Path}: $marker"
    }
  }
}

$work = Join-Path $repoRoot ('out/tmp/pqc-fixture-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $work | Out-Null
try {
  $keyId = 'official-test-2026-mldsa65'
  $index = Join-Path $work 'index.json'
  $indexSig = Join-Path $work 'index.sig.json'
  $keyring = Join-Path $work 'trusted-keys.json'
  [IO.File]::WriteAllText($index,
    '{"format_version":1,"channel":"stable","generated_at":"2026-08-21T00:00:00Z",' +
    '"key_id":"' + $keyId + '","packages":[{"id":"fcitx5-rime","title":"Rime",' +
    '"summary":"Rime input engine","version":"1.0.0","release_sequence":1,' +
    '"type":"addon","architecture":"x64","download_url":"https://packages.example.invalid/fcitx5-rime.fcpkg",' +
    '"sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",' +
    '"dependencies":[]}]}',
    [Text.UTF8Encoding]::new($false))

  Invoke-Required { & $signer --sign repository-index $index $indexSig $keyring $keyId }
  Assert-NoPrivateKeyMaterial -Path $keyring
  Invoke-Required { & $package --verify-repository-v2 $index $indexSig $keyring stable }
  Invoke-ExpectedFailure {
    & $package --verify-repository-v2 $index (Join-Path $work 'missing-index.sig.json') $keyring stable
  }

  $manifest = Join-Path $work 'manifest.json'
  $manifestSig = Join-Path $work 'manifest.sig.json'
  [IO.File]::WriteAllText($manifest,
    "{`n" +
    "  `"format_version`": 1,`n" +
    "  `"id`": `"fcitx5-rime`",`n" +
    "  `"version`": `"1.0.0`",`n" +
    "  `"type`": `"addon`",`n" +
    "  `"architecture`": `"x64`",`n" +
    "  `"min_os`": `"6.1-sp1`",`n" +
    "  `"core_api`": `"1`",`n" +
    "  `"addon_abi`": `"1`",`n" +
    "  `"dependencies`": [],`n" +
    "  `"license`": `"MIT`",`n" +
    "  `"source_commit`": `"0123456789abcdef`",`n" +
    "  `"permissions`": [`"native-code`"],`n" +
    "  `"files`": [{`"path`": `"bin/addon.dll`", `"size`": 1, `"sha256`": `"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef`"}],`n" +
    "  `"key_id`": `"$keyId`"`n" +
    "}`n",
    [Text.UTF8Encoding]::new($false))

  Invoke-Required { & $signer --sign package-manifest $manifest $manifestSig $keyring $keyId }
  Assert-NoPrivateKeyMaterial -Path $keyring
  Invoke-Required { & $package --verify-manifest-v2 $manifest $manifestSig $keyring }
  Invoke-ExpectedFailure {
    & $package --verify-manifest-v2 $manifest (Join-Path $work 'missing-manifest.sig.json') $keyring
  }
} finally {
  if (Test-Path -LiteralPath $work) {
    Remove-Item -LiteralPath $work -Recurse -Force
  }
}

Write-Host 'PQC signing fixture smoke passed.'
