[CmdletBinding()]
param(
  [string] $Version = '0.1.0',
  [ValidateSet('stable', 'beta', 'nightly')]
  [string] $Channel = 'stable',
  [switch] $SkipInstaller
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$outRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out/package'))
$core = Join-Path $repoRoot 'out/stage/fcitx5'
$x64 = Join-Path $repoRoot 'out/build/windows-x64-dev/Release'
$x86 = Join-Path $repoRoot 'out/build/windows-x86-dev/Release'
$work = Join-Path $outRoot ('stage-' + [guid]::NewGuid().ToString('N'))
$root = Join-Path $work 'Fcitx5'
$artifacts = Join-Path $outRoot 'artifacts'

$required = @(
  (Join-Path $core 'bin/fcitx5-engine.exe'),
  (Join-Path $x64 'fcitx5-config.exe'), (Join-Path $x64 'fcitx5-control.exe'),
  (Join-Path $x64 'fcitx5-launcher.exe'), (Join-Path $x64 'fcitx5-ui.exe'),
  (Join-Path $x64 'fcitx5-package.exe'), (Join-Path $x64 'fcitx5-downloader.exe'),
  (Join-Path $x64 'fcitx5-deployer.exe'), (Join-Path $x64 'fcitx5-provider.exe'),
  (Join-Path $x64 'fcitx5-updater.exe'),
  (Join-Path $x64 'fcitx5-bootstrap.exe'),
  (Join-Path $x64 'fcitx5-register.exe'), (Join-Path $x64 'fcitx5-tsf.dll'),
  (Join-Path $x86 'fcitx5-register.exe'), (Join-Path $x86 'fcitx5-tsf.dll'))
foreach ($path in $required) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing package input: $path" }
}

New-Item -ItemType Directory -Force -Path $root, $artifacts | Out-Null
Copy-Item -LiteralPath (Join-Path $core 'bin') -Destination $root -Recurse
Copy-Item -LiteralPath (Join-Path $core 'lib') -Destination $root -Recurse
Copy-Item -LiteralPath (Join-Path $core 'share') -Destination $root -Recurse
foreach ($path in @((Join-Path $root 'lib/cmake'), (Join-Path $root 'share/fcitx5/testing'))) {
  if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force }
}
Get-ChildItem -LiteralPath (Join-Path $root 'lib/fcitx5') -Filter 'libtest*.dll' -File `
  -ErrorAction SilentlyContinue | Remove-Item -Force

$bin = Join-Path $root 'bin'
foreach ($name in @('fcitx5-config.exe', 'fcitx5-control.exe', 'fcitx5-launcher.exe',
                     'fcitx5-ui.exe', 'fcitx5-register.exe', 'fcitx5-package.exe',
                     'fcitx5-downloader.exe', 'fcitx5-deployer.exe',
                     'fcitx5-provider.exe', 'fcitx5-updater.exe')) {
  Copy-Item -LiteralPath (Join-Path $x64 $name) -Destination $bin -Force
}
foreach ($name in @('Start Fcitx5.exe', 'Fcitx5 Settings.exe',
                     'Unregister Fcitx5.exe')) {
  Copy-Item -LiteralPath (Join-Path $x64 'fcitx5-bootstrap.exe') `
    -Destination (Join-Path $root $name) -Force
}
Copy-Item -LiteralPath (Join-Path $x86 'fcitx5-register.exe') `
  -Destination (Join-Path $bin 'fcitx5-register-x86.exe') -Force
New-Item -ItemType Directory -Force -Path (Join-Path $root 'tsf/x64'),
  (Join-Path $root 'tsf/x86'), (Join-Path $bin 'locales'),
  (Join-Path $root 'themes'), (Join-Path $root 'data'),
  (Join-Path $root 'security') | Out-Null
Copy-Item -LiteralPath (Join-Path $x64 'fcitx5-tsf.dll') -Destination (Join-Path $root 'tsf/x64')
Copy-Item -LiteralPath (Join-Path $x86 'fcitx5-tsf.dll') -Destination (Join-Path $root 'tsf/x86')
Copy-Item -Path (Join-Path $repoRoot 'locales/*') -Destination (Join-Path $bin 'locales')
Copy-Item -LiteralPath (Join-Path $repoRoot 'resources/themes/default') `
  -Destination (Join-Path $root 'themes') -Recurse
Copy-Item -LiteralPath (Join-Path $repoRoot 'security/trusted-keys.template.json') `
  -Destination (Join-Path $root 'security/trusted-keys.json')
[IO.File]::WriteAllText((Join-Path $root 'portable.flag'), "portable`n",
  [Text.UTF8Encoding]::new($false))

$files = Get-ChildItem -LiteralPath $root -File -Recurse | Sort-Object FullName | ForEach-Object {
  [ordered]@{
    path = [IO.Path]::GetRelativePath($root, $_.FullName).Replace('\', '/')
    size = $_.Length
    sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
  }
}
$manifest = [ordered]@{
  format_version = 1
  product = 'fcitx5-windows-next'
  version = $Version
  channel = $Channel
  source_commit = (git -C $repoRoot rev-parse HEAD).Trim()
  source_tree_clean = @((git -C $repoRoot status --porcelain=v1 --untracked-files=all)).Count -eq 0
  architecture = 'x64-with-x86-tsf'
  files = @($files)
}
[IO.File]::WriteAllText((Join-Path $root 'manifest.json'),
  (($manifest | ConvertTo-Json -Depth 5) + "`n"), [Text.UTF8Encoding]::new($false))

$artifactSuffix = if ($Channel -eq 'stable') { '' } else { "-$Channel" }
$portable = Join-Path $artifacts "fcitx5-windows-$Version$artifactSuffix-portable.zip"
Compress-Archive -Path $root -DestinationPath $portable -CompressionLevel Optimal -Force
if (-not $SkipInstaller) {
  & (Join-Path $PSScriptRoot 'prepare-package-toolchains.ps1')
  $iscc = Join-Path $repoRoot 'out/toolchains/inno-7.0.2/ISCC.exe'
  # Build to the immutable per-stage directory first. Antivirus and Explorer may briefly hold the
  # previously published fixed-name installer; compiling directly over it makes that transient
  # Windows lock look like a source/build failure.
  $installerOutput = Join-Path $work 'installer-output'
  New-Item -ItemType Directory -Force -Path $installerOutput | Out-Null
  & $iscc "/DProductVersion=$Version" "/DReleaseChannel=$Channel" `
    "/DStageDir=$root" "/DArtifactDir=$installerOutput" `
    (Join-Path $repoRoot 'installer/fcitx5-windows.iss')
  if ($LASTEXITCODE -ne 0) { throw 'Inno Setup package build failed.' }
  $installerName = "fcitx5-windows-$Version$artifactSuffix-setup.exe"
  $stagedInstaller = Join-Path $installerOutput $installerName
  $publishedInstaller = Join-Path $artifacts $installerName
  $published = $false
  for ($attempt = 1; $attempt -le 20 -and -not $published; $attempt++) {
    try {
      Copy-Item -LiteralPath $stagedInstaller -Destination $publishedInstaller -Force
      $published = $true
    } catch [IO.IOException] {
      if ($attempt -eq 20) { throw }
      Start-Sleep -Milliseconds 250
    }
  }
}
[IO.File]::WriteAllText((Join-Path $outRoot 'current-stage.txt'), $root,
  [Text.UTF8Encoding]::new($false))
Write-Host "Package artifacts staged from one build: $artifacts"
