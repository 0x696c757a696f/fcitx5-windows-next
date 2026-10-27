[CmdletBinding()]
param(
  [string] $Version = '0.1.0',
  [ValidateSet('stable', 'beta', 'nightly')]
  [string] $Channel = 'stable',
  [string] $Generation = '',
  [switch] $SkipInstaller
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$outRoot = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out/package'))
$core = Join-Path $repoRoot 'out/stage/fcitx5'
$x64 = Join-Path $repoRoot 'out/build/windows-x64-release/Release'
$x86 = Join-Path $repoRoot 'out/build/windows-x86-release/Release'
$bootstrap = Join-Path $x64 'fcitx5-bootstrap.exe'
$approvedIcon = Join-Path $repoRoot 'resources/icons/fcitx5.ico'
$work = Join-Path $outRoot ('stage-' + [guid]::NewGuid().ToString('N'))
$root = Join-Path $work 'Fcitx5'
$artifacts = Join-Path $outRoot 'artifacts'
$sourceCommit = (git -C $repoRoot rev-parse HEAD).Trim()
if ([string]::IsNullOrWhiteSpace($Generation)) {
  $Generation = (git -C $repoRoot rev-parse --short=12 HEAD).Trim().ToLowerInvariant()
}
if ($Generation -notmatch '^[a-z0-9_-]{1,32}$') {
  throw "Invalid release generation '$Generation'. Use 1-32 chars: lowercase letters, digits, '_' or '-'."
}

$required = @(
  (Join-Path $core 'bin/fcitx5-engine.exe'),
  (Join-Path $x64 'fcitx5-config.exe'), (Join-Path $x64 'fcitx5-control.exe'),
  (Join-Path $x64 'fcitx5-launcher.exe'), (Join-Path $x64 'fcitx5-ui.exe'),
  (Join-Path $x64 'fcitx5-package.exe'), (Join-Path $x64 'fcitx5-downloader.exe'),
  (Join-Path $x64 'fcitx5-deployer.exe'), (Join-Path $x64 'fcitx5-provider.exe'),
  (Join-Path $x64 'fcitx5-updater.exe'),
  $bootstrap,
  (Join-Path $x64 'fcitx5-release-pqc-signer.exe'),
  (Join-Path $x64 'fcitx5-register.exe'), (Join-Path $x64 'fcitx5-tsf.dll'),
  (Join-Path $x86 'fcitx5-register.exe'), (Join-Path $x86 'fcitx5-tsf.dll'))
foreach ($path in $required) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing package input: $path" }
}
if (-not (Test-Path -LiteralPath $approvedIcon -PathType Leaf)) {
  throw "Missing approved application icon: $approvedIcon"
}
$bootstrapInfo = Get-Item -LiteralPath $bootstrap
$iconInfo = Get-Item -LiteralPath $approvedIcon
if ($bootstrapInfo.LastWriteTimeUtc -lt $iconInfo.LastWriteTimeUtc) {
  throw "Stale fcitx5-bootstrap.exe: rebuild after resources/icons/fcitx5.ico changed."
}

if (-not ('Fcitx5.StagePeResources' -as [type])) {
  Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

namespace Fcitx5
{
public static class StagePeResources
{
    private const uint LoadLibraryAsDatafile = 0x00000002;
    private const uint LoadLibraryAsImageResource = 0x00000020;
    private const int RtGroupIcon = 14;

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    private static extern IntPtr LoadLibraryEx(string fileName, IntPtr file, uint flags);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern bool FreeLibrary(IntPtr module);

    public static bool HasGroupIcon(string fileName, int resourceId)
    {
        var module = LoadLibraryEx(fileName, IntPtr.Zero,
            LoadLibraryAsDatafile | LoadLibraryAsImageResource);
        if (module == IntPtr.Zero)
            return false;
        try
        {
            return FindResource(module, new IntPtr(resourceId),
                new IntPtr(RtGroupIcon)) != IntPtr.Zero;
        }
        finally
        {
            FreeLibrary(module);
        }
    }
}
}
'@ -Language CSharp -ErrorAction Stop
}

function Assert-ApprovedGroupIcon([string] $path, [string] $label) {
  if (-not [Fcitx5.StagePeResources]::HasGroupIcon($path, 101)) {
    throw "$label is missing RT_GROUP_ICON resource 101 from resources/icons/fcitx5.ico: $path"
  }
}

Assert-ApprovedGroupIcon $bootstrap 'fcitx5-bootstrap.exe'

New-Item -ItemType Directory -Force -Path $root, $artifacts | Out-Null
Copy-Item -LiteralPath (Join-Path $core 'bin') -Destination $root -Recurse
Copy-Item -LiteralPath (Join-Path $core 'lib') -Destination $root -Recurse
Copy-Item -LiteralPath (Join-Path $core 'share') -Destination $root -Recurse
# Drop build-time leftovers of the native install: CMake/pkg-config metadata,
# import libraries, Linux desktop integration files, shell scripts and the
# libime/Sogou dictionary build tools, none of which the Windows runtime loads.
foreach ($relative in @('lib/cmake', 'lib/pkgconfig', 'share/cmake', 'share/metainfo',
                         'share/applications', 'share/fcitx5/testing',
                         'bin/fcitx5-configtool', 'bin/fcitx5-diagnose',
                         'bin/scel2org5.exe')) {
  $path = Join-Path $root $relative
  if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Recurse -Force }
}
Get-ChildItem -LiteralPath (Join-Path $root 'bin') -Filter 'libime_*' -File |
  Remove-Item -Force
Get-ChildItem -LiteralPath (Join-Path $root 'lib') -Filter '*.a' -File |
  Remove-Item -Force
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
  Copy-Item -LiteralPath $bootstrap `
    -Destination (Join-Path $root $name) -Force
}

$bootstrapHash = (Get-FileHash -LiteralPath $bootstrap -Algorithm SHA256).Hash
foreach ($name in @('Start Fcitx5.exe', 'Fcitx5 Settings.exe',
                     'Unregister Fcitx5.exe')) {
  $finalPath = Join-Path $root $name
  Assert-ApprovedGroupIcon $finalPath $name
  $finalHash = (Get-FileHash -LiteralPath $finalPath -Algorithm SHA256).Hash
  if ($finalHash -ne $bootstrapHash) {
    throw "$name does not preserve the approved bootstrap PE resource chain."
  }
}
Copy-Item -LiteralPath (Join-Path $x86 'fcitx5-register.exe') `
  -Destination (Join-Path $bin 'fcitx5-register-x86.exe') -Force
New-Item -ItemType Directory -Force -Path (Join-Path $root 'tsf/x64'),
  (Join-Path $root 'tsf/x86'), (Join-Path $bin 'locales'),
  (Join-Path $root 'themes'), (Join-Path $root 'data'),
  (Join-Path $root 'security') | Out-Null
Copy-Item -LiteralPath (Join-Path $x64 'fcitx5-tsf.dll') -Destination (Join-Path $root 'tsf/x64')
Copy-Item -LiteralPath (Join-Path $x86 'fcitx5-tsf.dll') -Destination (Join-Path $root 'tsf/x86')
[IO.File]::WriteAllText((Join-Path $root 'tsf/x64/fcitx5-tsf.generation'),
  "$Generation`n", [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $root 'tsf/x86/fcitx5-tsf.generation'),
  "$Generation`n", [Text.UTF8Encoding]::new($false))
Copy-Item -Path (Join-Path $repoRoot 'locales/*') -Destination (Join-Path $bin 'locales')
Copy-Item -Path (Join-Path $repoRoot 'resources/themes/*') `
  -Destination (Join-Path $root 'themes') -Recurse
Copy-Item -LiteralPath (Join-Path $repoRoot 'security/trusted-keys.template.json') `
  -Destination (Join-Path $root 'security/trusted-keys.json')
[IO.Directory]::CreateDirectory((Join-Path $outRoot 'release-tools')) | Out-Null
Copy-Item -LiteralPath (Join-Path $x64 'fcitx5-release-pqc-signer.exe') `
  -Destination (Join-Path $outRoot 'release-tools/fcitx5-release-pqc-signer.exe') -Force
[IO.File]::WriteAllText((Join-Path $root 'portable.flag'), "portable`n",
  [Text.UTF8Encoding]::new($false))

$runtimeRoot = Join-Path $root (Join-Path 'runtime' $Generation)
New-Item -ItemType Directory -Force -Path $runtimeRoot | Out-Null
foreach ($name in @('bin', 'lib', 'share', 'themes', 'data')) {
  $source = Join-Path $root $name
  if (Test-Path -LiteralPath $source) {
    Copy-Item -LiteralPath $source -Destination $runtimeRoot -Recurse
  }
}
Copy-Item -LiteralPath (Join-Path $root 'portable.flag') -Destination $runtimeRoot -Force
$currentGeneration = [ordered]@{
  format_version = 1
  current_generation = $Generation
  previous_generation = ''
  build_id = $sourceCommit
}
[IO.File]::WriteAllText((Join-Path $root 'current.json'),
  (($currentGeneration | ConvertTo-Json -Depth 3) + "`n"), [Text.UTF8Encoding]::new($false))

# The runtime generation is a derived copy of the top-level payload: the
# installer stores it once and portable copies create it on first start, so
# the manifest lists only the top-level files.
$runtimeParent = Join-Path $root 'runtime'
$runtimePrefix = $runtimeParent + [IO.Path]::DirectorySeparatorChar
$files = Get-ChildItem -LiteralPath $root -File -Recurse |
  Where-Object { -not $_.FullName.StartsWith($runtimePrefix, [StringComparison]::OrdinalIgnoreCase) } |
  Sort-Object FullName | ForEach-Object {
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
  release_generation = $Generation
  source_commit = $sourceCommit
  source_tree_clean = @((git -C $repoRoot status --porcelain=v1 --untracked-files=all)).Count -eq 0
  architecture = 'x64-with-x86-tsf'
  files = @($files)
}
[IO.File]::WriteAllText((Join-Path $root 'manifest.json'),
  (($manifest | ConvertTo-Json -Depth 5) + "`n"), [Text.UTF8Encoding]::new($false))

function Write-PayloadSizeReport([string] $payloadRoot) {
  $all = @(Get-ChildItem -LiteralPath $payloadRoot -File -Recurse)
  $total = ($all | Measure-Object -Property Length -Sum).Sum
  Write-Host ('Payload: {0} files, {1:N1} MiB' -f $all.Count, ($total / 1MB))
  $all | Group-Object {
    $parts = [IO.Path]::GetRelativePath($payloadRoot, $_.FullName).Replace('\', '/').Split('/')
    if ($parts.Count -gt 2) { $parts[0] + '/' + $parts[1] } else { $parts[0] }
  } | ForEach-Object {
    [pscustomobject]@{
      Directory = $_.Name
      Files = $_.Count
      MiB = [math]::Round((($_.Group | Measure-Object -Property Length -Sum).Sum / 1MB), 2)
    }
  } | Sort-Object MiB -Descending | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
  $all | Sort-Object Length -Descending | Select-Object -First 40 | ForEach-Object {
    [pscustomobject]@{
      Path = [IO.Path]::GetRelativePath($payloadRoot, $_.FullName).Replace('\', '/')
      MiB = [math]::Round($_.Length / 1MB, 2)
    }
  } | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
}
Write-PayloadSizeReport $root

$artifactSuffix = if ($Channel -eq 'stable') { '' } else { "-$Channel" }
$portable = Join-Path $artifacts "fcitx5-windows-$Version$artifactSuffix-portable.zip"
# Portable archives leave the runtime generation out; "Start Fcitx5.exe"
# creates it from the top-level payload on first start.
$heldRuntime = Join-Path $work 'runtime-held'
Move-Item -LiteralPath $runtimeParent -Destination $heldRuntime
try {
  Compress-Archive -Path $root -DestinationPath $portable -CompressionLevel Optimal -Force
} finally {
  Move-Item -LiteralPath $heldRuntime -Destination $runtimeParent
}
if (-not $SkipInstaller) {
  & (Join-Path $PSScriptRoot 'prepare-package-toolchains.ps1')
  $iscc = Join-Path $repoRoot 'out/toolchains/inno-7.0.2/ISCC.exe'
  # Build to the immutable per-stage directory first. Antivirus and Explorer may briefly hold the
  # previously published fixed-name installer; compiling directly over it makes that transient
  # Windows lock look like a source/build failure.
  $installerOutput = Join-Path $work 'installer-output'
  New-Item -ItemType Directory -Force -Path $installerOutput | Out-Null
  & $iscc "/DProductVersion=$Version" "/DReleaseChannel=$Channel" `
    "/DReleaseGeneration=$Generation" `
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
Get-ChildItem -LiteralPath $artifacts -File | ForEach-Object {
  Write-Host ('{0}: {1:N1} MiB' -f $_.Name, ($_.Length / 1MB))
}
Write-Host "Package artifacts staged from one build: $artifacts"
