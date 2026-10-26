[CmdletBinding()]
param(
  [Parameter(Mandatory)] [string] $StageRoot,
  [string] $OutputRoot = '',
  [string] $CacheRoot = '',
  [string] $SystemRoot = ''
)

# Downloads the pinned fcitx5-plugins release, verifies every archive against the
# inventory, and lays each package's payload out under $OutputRoot/<id>. DLLs the
# verified stage already ships are dropped, and every PE file left must resolve
# all of its imports, by name, against the package, the stage or the system.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$stage = [IO.Path]::GetFullPath($StageRoot)
if (-not (Test-Path -LiteralPath (Join-Path $stage 'bin/fcitx5-engine.exe') -PathType Leaf)) {
  throw "Fork plugin inputs must be checked against a verified package stage: $stage"
}
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
  $OutputRoot = Join-Path $repoRoot 'out/package/fork-plugins'
}
if ([string]::IsNullOrWhiteSpace($CacheRoot)) {
  $CacheRoot = Join-Path $repoRoot 'out/package/fork-plugin-cache'
}
if ([string]::IsNullOrWhiteSpace($SystemRoot)) {
  $SystemRoot = Join-Path ([Environment]::GetFolderPath('Windows')) 'System32'
}
$OutputRoot = [IO.Path]::GetFullPath($OutputRoot)
$CacheRoot = [IO.Path]::GetFullPath($CacheRoot)

$inventory = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'fork-plugin-inventory.json') -Raw |
  ConvertFrom-Json
if ($inventory.format_version -ne 1 -or $inventory.release -notmatch '^windows-[0-9]{8}-[0-9]+$' -or
    $inventory.commit -notmatch '^[0-9a-f]{40}$' -or
    $inventory.repository -notmatch '^https://github\.com/[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+\.git$' -or
    @($inventory.packages).Count -lt 1) {
  throw 'Fork plugin inventory is invalid.'
}
$archiveHashes = @{}
foreach ($property in $inventory.archives.PSObject.Properties) {
  if ($property.Name -notmatch '^[a-z0-9][a-z0-9._-]*\.tar\.bz2$' -or $property.Value -notmatch '^[0-9a-f]{64}$') {
    throw "Fork plugin inventory has an invalid archive pin: $($property.Name)"
  }
  $archiveHashes[$property.Name] = $property.Value
}
$ids = @{}
foreach ($plugin in @($inventory.packages)) {
  if ($plugin.id -notmatch '^fcitx5-[a-z0-9][a-z0-9-]*$' -or $ids.ContainsKey($plugin.id) -or
      $plugin.type -notin @('addon', 'inputmethod-data') -or
      ($plugin.type -eq 'addon') -ne ($plugin.architecture -eq 'x64') -or
      ($plugin.type -eq 'inputmethod-data' -and $plugin.architecture -ne 'any') -or
      [string]::IsNullOrWhiteSpace($plugin.title) -or [string]::IsNullOrWhiteSpace($plugin.license) -or
      @($plugin.archives).Count -lt 1) {
    throw "Fork plugin inventory has an invalid package: $($plugin.id)"
  }
  foreach ($archive in @($plugin.archives)) {
    if (-not $archiveHashes.ContainsKey($archive)) { throw "Unpinned archive $archive for $($plugin.id)." }
  }
  $ids[$plugin.id] = $true
}

function Read-UInt16([byte[]] $Bytes, [int] $Offset) { [BitConverter]::ToUInt16($Bytes, $Offset) }
function Read-UInt32([byte[]] $Bytes, [int] $Offset) { [BitConverter]::ToUInt32($Bytes, $Offset) }

function Read-CString([byte[]] $Bytes, [int] $Offset) {
  $end = [Array]::IndexOf($Bytes, [byte] 0, $Offset)
  if ($end -lt 0) { throw 'Unterminated PE string.' }
  return [Text.Encoding]::ASCII.GetString($Bytes, $Offset, $end - $Offset)
}

function Read-PeImage([string] $Path) {
  $bytes = [IO.File]::ReadAllBytes($Path)
  if ($bytes.Length -lt 64 -or (Read-UInt16 $bytes 0) -ne 0x5A4D) { throw "Not a PE file: $Path" }
  $pe = [int](Read-UInt32 $bytes 0x3C)
  if ($pe -lt 0 -or $pe + 24 -gt $bytes.Length -or (Read-UInt32 $bytes $pe) -ne 0x4550) {
    throw "Invalid PE header: $Path"
  }
  $machine = Read-UInt16 $bytes ($pe + 4)
  $sectionCount = Read-UInt16 $bytes ($pe + 6)
  $optionalSize = Read-UInt16 $bytes ($pe + 20)
  $optional = $pe + 24
  $magic = Read-UInt16 $bytes $optional
  $is64 = switch ($magic) { 0x20B { $true } 0x10B { $false } default { throw "Unknown PE magic: $Path" } }
  $directories = $optional + $(if ($is64) { 112 } else { 96 })
  $sections = @()
  $sectionTable = $optional + $optionalSize
  for ($index = 0; $index -lt $sectionCount; $index++) {
    $entry = $sectionTable + 40 * $index
    $sections += [pscustomobject]@{
      VirtualAddress = Read-UInt32 $bytes ($entry + 12)
      VirtualSize = [Math]::Max((Read-UInt32 $bytes ($entry + 8)), (Read-UInt32 $bytes ($entry + 16)))
      RawPointer = Read-UInt32 $bytes ($entry + 20)
    }
  }
  $toOffset = {
    param([uint32] $Rva)
    foreach ($section in $sections) {
      if ($Rva -ge $section.VirtualAddress -and $Rva -lt $section.VirtualAddress + $section.VirtualSize) {
        return [int]($Rva - $section.VirtualAddress + $section.RawPointer)
      }
    }
    throw "PE RVA 0x$($Rva.ToString('x')) is outside every section: $Path"
  }
  $directory = {
    param([int] $Index)
    $entry = $directories + 8 * $Index
    [pscustomobject]@{ Rva = Read-UInt32 $bytes $entry; Size = Read-UInt32 $bytes ($entry + 4) }
  }

  $exports = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  $exportDirectory = & $directory 0
  if ($exportDirectory.Rva -ne 0) {
    $export = & $toOffset $exportDirectory.Rva
    $nameCount = Read-UInt32 $bytes ($export + 24)
    $names = & $toOffset (Read-UInt32 $bytes ($export + 32))
    for ($index = 0; $index -lt $nameCount; $index++) {
      [void]$exports.Add((Read-CString $bytes (& $toOffset (Read-UInt32 $bytes ($names + 4 * $index)))))
    }
  }

  $imports = @{}
  $thunkSize = if ($is64) { 8 } else { 4 }
  # Directory 1 holds regular imports and directory 13 delay-load imports; both
  # must resolve when the addon runs.
  foreach ($spec in @(@{ Index = 1; Size = 20; Name = 12; Lookup = 0; Address = 16 },
                      @{ Index = 13; Size = 32; Name = 4; Lookup = 16; Address = 12 })) {
    $importDirectory = & $directory $spec.Index
    if ($importDirectory.Rva -eq 0) { continue }
    for ($descriptor = & $toOffset $importDirectory.Rva; ; $descriptor += $spec.Size) {
      $nameRva = Read-UInt32 $bytes ($descriptor + $spec.Name)
      if ($nameRva -eq 0) { break }
      $dll = (Read-CString $bytes (& $toOffset $nameRva)).ToLowerInvariant()
      if (-not $imports.ContainsKey($dll)) { $imports[$dll] = [Collections.Generic.List[string]]::new() }
      $lookupRva = Read-UInt32 $bytes ($descriptor + $spec.Lookup)
      if ($lookupRva -eq 0) { $lookupRva = Read-UInt32 $bytes ($descriptor + $spec.Address) }
      for ($thunk = & $toOffset $lookupRva; ; $thunk += $thunkSize) {
        $value = if ($is64) { [BitConverter]::ToUInt64($bytes, $thunk) } else { [uint64](Read-UInt32 $bytes $thunk) }
        if ($value -eq 0) { break }
        $ordinalFlag = if ($is64) { [uint64]1 -shl 63 } else { [uint64]0x80000000 }
        if (($value -band $ordinalFlag) -ne 0) { continue }
        $imports[$dll].Add((Read-CString $bytes ((& $toOffset ([uint32]($value -band 0x7FFFFFFF))) + 2)))
      }
    }
  }
  return [pscustomobject]@{ Machine = $machine; Exports = $exports; Imports = $imports }
}

function Get-DllIndex([string[]] $Directories) {
  $index = @{}
  foreach ($directory in $Directories) {
    if (-not (Test-Path -LiteralPath $directory -PathType Container)) { continue }
    foreach ($file in Get-ChildItem -LiteralPath $directory -File -Filter '*.dll') {
      $name = $file.Name.ToLowerInvariant()
      if (-not $index.ContainsKey($name)) { $index[$name] = $file.FullName }
    }
  }
  return $index
}

$exportCache = @{}
function Get-Exports([string] $Path) {
  if (-not $exportCache.ContainsKey($Path)) { $exportCache[$Path] = (Read-PeImage $Path).Exports }
  return $exportCache[$Path]
}

function Assert-PackageImports([string] $PackageRoot, [string] $Id, $StageDlls) {
  $packageDlls = Get-DllIndex @((Join-Path $PackageRoot 'bin'))
  foreach ($file in Get-ChildItem -LiteralPath $PackageRoot -File -Recurse |
      Where-Object { $_.Extension -in @('.dll', '.exe') }) {
    $image = Read-PeImage $file.FullName
    if ($image.Machine -ne 0x8664) { throw "$Id ships a non-x64 PE file: $($file.FullName)" }
    foreach ($dll in $image.Imports.Keys) {
      if ($dll.StartsWith('api-ms-win-') -or $dll.StartsWith('ext-ms-')) { continue }
      $target = if ($packageDlls.ContainsKey($dll)) { $packageDlls[$dll] }
        elseif ($StageDlls.ContainsKey($dll)) { $StageDlls[$dll] }
        else { $null }
      if (-not $target) {
        if (Test-Path -LiteralPath (Join-Path $SystemRoot $dll) -PathType Leaf) { continue }
        throw "$Id/$($file.Name) imports $dll, which neither the package nor the stage ships."
      }
      $exports = Get-Exports $target
      foreach ($symbol in $image.Imports[$dll]) {
        if (-not $exports.Contains($symbol)) {
          throw "$Id/$($file.Name) imports $symbol from $dll, which $target does not export."
        }
      }
    }
  }
}

$onWindows = $IsWindows -or $env:OS -eq 'Windows_NT'
$curl = if ($onWindows) { Join-Path $SystemRoot 'curl.exe' } else { 'curl' }
$tar = if ($onWindows) { Join-Path $SystemRoot 'tar.exe' } else { 'tar' }
$assetBase = $inventory.repository -replace '\.git$', ''
$assetBase = "$assetBase/releases/download/$($inventory.release)"
New-Item -ItemType Directory -Force -Path $CacheRoot | Out-Null
foreach ($archive in $archiveHashes.Keys) {
  $path = Join-Path $CacheRoot $archive
  if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or
      (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $archiveHashes[$archive]) {
    Write-Host "Downloading $archive"
    & $curl --fail --location --silent --show-error --retry 3 --connect-timeout 30 --max-time 900 `
      --output $path "$assetBase/$archive"
    if ($LASTEXITCODE -ne 0) { throw "Could not download $archive." }
  }
  if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $archiveHashes[$archive]) {
    throw "Fork plugin archive does not match its pinned SHA-256: $archive"
  }
}

$stageDlls = Get-DllIndex @((Join-Path $stage 'bin'))
if (Test-Path -LiteralPath $OutputRoot) { Remove-Item -LiteralPath $OutputRoot -Recurse -Force }
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
foreach ($plugin in @($inventory.packages)) {
  Write-Host "Preparing $($plugin.id)"
  $packageRoot = Join-Path $OutputRoot $plugin.id
  New-Item -ItemType Directory -Force -Path $packageRoot | Out-Null
  foreach ($archive in @($plugin.archives)) {
    $listing = @(& $tar -tvjf (Join-Path $CacheRoot $archive))
    if ($LASTEXITCODE -ne 0) { throw "Could not list $archive." }
    foreach ($entry in $listing) {
      if ($entry -notmatch '^[-d]' -or $entry -match ' (link to|->) ') {
        throw "$archive contains a link or special file: $entry"
      }
    }
    & $tar -xjf (Join-Path $CacheRoot $archive) -C $packageRoot
    if ($LASTEXITCODE -ne 0) { throw "Could not extract $archive." }
  }
  $descriptor = Join-Path $packageRoot 'plugin'
  if (Test-Path -LiteralPath $descriptor) { Remove-Item -LiteralPath $descriptor -Recurse -Force }
  $bin = Join-Path $packageRoot 'bin'
  if (Test-Path -LiteralPath $bin -PathType Container) {
    foreach ($file in Get-ChildItem -LiteralPath $bin -File -Filter '*.dll') {
      if ($stageDlls.ContainsKey($file.Name.ToLowerInvariant())) { Remove-Item -LiteralPath $file.FullName -Force }
    }
  }
  $files = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -Force |
    Where-Object { -not $_.PSIsContainer -or ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) })
  if ($files.Count -eq 0) { throw "Fork plugin $($plugin.id) has an empty payload." }
  foreach ($file in $files) {
    $relative = [IO.Path]::GetRelativePath($packageRoot, $file.FullName).Replace('\', '/')
    if ($file.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw "$($plugin.id) payload has a link: $relative" }
    if ($relative -notmatch '^[A-Za-z0-9][A-Za-z0-9._/-]*$' -or $relative.Contains('..')) {
      throw "$($plugin.id) payload has an unsafe path: $relative"
    }
  }
  if ($plugin.type -eq 'addon') {
    if (@(Get-ChildItem -LiteralPath (Join-Path $packageRoot 'lib/fcitx5') -File -Filter '*.dll' -ErrorAction SilentlyContinue).Count -eq 0) {
      throw "Fork addon $($plugin.id) has no addon library."
    }
    Assert-PackageImports $packageRoot $plugin.id $stageDlls
  } elseif (@($files | Where-Object { $_.Extension -in @('.dll', '.exe') }).Count -ne 0) {
    throw "Fork data package $($plugin.id) ships native code."
  }
}
Write-Host "Prepared $(@($inventory.packages).Count) fork plugin payloads from $($inventory.release): $OutputRoot"
