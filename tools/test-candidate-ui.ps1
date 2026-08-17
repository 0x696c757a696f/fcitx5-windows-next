[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$build = Join-Path $repoRoot 'out/build/windows-x64-dev/Debug'
$stage = Join-Path $repoRoot 'out/stage/fcitx5'
$uiSource = Join-Path $build 'fcitx5-ui.exe'
$uiTarget = Join-Path $stage 'bin/fcitx5-ui.exe'
$resourceSource = Join-Path $build 'resources'
$resourceTarget = Join-Path $stage 'bin/resources'
$engine = Join-Path $stage 'bin/fcitx5-engine.exe'
$test = Join-Path $build 'fcitx5_engine_integration_test.exe'
foreach ($path in @($uiSource, $engine, $test)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Missing candidate UI test artifact: $path"
  }
}
Copy-Item -LiteralPath $uiSource -Destination $uiTarget -Force
if (Test-Path -LiteralPath $resourceSource -PathType Container) {
  New-Item -ItemType Directory -Path $resourceTarget -Force | Out-Null
  Copy-Item -Path (Join-Path $resourceSource '*') -Destination $resourceTarget -Recurse -Force
}
$ui = Start-Process -FilePath $uiTarget -ArgumentList '--test-once' `
  -PassThru -WindowStyle Hidden
try {
  Start-Sleep -Milliseconds 300
  & $test $engine
  if ($LASTEXITCODE -ne 0) { throw 'Real engine candidate test failed.' }
  if (-not $ui.WaitForExit(5000) -or $ui.ExitCode -ne 0) {
    throw 'Independent UI did not consume the authenticated candidate snapshot.'
  }
} finally {
  if (-not $ui.HasExited) { Stop-Process -Id $ui.Id }
}
Write-Host 'Engine-owned candidate snapshot reached the independent UI process.'
