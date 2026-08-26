Set-StrictMode -Version Latest

function Get-CargoRegistryPackages {
  param([Parameter(Mandatory)] [string] $CargoLockPath)

  if (-not (Test-Path -LiteralPath $CargoLockPath -PathType Leaf)) {
    return @()
  }
  $cargoLock = Get-Content -LiteralPath $CargoLockPath -Raw
  $packageMatches = [regex]::Matches(
    $cargoLock,
    '(?ms)\[\[package\]\]\s+name = "([^"]+)".*?(?=\n\[\[package\]\]|\z)'
  )
  $packages = @()
  foreach ($match in $packageMatches) {
    $packageBlock = $match.Value
    $isRegistryCrate = $packageBlock -match '(?m)^\s*source\s*=\s*"registry\+https://github\.com/rust-lang/crates\.io-index"\s*$'
    if (-not $isRegistryCrate) {
      continue
    }
    $version = ''
    if ($packageBlock -match '(?m)^\s*version\s*=\s*"([^"]+)"\s*$') {
      $version = $Matches[1]
    }
    $packages += [pscustomobject]@{
      Name = $match.Groups[1].Value
      NormalizedName = ([string]$match.Groups[1].Value).Replace('-', '_')
      Version = $version
    }
  }
  return $packages
}

function Assert-CargoInventoryMatchesManifest {
  param(
    [Parameter(Mandatory)] [string] $CargoLockPath,
    [Parameter(Mandatory)] $Manifest
  )

  $cargoPackages = @(Get-CargoRegistryPackages -CargoLockPath $CargoLockPath)
  $manifestByNameVersion = @{}
  foreach ($package in @($Manifest.packages)) {
    $packageName = [string]$package.name
    if (-not $packageName.StartsWith('rust-crate-', [StringComparison]::Ordinal)) {
      continue
    }
    $crateName = if ($null -ne $package.PSObject.Properties['crate']) {
      [string]$package.crate
    } else {
      $packageName.Substring('rust-crate-'.Length)
    }
    $normalized = $crateName.Replace('-', '_')
    $version = [string]$package.version
    $key = "${normalized}@${version}"
    if ($manifestByNameVersion.ContainsKey($key)) {
      throw "Duplicate Cargo dependency inventory records normalize to '$key'."
    }
    $manifestByNameVersion[$key] = $package
  }

  $untrackedCargoPackages = [System.Collections.Generic.List[string]]::new()
  $mismatchedCargoPackages = [System.Collections.Generic.List[string]]::new()
  foreach ($package in $cargoPackages) {
    $key = "$($package.NormalizedName)@$($package.Version)"
    if (-not $manifestByNameVersion.ContainsKey($key)) {
      $untrackedCargoPackages.Add($package.Name)
      continue
    }
    $manifestRecord = $manifestByNameVersion[$key]
    if ([string]$manifestRecord.version -ne $package.Version) {
      $mismatchedCargoPackages.Add(
        "$($package.Name) Cargo.lock=$($package.Version) inventory=$($manifestRecord.version)"
      )
    }
  }

  if ($untrackedCargoPackages.Count -gt 0) {
    throw "Cargo.lock contains untracked third-party crate sources; add Cargo dependency inventory/SBOM/license review:`n$($untrackedCargoPackages -join "`n")"
  }
  if ($mismatchedCargoPackages.Count -gt 0) {
    throw "Cargo.lock crate versions differ from third_party/dependencies.json:`n$($mismatchedCargoPackages -join "`n")"
  }

  return $cargoPackages
}
