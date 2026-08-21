[CmdletBinding()]
param([string] $Version = '0.1.0')

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$outRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out'))
$zip = Join-Path $outRoot "package/artifacts/fcitx5-windows-$Version-portable.zip"
if (-not (Test-Path -LiteralPath $zip -PathType Leaf)) { throw "Missing portable ZIP: $zip" }

$smokeRoot = Join-Path $outRoot ('portable-smoke-' + [guid]::NewGuid().ToString('N'))
$first = Join-Path $smokeRoot 'location-a'
$second = Join-Path $smokeRoot 'location-b'
$configMarker = $null
$dictionaryMarker = "# irreplaceable-rime-user-data`n"
New-Item -ItemType Directory -Path $first, $second -Force | Out-Null

function Stop-PortableSmokeProcesses {
  param([Parameter(Mandatory)] [string] $Root)
  $resolvedRoot = [IO.Path]::GetFullPath($Root).TrimEnd('\') + '\'
  $allowedNames = @(
    'Start Fcitx5.exe',
    'Fcitx5 Settings.exe',
    'Unregister Fcitx5.exe',
    'fcitx5-launcher.exe',
    'fcitx5-ui.exe',
    'fcitx5-engine.exe',
    'fcitx5-config.exe',
    'fcitx5-control.exe'
  )
  Get-CimInstance Win32_Process |
    Where-Object {
      $_.ExecutablePath -and
      $allowedNames -contains [IO.Path]::GetFileName($_.ExecutablePath) -and
      [IO.Path]::GetFullPath($_.ExecutablePath).StartsWith(
        $resolvedRoot, [StringComparison]::OrdinalIgnoreCase)
    } |
    ForEach-Object {
      try {
        $process = Get-Process -Id $_.ProcessId -ErrorAction Stop
        Stop-Process -InputObject $process -Force -ErrorAction Stop
      } catch {
        Write-Warning "Failed to stop portable smoke process $($_.ProcessId): $($_.Exception.Message)"
      }
    }
}

function Test-PackageOutputWritable {
  $packageRoot = Join-Path $outRoot 'package'
  if (-not (Test-Path -LiteralPath $packageRoot -PathType Container)) { return }
  $probe = Join-Path $packageRoot ('.portable-smoke-write-probe-' + [guid]::NewGuid().ToString('N'))
  [IO.File]::WriteAllText($probe, "probe`n", [Text.UTF8Encoding]::new($false))
  Remove-Item -LiteralPath $probe -Force
}

try {
  Expand-Archive -LiteralPath $zip -DestinationPath $first
  $app = Join-Path $first 'Fcitx5'
  foreach ($location in @('first', 'moved')) {
    foreach ($entry in @('Start Fcitx5.exe', 'Fcitx5 Settings.exe',
                         'Unregister Fcitx5.exe')) {
      $entryPath = Join-Path $app $entry
      if (-not (Test-Path -LiteralPath $entryPath -PathType Leaf)) {
        throw "Missing portable entry point at $location location: $entry"
      }
      $entryTest = Start-Process -FilePath $entryPath -ArgumentList '--self-test' `
        -Wait -PassThru -WindowStyle Hidden
      if ($entryTest.ExitCode -ne 0) {
        throw "Portable entry point self-test failed at $location location: $entry"
      }
    }
    $config = Start-Process -FilePath (Join-Path $app 'bin/fcitx5-config.exe') `
      -ArgumentList '--self-test' -Wait -PassThru -WindowStyle Hidden
    if ($config.ExitCode -ne 0) { throw "Config self-test failed at $location location." }
    $uiContract = Start-Process -FilePath (Join-Path $app 'bin/fcitx5-config.exe') `
      -ArgumentList '--ui-contract-test' -Wait -PassThru -WindowStyle Hidden
    if ($uiContract.ExitCode -ne 0) {
      throw "Config UI behavior contract failed at $location location."
    }
    $uiInteraction = Start-Process -FilePath (Join-Path $app 'bin/fcitx5-config.exe') `
      -ArgumentList '--ui-interaction-test' -Wait -PassThru -WindowStyle Hidden
    if ($uiInteraction.ExitCode -ne 0) {
      throw "Config complete interaction sweep failed at $location location."
    }
    $status = (& (Join-Path $app 'bin/fcitx5-control.exe') --status) | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw "Control status failed at $location location." }
    $actual = [IO.Path]::GetFullPath($status.data_root.Replace('/', '\')).TrimEnd('\')
    $expected = [IO.Path]::GetFullPath((Join-Path $app 'data')).TrimEnd('\')
    if ($actual -ne $expected) {
      throw "Portable data root mismatch at $location location: $actual != $expected"
    }
    $packageStatus = (& (Join-Path $app 'bin/fcitx5-control.exe') --packages-list) |
      ConvertFrom-Json
    $expectedBundled = @('fcitx5-chinese-addons', 'fcitx5-chttrans', 'fcitx5-lua',
                         'fcitx5-rime', 'librime-lua')
    $actualBundled = @($packageStatus.packages |
      Where-Object state -eq 'bundled' | ForEach-Object id | Sort-Object)
    if ($packageStatus.repository_available -or
        (Compare-Object $expectedBundled $actualBundled)) {
      throw 'Portable plugin manager did not expose the complete bundled component set.'
    }
    if ($location -eq 'first') {
      & (Join-Path $app 'bin/fcitx5-control.exe') --set-presentation dark `
        builtin:default horizontal enabled 6 'Microsoft YaHei'
      if ($LASTEXITCODE -ne 0) { throw 'Portable presentation save failed.' }
      $presentation = (& (Join-Path $app 'bin/fcitx5-control.exe') --get-presentation) |
        ConvertFrom-Json
      if ($presentation.appearance_mode -ne 'dark' -or
          $presentation.orientation -ne 'horizontal' -or
          -not $presentation.scroll_mode -or
          $presentation.candidate_page_size -ne '6' -or
          $presentation.candidate_font -ne 'Microsoft YaHei') {
        throw 'Portable presentation did not round-trip through the typed Control API.'
      }
      $configMarker = [IO.File]::ReadAllText((Join-Path $app 'data/config.toml'))
      $rimeUser = Join-Path $app 'data/Fcitx5/rime/user.dict.yaml'
      New-Item -ItemType Directory -Force -Path (Split-Path -Parent $rimeUser) | Out-Null
      [IO.File]::WriteAllText($rimeUser, $dictionaryMarker,
        [Text.UTF8Encoding]::new($false))
      $moved = Join-Path $second 'Fcitx5'
      Move-Item -LiteralPath $app -Destination $moved
      $app = $moved
    } else {
      $presentation = (& (Join-Path $app 'bin/fcitx5-control.exe') --get-presentation) |
        ConvertFrom-Json
      if ($presentation.appearance_mode -ne 'dark' -or
          $presentation.orientation -ne 'horizontal' -or -not $presentation.scroll_mode) {
        throw 'Moved portable copy did not reload its saved presentation settings.'
      }
      foreach ($preserved in @(
          [pscustomobject]@{ Path = (Join-Path $app 'data/config.toml'); Text = $configMarker }
          [pscustomobject]@{ Path = (Join-Path $app 'data/Fcitx5/rime/user.dict.yaml'); Text = $dictionaryMarker }
        )) {
        if (-not (Test-Path -LiteralPath $preserved.Path -PathType Leaf) -or
            [IO.File]::ReadAllText($preserved.Path) -ne $preserved.Text) {
          throw "Portable move did not preserve user data: $($preserved.Path)"
        }
      }
      # Simulate extracting a newer portable archive over the existing directory.
      # The release archive contains an empty data directory, never default user files.
      Expand-Archive -LiteralPath $zip -DestinationPath $second -Force
      if ([IO.File]::ReadAllText((Join-Path $app 'data/config.toml')) -ne $configMarker -or
          [IO.File]::ReadAllText((Join-Path $app 'data/Fcitx5/rime/user.dict.yaml')) -ne
            $dictionaryMarker) {
        throw 'Portable upgrade extraction overwrote user configuration or Rime data.'
      }
    }
  }
  Write-Host 'Portable ZIP self-test, move, and user-data-preserving upgrade tests passed.'
} finally {
  Stop-PortableSmokeProcesses -Root $smokeRoot
  $resolved = [IO.Path]::GetFullPath($smokeRoot)
  $prefix = $outRoot.TrimEnd('\') + '\portable-smoke-'
  if ($resolved.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolved)) {
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
}
Test-PackageOutputWritable
