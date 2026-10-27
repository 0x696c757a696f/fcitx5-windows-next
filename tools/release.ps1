[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $Version,
  [ValidateSet('stable', 'beta', 'nightly')] [string] $Channel = 'stable',
  [string] $CertificateThumbprint = '',
  # Publish without Authenticode signatures while no code signing certificate is available.
  # Packages, the plugin index and the release manifest are still ML-DSA-65 + Ed25519 signed.
  [switch] $WithoutAuthenticode,
  [Parameter(Mandatory)] [string] $TrustedKeyring,
  [Parameter(Mandatory)] [string] $PluginSigningKey,
  [Parameter(Mandatory)] [string] $Ed25519SigningKey,
  [Parameter(Mandatory)] [ValidateRange(1, [UInt64]::MaxValue)] [UInt64] $PluginReleaseSequence,
  [string] $TimestampUrl = 'http://timestamp.digicert.com'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($WithoutAuthenticode -eq [bool]$CertificateThumbprint) {
  throw 'Pass either -CertificateThumbprint or -WithoutAuthenticode, not both or neither.'
}
$authenticode = -not $WithoutAuthenticode
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$dirty = @(git -C $repoRoot status --porcelain=v1 --untracked-files=all)
if ($LASTEXITCODE -ne 0 -or $dirty.Count -ne 0) {
  throw 'Release promotion requires a clean source tree; package/dev stages may be dirty but cannot be signed.'
}
$outRoot = Join-Path $repoRoot 'out/package'
$stagePointer = Join-Path $outRoot 'current-stage.txt'
if (-not (Test-Path -LiteralPath $stagePointer -PathType Leaf)) {
  throw 'No tested package stage exists. Run tools/build.ps1 package first.'
}
$stage = [IO.Path]::GetFullPath((Get-Content -LiteralPath $stagePointer -Raw).Trim())
$stagePrefix = [IO.Path]::GetFullPath($outRoot).TrimEnd('\') + '\'
if (-not $stage.StartsWith($stagePrefix, [StringComparison]::OrdinalIgnoreCase) -or
    -not (Test-Path -LiteralPath $stage -PathType Container)) { throw 'Stage pointer is outside the package output.' }
$keyring = Get-Content -LiteralPath $TrustedKeyring -Raw | ConvertFrom-Json
if ($keyring.format_version -ne 2 -or $keyring.keys.Count -lt 1) {
  throw 'Stable release keyring must be v2 and contain at least one trusted or revoked key.'
}
Copy-Item -LiteralPath $TrustedKeyring -Destination (Join-Path $stage 'security/trusted-keys.json') -Force
& (Join-Path $stage 'bin/fcitx5-package.exe') --validate-keyring `
  (Join-Path $stage 'security/trusted-keys.json')
if ($LASTEXITCODE -ne 0) { throw 'Protected trusted keyring is not usable by Package Core.' }
$sourceCommit = (git -C $repoRoot rev-parse HEAD).Trim()
$manifest = Get-Content -LiteralPath (Join-Path $stage 'manifest.json') -Raw | ConvertFrom-Json
if ($manifest.version -ne $Version -or $manifest.channel -ne $Channel -or
    $manifest.source_commit -ne $sourceCommit -or $manifest.source_tree_clean -ne $true) {
  throw 'Tested stage lineage does not match requested release identity.'
}
$generation = [string] $manifest.release_generation
if ($generation -notmatch '^[a-z0-9_-]{1,32}$') { throw 'Tested stage has no valid release generation.' }
$runtimeRoot = Join-Path $stage (Join-Path 'runtime' $generation)
$runtimePrefix = $runtimeRoot.TrimEnd('\') + '\'
# The runtime generation mirrors the top-level bin/lib/share trees. Sign only the
# top-level copies and mirror the signed bytes so both stay identical; the
# installer stores each mirrored file once.
$peFiles = @(Get-ChildItem -LiteralPath $stage -File -Recurse -Include *.exe,*.dll |
  Select-Object -ExpandProperty FullName |
  Where-Object { -not $_.StartsWith($runtimePrefix, [StringComparison]::OrdinalIgnoreCase) })
if ($authenticode) {
  & (Join-Path $PSScriptRoot 'sign-release.ps1') -Paths $peFiles `
    -CertificateThumbprint $CertificateThumbprint -TimestampUrl $TimestampUrl
}
foreach ($path in $peFiles) {
  $relative = [IO.Path]::GetRelativePath($stage, $path)
  $mirror = Join-Path $runtimeRoot $relative
  if (Test-Path -LiteralPath $mirror -PathType Leaf) {
    Copy-Item -LiteralPath $path -Destination $mirror -Force
  }
}
$forkPluginRoot = Join-Path $outRoot 'fork-plugins'
& (Join-Path $PSScriptRoot 'prepare-fork-plugins.ps1') -StageRoot $stage -OutputRoot $forkPluginRoot
# Fork plugins come from another repository's release; sign their binaries too
# so every DLL a user installs carries the same publisher signature.
$forkPluginPe = @(Get-ChildItem -LiteralPath $forkPluginRoot -File -Recurse -Include *.exe,*.dll |
  Select-Object -ExpandProperty FullName)
if ($authenticode -and $forkPluginPe.Count -gt 0) {
  & (Join-Path $PSScriptRoot 'sign-release.ps1') -Paths $forkPluginPe `
    -CertificateThumbprint $CertificateThumbprint -TimestampUrl $TimestampUrl
}
& (Join-Path $PSScriptRoot 'release-plugin-artifacts.ps1') `
  -Version $Version -Channel $Channel -ReleaseSequence $PluginReleaseSequence -StageRoot $stage `
  -TrustedKeyring $TrustedKeyring -SigningKey $PluginSigningKey -Ed25519SigningKey $Ed25519SigningKey `
  -ForkPayloadRoot $forkPluginRoot

$runtimeParent = Join-Path $stage 'runtime'
$files = @(Get-ChildItem -LiteralPath $stage -File -Recurse |
  Where-Object { $_.Name -ne 'manifest.json' -and
    -not $_.FullName.StartsWith($runtimeParent + '\', [StringComparison]::OrdinalIgnoreCase) } |
  Sort-Object FullName | ForEach-Object {
    [ordered]@{ path = [IO.Path]::GetRelativePath($stage, $_.FullName).Replace('\', '/');
      size = $_.Length; sha256 = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
  })
$manifest.files = $files
[IO.File]::WriteAllText((Join-Path $stage 'manifest.json'),
  (($manifest | ConvertTo-Json -Depth 8) + "`n"), [Text.UTF8Encoding]::new($false))
$artifacts = Join-Path $outRoot 'release-artifacts'
New-Item -ItemType Directory -Force -Path $artifacts | Out-Null
$suffix = if ($Channel -eq 'stable') { '' } else { "-$Channel" }
$portable = Join-Path $artifacts "fcitx5-windows-$Version$suffix-portable.zip"
$heldRuntime = Join-Path (Split-Path -Parent $stage) 'runtime-held'
Move-Item -LiteralPath $runtimeParent -Destination $heldRuntime
try {
  Compress-Archive -Path $stage -DestinationPath $portable -CompressionLevel Optimal -Force
} finally {
  Move-Item -LiteralPath $heldRuntime -Destination $runtimeParent
}
& (Join-Path $PSScriptRoot 'prepare-package-toolchains.ps1')
$iscc = Join-Path $repoRoot 'out/toolchains/inno-7.0.2/ISCC.exe'
$isccSigning = @()
if ($authenticode) {
  $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
  $signtool = Get-ChildItem -LiteralPath $kits -Filter signtool.exe -File -Recurse |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
  if (-not $signtool) { throw 'SignTool was not found.' }
  $thumbprint = $CertificateThumbprint.Replace(' ', '').ToUpperInvariant()
  $uninstallerSign = "`$q$signtool`$q sign /sha1 $thumbprint /fd SHA256"
  if ($TimestampUrl) { $uninstallerSign += " /tr $TimestampUrl /td SHA256" }
  $uninstallerSign += ' $f'
  $isccSigning = @("/Sfcitx5sign=$uninstallerSign", '/DSignToolName=fcitx5sign')
}
& $iscc @isccSigning `
  "/DProductVersion=$Version" "/DReleaseChannel=$Channel" `
  "/DReleaseGeneration=$generation" "/DStageDir=$stage" "/DArtifactDir=$artifacts" (Join-Path $repoRoot 'installer/fcitx5-windows.iss')
if ($LASTEXITCODE -ne 0) { throw 'Release installer packaging failed.' }
$installer = Join-Path $artifacts "fcitx5-windows-$Version$suffix-setup.exe"
if ($authenticode) {
  & (Join-Path $PSScriptRoot 'sign-release.ps1') -Paths @($installer) `
    -CertificateThumbprint $CertificateThumbprint -TimestampUrl $TimestampUrl
}
& (Join-Path $PSScriptRoot 'test-installer.ps1') -Version $Version `
  -InstallerPath $installer -Elevated
$sbom = Join-Path $artifacts "fcitx5-windows-$Version$suffix.spdx.json"
& (Join-Path $PSScriptRoot 'generate-sbom.ps1') -StageRoot $stage -OutputPath $sbom `
  -Version $Version -SourceCommit $sourceCommit
# Flat list: a nested array here would reach the release manifest as one entry
# named after the last archive, with lists for its size and hash.
$pluginReleaseFiles = @(Get-ChildItem -LiteralPath $artifacts -Filter '*.fcpkg' -File |
    Select-Object -ExpandProperty FullName) +
  @((Join-Path $artifacts 'index.json'), (Join-Path $artifacts 'index.sig.json'))
foreach ($path in $pluginReleaseFiles) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Missing signed plugin release asset: $path" }
}
$releaseFiles = @($portable, $installer, $sbom) + $pluginReleaseFiles
$releaseManifest = [ordered]@{ format_version = 1; product = 'fcitx5-windows-next';
  version = $Version; channel = $Channel; source_commit = $sourceCommit;
  build_once_stage_manifest_sha256 = (Get-FileHash (Join-Path $stage 'manifest.json') -Algorithm SHA256).Hash.ToLowerInvariant();
  artifacts = @($releaseFiles | ForEach-Object { [ordered]@{ name = [IO.Path]::GetFileName($_);
    size = (Get-Item $_).Length; sha256 = (Get-FileHash $_ -Algorithm SHA256).Hash.ToLowerInvariant() } }) }
$releaseManifestPath = Join-Path $artifacts "release-manifest-$Version$suffix.json"
[IO.File]::WriteAllText($releaseManifestPath, (($releaseManifest | ConvertTo-Json -Depth 8) + "`n"),
  [Text.UTF8Encoding]::new($false))
# The release manifest pins the SHA-256 of the installer, the portable archive
# and every plugin asset, so its dual signature covers all of them.
$packageCli = Join-Path $stage 'bin/fcitx5-package.exe'
& (Join-Path $PSScriptRoot 'sign-release-metadata.ps1') -Path $releaseManifestPath `
  -SignedObject 'release-manifest' -SignaturePath ($releaseManifestPath -replace '\.json$', '.sig.json') `
  -TrustedKeyring $TrustedKeyring -SigningKey $PluginSigningKey -Ed25519SigningKey $Ed25519SigningKey `
  -Signer (Join-Path $outRoot 'release-tools/fcitx5-release-pqc-signer.exe') -PackageCli $packageCli
if ($authenticode) {
  Add-Type -AssemblyName System.Security.Cryptography.Pkcs
  $cert = Get-Item "Cert:\CurrentUser\My\$($CertificateThumbprint.Replace(' ',''))" -ErrorAction Stop
  $content = [IO.File]::ReadAllBytes($releaseManifestPath)
  $cms = [Security.Cryptography.Pkcs.SignedCms]::new(
    [Security.Cryptography.Pkcs.ContentInfo]::new($content), $true)
  $cms.ComputeSignature([Security.Cryptography.Pkcs.CmsSigner]::new($cert))
  [IO.File]::WriteAllBytes($releaseManifestPath + '.p7s', $cms.Encode())
}
& (Join-Path $PSScriptRoot 'generate-system-packages.ps1') `
  -ReleaseManifest $releaseManifestPath `
  -BaseUrl "https://github.com/0x696c757a696f/fcitx5-windows-next/releases/download/v$Version" `
  -OutputDirectory (Join-Path $artifacts 'system-packages')
$provenance = [ordered]@{ _type = 'https://in-toto.io/Statement/v1';
  subject = @($releaseManifest.artifacts | ForEach-Object { [ordered]@{ name = $_.name; digest = [ordered]@{ sha256 = $_.sha256 } } });
  predicateType = 'https://slsa.dev/provenance/v1'; predicate = [ordered]@{
    buildDefinition = [ordered]@{ buildType = 'https://fcitx5-windows.org/build/v1'; externalParameters = [ordered]@{ version = $Version; channel = $Channel }; resolvedDependencies = @([ordered]@{ uri = "git+$repoRoot"; digest = [ordered]@{ gitCommit = $sourceCommit } }) };
    runDetails = [ordered]@{ builder = [ordered]@{ id = 'tools/release.ps1' }; metadata = [ordered]@{ invocationId = [guid]::NewGuid().ToString() } } } }
[IO.File]::WriteAllText((Join-Path $artifacts "provenance-$Version$suffix.json"),
  (($provenance | ConvertTo-Json -Depth 12) + "`n"), [Text.UTF8Encoding]::new($false))
& (Join-Path $PSScriptRoot 'test-release-artifacts.ps1') `
  -ArtifactDirectory $artifacts -ReleaseManifest $releaseManifestPath `
  -PackageCli $packageCli -TrustedKeyring $TrustedKeyring `
  -WithoutAuthenticode:$WithoutAuthenticode
Write-Host "Signed release gate passed without rebuilding source: $artifacts"
