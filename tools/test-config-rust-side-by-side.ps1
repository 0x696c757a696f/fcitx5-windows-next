param(
  [Parameter(Mandatory = $true)] [string] $CppConfigExe,
  [Parameter(Mandatory = $true)] [string] $CargoExecutable,
  [Parameter(Mandatory = $true)] [string] $CargoTarget,
  [Parameter(Mandatory = $true)] [string] $Report
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$OutputEncoding = [System.Text.UTF8Encoding]::new()

$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$cppConfig = [IO.Path]::GetFullPath($CppConfigExe)
$cargo = [IO.Path]::GetFullPath($CargoExecutable)
$reportPath = [IO.Path]::GetFullPath($Report)
$reportDirectory = Split-Path -Parent $reportPath
if (-not (Test-Path -LiteralPath $cppConfig -PathType Leaf)) {
  throw "C++ Config baseline executable is missing: $cppConfig"
}
if (-not (Test-Path -LiteralPath $cargo -PathType Leaf)) {
  throw "Cargo executable is missing: $cargo"
}
if (-not (Test-Path -LiteralPath $reportDirectory -PathType Container)) {
  New-Item -ItemType Directory -Force -Path $reportDirectory | Out-Null
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

$cppContracts = @(
  @{ Name = 'cpp-config-self-test'; Args = @('--self-test') },
  @{ Name = 'cpp-config-i18n'; Args = @('--check-i18n') },
  @{ Name = 'cpp-config-resources'; Args = @('--check-resources') },
  @{ Name = 'cpp-config-behavior'; Args = @('--ui-contract-test') },
  @{ Name = 'cpp-config-visual'; Args = @('--ui-visual-contract-test') },
  @{ Name = 'cpp-config-live-preview'; Args = @('--ui-live-preview-contract-test') },
  @{ Name = 'cpp-config-interaction'; Args = @('--ui-interaction-test') }
)
$cppResults = foreach ($contract in $cppContracts) {
  Invoke-CheckedProcess -FilePath $cppConfig -Arguments $contract.Args -Name $contract.Name
}

$rustReport = Join-Path $reportDirectory 'config-rust-side-by-side-differential-rust.json'
$cargoArguments = @(
  'run',
  '--locked',
  '--manifest-path',
  (Join-Path $repoRoot 'Cargo.toml'),
  '-p',
  'fcitx5-config-poc',
  '--bin',
  'fcitx5-config-rust',
  '--target',
  $CargoTarget,
  '--',
  '--self-check',
  '--report',
  $rustReport
)
[void] (Invoke-CheckedProcess -FilePath $cargo -Arguments $cargoArguments -Name 'rust-config-side-by-side-self-check')

$rust = Get-Content -LiteralPath $rustReport -Raw -Encoding UTF8 | ConvertFrom-Json
$requiredTrueFields = @(
  'config_rust_cutover_plan',
  'frozen_corpus_from_config_ux_009',
  'side_by_side_executable_target_declared',
  'side_by_side_uses_frozen_corpus',
  'preserves_product_binary_name',
  'side_by_side_differential_required',
  'typed_control_only',
  'no_input_hot_path_access',
  'no_arbitrary_shell',
  'language_selector',
  'localized_dialogs',
  'candidate_preview_embedded',
  'candidate_preview_current_theme',
  'candidate_preview_not_external_window',
  'candidate_preview_embedded_in_config_content',
  'candidate_preview_uses_real_theme_contract',
  'theme_library_model_rust_owned',
  'font_selection',
  'advanced_appearance_controls',
  'input_method_list',
  'settings_operation_state_machine',
  'theme_action_state_machine',
  'theme_operations_backend_live',
  'typed_control_schema_consumed',
  'package_action_state_machine',
  'signed_repository_required_for_install',
  'unconfigured_repository_install_blocked',
  'addon_install_transition_checked',
  'addon_update_transition_checked',
  'addon_uninstall_transition_checked',
  'addon_enable_transition_checked',
  'addon_disable_transition_checked',
  'update_refresh_transition_checked',
  'localized_operation_errors',
  'no_unsafe_commands_for_package_actions',
  'layout_rects_inside_window',
  'layout_rects_non_overlapping'
)
foreach ($field in $requiredTrueFields) {
  if ($rust.$field -ne $true) {
    throw "Rust side-by-side report does not preserve required Config corpus field: $field"
  }
}
foreach ($dpi in 100, 125, 150, 200, 300) {
  if ($rust.checked_dpi_scale_percents -notcontains $dpi) {
    throw "Rust side-by-side report is missing $dpi percent DPI corpus coverage"
  }
}
if ($rust.component -ne 'fcitx5-config-rust') {
  throw "Rust side-by-side report used the wrong component name: $($rust.component)"
}
if ($rust.rust_shipping_target_name -ne 'fcitx5-config.exe') {
  throw "Rust side-by-side report lost the shipping Config binary name"
}
if ($rust.permanent_runtime_selector -ne $false) {
  throw 'Rust side-by-side report must not declare a permanent runtime selector'
}

$reportObject = [ordered]@{
  component = 'fcitx5-config-rust'
  kind = 'config-rust-side-by-side-differential'
  cpp_baseline = $cppConfig
  rust_side_by_side = 'fcitx5-config-rust'
  rust_shipping_target_name = 'fcitx5-config.exe'
  cpp_contracts_passed = $cppResults
  rust_report = $rustReport
  compared_contracts = @(
    'startup',
    'i18n',
    'resources',
    'behavior',
    'visual-no-overlap',
    'live-preview',
    'interaction'
  )
  side_by_side_uses_frozen_corpus = $true
  permanent_runtime_selector = $false
  result = 'PASS'
}
$reportObject | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $reportPath -Encoding UTF8
Write-Host "Config Rust side-by-side differential passed: $reportPath"
