param(
  [Parameter(Mandatory = $true)] [string] $CargoTarget,
  [ValidateSet('dev', 'release')] [string] $CargoProfile = 'dev',
  [Parameter(Mandatory = $true)] [string] $OutputDirectory,
  [string] $ShippingConfigExe = '',
  [Parameter(Mandatory = $true)] [string] $Report
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)
$reportPath = [IO.Path]::GetFullPath($Report)
$reportDirectory = Split-Path -Parent $reportPath
if (-not (Test-Path -LiteralPath $reportDirectory -PathType Container)) {
  New-Item -ItemType Directory -Force -Path $reportDirectory | Out-Null
}
if (-not (Test-Path -LiteralPath $outputRoot -PathType Container)) {
  New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null
}

function Invoke-CheckedProcess {
  param(
    [Parameter(Mandatory = $true)] [string] $FilePath,
    [Parameter(Mandatory = $true)] [string[]] $Arguments,
    [Parameter(Mandatory = $true)] [string] $Name
  )

  $startInfo = [Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = $FilePath
  foreach ($argument in $Arguments) {
    [void] $startInfo.ArgumentList.Add($argument)
  }
  $startInfo.UseShellExecute = $false
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  $startInfo.CreateNoWindow = $true

  $process = [Diagnostics.Process]::Start($startInfo)
  $stdout = $process.StandardOutput.ReadToEnd()
  $stderr = $process.StandardError.ReadToEnd()
  $process.WaitForExit()
  if ($process.ExitCode -ne 0) {
    throw "$Name failed with exit code $($process.ExitCode): $stderr $stdout"
  }
  return [ordered]@{
    name = $Name
    arguments = $Arguments
    exit_code = $process.ExitCode
  }
}

$targetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
  Join-Path $repoRoot 'out\toolchains\rust\target'
} else {
  [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR)
}
$profileDirectory = if ($CargoProfile -eq 'dev') { 'debug' } else { 'release' }
$rustShipping = Join-Path $targetRoot "$CargoTarget\$profileDirectory\fcitx5-config.exe"
if (-not (Test-Path -LiteralPath $rustShipping -PathType Leaf)) {
  throw "Rust Config shipping executable was not built: $rustShipping"
}

$shippingExe = if ([string]::IsNullOrWhiteSpace($ShippingConfigExe)) {
  $rustShipping
} else {
  [IO.Path]::GetFullPath($ShippingConfigExe)
}
if (-not (Test-Path -LiteralPath $shippingExe -PathType Leaf)) {
  throw "Rust Config shipping-lineage executable is missing: $shippingExe"
}
$rustHash = (Get-FileHash -LiteralPath $rustShipping -Algorithm SHA256).Hash
$shippingHash = (Get-FileHash -LiteralPath $shippingExe -Algorithm SHA256).Hash
if ($rustHash -ne $shippingHash) {
  throw "Shipping fcitx5-config.exe is not byte-identical to the Rust shipping build output"
}

$rustReport = Join-Path $reportDirectory 'config-rust-shipping-lineage-self-check.json'
[void] (Invoke-CheckedProcess -FilePath $shippingExe -Arguments @('--self-check', '--report', $rustReport) -Name 'rust-config-shipping-lineage-self-check')

$rust = Get-Content -LiteralPath $rustReport -Raw -Encoding UTF8 | ConvertFrom-Json
if ($rust.component -ne 'fcitx5-config') {
  throw "Rust shipping-lineage report did not run under the shipping component name: $($rust.component)"
}
if ($rust.rust_shipping_target_name -ne 'fcitx5-config.exe') {
  throw "Rust shipping-lineage report lost the shipping executable name"
}
if ($rust.preserves_product_binary_name -ne $true) {
  throw "Rust shipping-lineage report did not preserve the product binary name"
}
if ($rust.shipping_config_replaced -ne $true) {
  throw "Rust shipping-lineage check must prove the CMake shipping target has been cut over"
}
if ($rust.permanent_runtime_selector -ne $false) {
  throw "Rust shipping-lineage report must not declare a permanent runtime selector"
}
if ($rust.candidate_preview_renderer_contract -ne 'shipping-candidate-real-preview-host-path') {
  throw "Rust shipping-lineage report did not preserve the Candidate preview host contract"
}

$reportObject = [ordered]@{
  component = 'fcitx5-config'
  kind = 'config-rust-shipping-lineage'
  rust_source_executable = $rustShipping
  rust_shipping_lineage_executable = $shippingExe
  rust_source_sha256 = $rustHash.ToLowerInvariant()
  rust_shipping_sha256 = $shippingHash.ToLowerInvariant()
  rust_report = $rustReport
  rust_shipping_target_name = 'fcitx5-config.exe'
  preserves_product_binary_name = $true
  shipping_config_replaced = $true
  permanent_runtime_selector = $false
  result = 'PASS'
}
$reportObject | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $reportPath -Encoding UTF8
Write-Host "config-rust-shipping-lineage-report=$reportPath result=PASS"
