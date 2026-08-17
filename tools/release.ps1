[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $Version,
  [ValidateSet('stable', 'beta', 'nightly')] [string] $Channel = 'stable',
  [Parameter(Mandatory)] [string] $CertificateThumbprint,
  [Parameter(Mandatory)] [string] $TrustedKeyring,
  [string] $TimestampUrl = 'http://timestamp.digicert.com'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
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
if ($keyring.format_version -ne 1 -or $keyring.keys.Count -lt 1) {
  throw 'Stable release keyring must contain at least one trusted or revoked key.'
}
Copy-Item -LiteralPath $TrustedKeyring -Destination (Join-Path $stage 'security/trusted-keys.json') -Force
& (Join-Path $stage 'bin/fcitx5-package.exe') --validate-keyring `
  (Join-Path $stage 'security/trusted-keys.json')
if ($LASTEXITCODE -ne 0) { throw 'Protected trusted keyring is not usable by Package Core.' }
$sourceCommit = (git -C $repoRoot rev-parse HEAD).Trim()
$manifest = Get-Content -LiteralPath (Join-Path $stage 'manifest.json') -Raw | ConvertFrom-Json
if ($manifest.version -ne $Version -or $manifest.channel -ne $Channel -or
    $manifest.source_commit -ne $sourceCommit) {
  throw 'Tested stage lineage does not match requested release identity.'
}
$peFiles = @(Get-ChildItem -LiteralPath $stage -File -Recurse -Include *.exe,*.dll |
  Select-Object -ExpandProperty FullName)
& (Join-Path $PSScriptRoot 'sign-release.ps1') -Paths $peFiles `
  -CertificateThumbprint $CertificateThumbprint -TimestampUrl $TimestampUrl

$files = @(Get-ChildItem -LiteralPath $stage -File -Recurse |
  Where-Object Name -ne 'manifest.json' | Sort-Object FullName | ForEach-Object {
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
Compress-Archive -Path $stage -DestinationPath $portable -CompressionLevel Optimal -Force
& (Join-Path $PSScriptRoot 'prepare-package-toolchains.ps1')
$iscc = Join-Path $repoRoot 'out/toolchains/inno-7.0.2/ISCC.exe'
& $iscc "/DProductVersion=$Version" "/DReleaseChannel=$Channel" "/DStageDir=$stage" `
  "/DArtifactDir=$artifacts" (Join-Path $repoRoot 'installer/fcitx5-windows.iss')
if ($LASTEXITCODE -ne 0) { throw 'Release installer packaging failed.' }
$installer = Join-Path $artifacts "fcitx5-windows-$Version$suffix-setup.exe"
& (Join-Path $PSScriptRoot 'sign-release.ps1') -Paths @($installer) `
  -CertificateThumbprint $CertificateThumbprint -TimestampUrl $TimestampUrl
& (Join-Path $PSScriptRoot 'test-installer.ps1') -Version $Version `
  -InstallerPath $installer -Elevated
$sbom = Join-Path $artifacts "fcitx5-windows-$Version$suffix.spdx.json"
& (Join-Path $PSScriptRoot 'generate-sbom.ps1') -StageRoot $stage -OutputPath $sbom `
  -Version $Version -SourceCommit $sourceCommit
$releaseFiles = @($portable, $installer, $sbom)
$releaseManifest = [ordered]@{ format_version = 1; product = 'fcitx5-windows-next';
  version = $Version; channel = $Channel; source_commit = $sourceCommit;
  build_once_stage_manifest_sha256 = (Get-FileHash (Join-Path $stage 'manifest.json') -Algorithm SHA256).Hash.ToLowerInvariant();
  artifacts = @($releaseFiles | ForEach-Object { [ordered]@{ name = [IO.Path]::GetFileName($_);
    size = (Get-Item $_).Length; sha256 = (Get-FileHash $_ -Algorithm SHA256).Hash.ToLowerInvariant() } }) }
$releaseManifestPath = Join-Path $artifacts "release-manifest-$Version$suffix.json"
[IO.File]::WriteAllText($releaseManifestPath, (($releaseManifest | ConvertTo-Json -Depth 8) + "`n"),
  [Text.UTF8Encoding]::new($false))
Add-Type -AssemblyName System.Security.Cryptography.Pkcs
$cert = Get-Item "Cert:\CurrentUser\My\$($CertificateThumbprint.Replace(' ',''))" -ErrorAction Stop
$content = [IO.File]::ReadAllBytes($releaseManifestPath)
$cms = [Security.Cryptography.Pkcs.SignedCms]::new(
  [Security.Cryptography.Pkcs.ContentInfo]::new($content), $true)
$cms.ComputeSignature([Security.Cryptography.Pkcs.CmsSigner]::new($cert))
[IO.File]::WriteAllBytes($releaseManifestPath + '.p7s', $cms.Encode())
& (Join-Path $PSScriptRoot 'generate-system-packages.ps1') `
  -ReleaseManifest $releaseManifestPath `
  -BaseUrl "https://github.com/fcitx/fcitx5-windows/releases/download/v$Version" `
  -OutputDirectory (Join-Path $artifacts 'system-packages')
$provenance = [ordered]@{ _type = 'https://in-toto.io/Statement/v1';
  subject = @($releaseManifest.artifacts | ForEach-Object { [ordered]@{ name = $_.name; digest = [ordered]@{ sha256 = $_.sha256 } } });
  predicateType = 'https://slsa.dev/provenance/v1'; predicate = [ordered]@{
    buildDefinition = [ordered]@{ buildType = 'https://fcitx5-windows.org/build/v1'; externalParameters = [ordered]@{ version = $Version; channel = $Channel }; resolvedDependencies = @([ordered]@{ uri = "git+$repoRoot"; digest = [ordered]@{ gitCommit = $sourceCommit } }) };
    runDetails = [ordered]@{ builder = [ordered]@{ id = 'tools/release.ps1' }; metadata = [ordered]@{ invocationId = [guid]::NewGuid().ToString() } } } }
[IO.File]::WriteAllText((Join-Path $artifacts "provenance-$Version$suffix.json"),
  (($provenance | ConvertTo-Json -Depth 12) + "`n"), [Text.UTF8Encoding]::new($false))
& (Join-Path $PSScriptRoot 'test-release-artifacts.ps1') `
  -ArtifactDirectory $artifacts -ReleaseManifest $releaseManifestPath
Write-Host "Signed release gate passed without rebuilding source: $artifacts"
