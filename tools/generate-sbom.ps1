[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $StageRoot,
  [Parameter(Mandatory)] [string] $OutputPath,
  [Parameter(Mandatory)] [string] $Version,
  [Parameter(Mandatory)] [string] $SourceCommit
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
. (Join-Path $PSScriptRoot 'cargo-inventory.ps1')
$stage = [IO.Path]::GetFullPath($StageRoot)
$output = [IO.Path]::GetFullPath($OutputPath)
if (-not (Test-Path -LiteralPath $stage -PathType Container)) { throw 'SBOM stage root is missing.' }
$inventory = Get-Content -LiteralPath (Join-Path $repoRoot 'third_party/dependencies.json') -Raw |
  ConvertFrom-Json
$cargoLockPath = Join-Path $repoRoot 'Cargo.lock'
$cargoRegistryPackages = @(Assert-CargoInventoryMatchesManifest `
    -CargoLockPath $cargoLockPath -Manifest $inventory)
$packages = @(
  [ordered]@{
    SPDXID = 'SPDXRef-Product'
    name = 'fcitx5-windows-next'
    versionInfo = $Version
    downloadLocation = 'NOASSERTION'
    filesAnalyzed = $true
    licenseConcluded = 'GPL-3.0-or-later'
    licenseDeclared = 'GPL-3.0-or-later'
    supplier = 'Organization: Fcitx5 for Windows Next contributors'
  }
)
foreach ($dependency in $inventory.packages) {
  $safe = ([string]$dependency.name -replace '[^A-Za-z0-9.-]', '-')
  $packages += [ordered]@{
    SPDXID = "SPDXRef-Package-$safe"
    name = [string]$dependency.name
    versionInfo = [string]$dependency.version
    downloadLocation = [string]$dependency.source
    filesAnalyzed = $false
    licenseConcluded = [string]$dependency.license
    licenseDeclared = [string]$dependency.license
    supplier = 'NOASSERTION'
  }
}
$cargoPackageCount = $cargoRegistryPackages.Count
$files = @(Get-ChildItem -LiteralPath $stage -File -Recurse | Sort-Object FullName | ForEach-Object {
  $relative = [IO.Path]::GetRelativePath($stage, $_.FullName).Replace('\', '/')
  $id = 'SPDXRef-File-' + (([Text.Encoding]::UTF8.GetBytes($relative) | ForEach-Object { $_.ToString('x2') }) -join '')
  [ordered]@{
    SPDXID = $id
    fileName = './' + $relative
    checksums = @([ordered]@{ algorithm = 'SHA256'; checksumValue =
      (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() })
    licenseConcluded = 'NOASSERTION'
    copyrightText = 'NOASSERTION'
  }
})
$relationships = @([ordered]@{ spdxElementId = 'SPDXRef-DOCUMENT'; relationshipType = 'DESCRIBES'; relatedSpdxElement = 'SPDXRef-Product' })
foreach ($package in $packages | Select-Object -Skip 1) {
  $relationships += [ordered]@{ spdxElementId = 'SPDXRef-Product'; relationshipType = 'DEPENDS_ON'; relatedSpdxElement = $package.SPDXID }
}
$document = [ordered]@{
  spdxVersion = 'SPDX-2.3'
  dataLicense = 'CC0-1.0'
  SPDXID = 'SPDXRef-DOCUMENT'
  name = "fcitx5-windows-$Version"
  documentNamespace = "https://fcitx5-windows.org/spdx/$Version/$SourceCommit"
  creationInfo = [ordered]@{ created = [DateTime]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ'); creators = @('Tool: tools/generate-sbom.ps1') }
  packages = $packages
  files = $files
  relationships = $relationships
}
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $output) | Out-Null
[IO.File]::WriteAllText($output, (($document | ConvertTo-Json -Depth 12) + "`n"),
  [Text.UTF8Encoding]::new($false))
Write-Host "SPDX SBOM: $output ($cargoPackageCount Cargo registry packages verified against inventory)"
