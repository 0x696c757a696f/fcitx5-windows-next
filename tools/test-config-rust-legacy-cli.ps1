param(
  [string] $CargoExecutable = 'cargo',
  [string] $CargoTarget = 'x86_64-pc-windows-msvc',
  [string] $Report = ''
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()

$repo = Resolve-Path (Join-Path $PSScriptRoot '..')
Set-Location $repo

$legacyArgs = @(
  '--self-test',
  '--check-i18n',
  '--check-resources',
  '--ui-contract-test',
  '--ui-visual-contract-test',
  '--ui-live-preview-contract-test',
  '--ui-interaction-test'
)

$results = @()
foreach ($legacyArg in $legacyArgs) {
  $output = & $CargoExecutable run --locked --manifest-path (Join-Path $repo 'Cargo.toml') `
    -p fcitx5-config-poc --bin fcitx5-config --target $CargoTarget -- $legacyArg
  $joined = $output -join "`n"
  if ($joined -notmatch '"legacy_config_cli_compat":true') {
    throw "Rust Config legacy CLI output for $legacyArg did not report compatibility."
  }
  if ($joined -notmatch '"rust_config_self_check_reused":true') {
    throw "Rust Config legacy CLI output for $legacyArg did not reuse the Rust self-check corpus."
  }
  if ($joined -notmatch '"result":"PASS"') {
    throw "Rust Config legacy CLI output for $legacyArg did not pass."
  }
  $results += [pscustomobject]@{
    argument = $legacyArg
    result = 'PASS'
  }
}

$reportObject = [pscustomobject]@{
  component = 'fcitx5-config'
  kind = 'rust-config-legacy-headless-cli'
  legacy_config_cli_compat = $true
  legacy_arguments = $results
  shipping_config_replaced = $true
  result = 'PASS'
}

if ($Report -ne '') {
  $parent = Split-Path -Parent $Report
  if ($parent -ne '') {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
  $reportObject | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $Report -Encoding utf8NoBOM
}

Write-Host 'Rust Config legacy headless CLI compatibility passed.'
