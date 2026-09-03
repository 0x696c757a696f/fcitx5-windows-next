[CmdletBinding()]
param(
  [Parameter(Mandatory = $false)]
  [string] $Commit,
  [switch] $Latest,
  [string] $UpstreamUrl = 'https://github.com/huanfeng/wind-ui-rust.git'
)

# Sync the vendored `huanfeng/wind-ui-rust` path dependency to a new upstream
# commit. The vendored tree under `third_party/wind-ui-rust` is consumed as a
# Rust path dependency (`windui`), so it must stay a flat in-tree copy rather
# than a git submodule (submodule working trees cannot carry the repo-local
# Windows patches this project applies).
#
# Vendoring convention:
#   * In scope:  src/, examples/, Cargo.toml, README.md, README.en.md,
#                LICENSE-APACHE, LICENSE-MIT
#   * Excluded:  build.rs, assets/, CHANGELOG.md, docs/, scripts/, .github/,
#                .githooks/, AGENTS.md, and other upstream meta files.
#                build.rs/assets only embed an icon into upstream example exes;
#                the path dependency builds the library, which needs neither.
#
# Repo-local Windows portability patches (must always be re-applied after a
# sync; a sync that cannot apply them fails closed and leaves the tree clean):
#   * third_party/patches/wind-ui-rust/win32-window-user-data.patch
#   * third_party/patches/wind-ui-rust/win32-tray-unaligned.patch
#
# The pin is recorded in third_party/dependencies.json and in the constant
# WIND_UI_RUST_REFERENCE_COMMIT in rust/config-poc/src/main.rs.
#
# The patches are first verified with `git apply --check` against the clean
# upstream clone, BEFORE the vendored tree is touched. If any patch would not
# apply, the script aborts with the vendored tree untouched.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$vendorDir = Join-Path $repoRoot 'third_party/wind-ui-rust'
$patchDir = Join-Path $repoRoot 'third_party/patches/wind-ui-rust'
$depsFile = Join-Path $repoRoot 'third_party/dependencies.json'
$configPocMain = Join-Path $repoRoot 'rust/config-poc/src/main.rs'
$tempParent = Join-Path $repoRoot 'out/tmp'

function Invoke-Checked {
  param([Parameter(Mandatory = $true)] [string] $FilePath,
        [Parameter(Mandatory = $true)] [string[]] $Arguments,
        [string] $Name)
  $startInfo = [Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = $FilePath
  foreach ($argument in $Arguments) { [void] $startInfo.ArgumentList.Add($argument) }
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
  return $stdout
}

function Invoke-GitApply {
  param([Parameter(Mandatory = $true)] [string] $WorkingDirectory,
        [Parameter(Mandatory = $true)] [string[]] $Arguments,
        [string] $Name)
  $startInfo = [Diagnostics.ProcessStartInfo]::new()
  $startInfo.FileName = 'git.exe'
  $startInfo.WorkingDirectory = $WorkingDirectory
  foreach ($argument in $Arguments) { [void] $startInfo.ArgumentList.Add($argument) }
  $startInfo.UseShellExecute = $false
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  $startInfo.CreateNoWindow = $true
  $process = [Diagnostics.Process]::Start($startInfo)
  $stdout = $process.StandardOutput.ReadToEnd()
  $stderr = $process.StandardError.ReadToEnd()
  $process.WaitForExit()
  if ($process.ExitCode -ne 0) {
    throw "$Name failed in $WorkingDirectory with exit code $($process.ExitCode): $stderr $stdout"
  }
  return $stdout
}

function Normalize-Lf([string] $Path) {
  $raw = [System.IO.File]::ReadAllText($Path)
  if ($raw.Contains("`r`n")) {
    $raw = $raw -replace "`r`n", "`n"
    [System.IO.File]::WriteAllText($Path, $raw, [System.Text.UTF8Encoding]::new($false))
  }
}

if (-not (Test-Path -LiteralPath $patchDir -PathType Container)) {
  throw "Missing wind-ui portability patch directory: $patchDir"
}
$patches = @(Get-ChildItem -LiteralPath $patchDir -Filter '*.patch' -File | Sort-Object Name)
if ($patches.Count -eq 0) {
  throw "No portability patches found in $patchDir"
}

if ($Latest) {
  if ($Commit) { throw 'Specify either -Commit or -Latest, not both.' }
  $remote = Invoke-Checked -FilePath 'git.exe' -Arguments @('ls-remote', $UpstreamUrl, 'HEAD') -Name 'git ls-remote'
  $Commit = ($remote -split '\s+')[0]
  if (-not $Commit) { throw 'Could not resolve upstream HEAD.' }
}
if (-not $Commit) {
  throw 'Specify -Commit <sha> (or -Latest).'
}
if ($Commit -notmatch '^[0-9a-f]{40}$') {
  throw "Invalid commit sha: $Commit"
}

# Use the repository toolchain cargo for the post-sync test run.
$env:RUSTUP_HOME = 'D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\rustup-home'
$env:CARGO_HOME = 'D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-msvc'
$cargo = 'D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
  $cargo = (Get-Command cargo.exe -ErrorAction SilentlyContinue).Source
  if (-not $cargo) { throw 'Cargo executable not found.' }
}

New-Item -ItemType Directory -Force -Path $tempParent | Out-Null
$stamp = [System.Guid]::NewGuid().ToString('N')
$cloneDir = Join-Path $tempParent "wind-ui-sync-$stamp"
$cloneRemoved = $false
try {
  Invoke-Checked -FilePath 'git.exe' -Arguments @('clone', '--quiet', $UpstreamUrl, $cloneDir) -Name 'git clone'
  Invoke-Checked -FilePath 'git.exe' -Arguments @('-C', $cloneDir, 'fetch', '--quiet', 'origin', $Commit) -Name 'git fetch'
  Invoke-Checked -FilePath 'git.exe' -Arguments @('-C', $cloneDir, 'checkout', '--quiet', '--detach', $Commit) -Name 'git checkout'
  $actual = Invoke-Checked -FilePath 'git.exe' -Arguments @('-C', $cloneDir, 'rev-parse', 'HEAD') -Name 'git rev-parse'
  if ($actual.Trim() -ne $Commit) {
    throw "Resolved HEAD $($actual.Trim()) does not match requested commit $Commit"
  }

  # Fail-closed patch verification against the CLEAN upstream clone, before the
  # vendored tree is modified. A patch that no longer applies aborts the sync
  # with the repository tree untouched.
  foreach ($patch in $patches) {
    Invoke-GitApply -WorkingDirectory $cloneDir `
      -Arguments @('apply', '--check', $patch.FullName) `
      -Name "patch check $($patch.Name)"
  }

  $inScope = @('src', 'examples', 'Cargo.toml', 'README.md', 'README.en.md',
    'LICENSE-APACHE', 'LICENSE-MIT')
  foreach ($entry in $inScope) {
    $source = Join-Path $cloneDir $entry
    if (-not (Test-Path -LiteralPath $source)) {
      throw "Upstream checkout is missing in-scope entry: $entry"
    }
    $destination = Join-Path $vendorDir $entry
    if (Test-Path -LiteralPath $destination) {
      Remove-Item -LiteralPath $destination -Recurse -Force
    }
    $item = Get-Item -LiteralPath $source
    if ($item.PSIsContainer) {
      New-Item -ItemType Directory -Force -Path $destination | Out-Null
      Get-ChildItem -LiteralPath $source -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $destination -Recurse -Force
      }
    } else {
      Copy-Item -LiteralPath $source -Destination $destination -Force
    }
  }

  # Normalize every copied vendored file to LF (the repo stores LF only via
  # `.gitattributes`: `* text=auto eol=lf`). Fresh upstream clones are checked
  # out with CRLF on Windows, so copy then normalize the whole tree.
  foreach ($name in @('Cargo.toml', 'README.md', 'README.en.md', 'LICENSE-APACHE', 'LICENSE-MIT')) {
    $path = Join-Path $vendorDir $name
    if (Test-Path -LiteralPath $path) { Normalize-Lf $path }
  }
  $allText = @(Get-ChildItem -LiteralPath (Join-Path $vendorDir 'src') -Recurse -File)
  $allText += @(Get-ChildItem -LiteralPath (Join-Path $vendorDir 'examples') -Recurse -File)
  foreach ($file in $allText) { Normalize-Lf $file.FullName }

  # Apply the repo-local Windows portability patches to the vendored tree.
  # Run from the repository root with --directory so the patch paths (which are
  # relative to the vendored tree root) resolve under third_party/wind-ui-rust.
  foreach ($patch in $patches) {
    Invoke-GitApply -WorkingDirectory $repoRoot `
      -Arguments @('apply', '--directory=third_party/wind-ui-rust', $patch.FullName) `
      -Name "apply $($patch.Name)"
  }

  # Update the dependency pin + upstream version in dependencies.json.
  $upstreamVersion = '0.0.0'
  $cargoToml = Join-Path $vendorDir 'Cargo.toml'
  $tomlText = [System.IO.File]::ReadAllText($cargoToml)
  $m = [regex]::Match($tomlText, '(?m)^version\s*=\s*"([^"]+)"')
  if ($m.Success) { $upstreamVersion = $m.Groups[1].Value }
  $short = $Commit.Substring(0, 8)
  $deps = Get-Content -LiteralPath $depsFile -Raw -Encoding UTF8 | ConvertFrom-Json
  foreach ($entry in $deps.packages) {
    if ($entry.name -eq 'wind-ui-rust') {
      $entry.version = "$upstreamVersion+$short"
      $entry.source = "https://github.com/huanfeng/wind-ui-rust/tree/$Commit"
    }
  }
  $depsJson = $deps | ConvertTo-Json -Depth 12
  $depsJson = $depsJson -replace "`r`n", "`n"
  if (-not $depsJson.EndsWith("`n")) { $depsJson += "`n" }
  [System.IO.File]::WriteAllText($depsFile, $depsJson,
    [System.Text.UTF8Encoding]::new($false))

  # Update the reference-commit constant (any prior pin) to the new commit,
  # preserving the existing formatting.
  $configText = [System.IO.File]::ReadAllText($configPocMain)
  $configText = $configText -replace
    'WIND_UI_RUST_REFERENCE_COMMIT: &str = "[0-9a-f]{40}"',
    "WIND_UI_RUST_REFERENCE_COMMIT: &str = `"$Commit`""
  # Multi-line assert_eq!(\n evidence.reference_commit,<newline>  "<hash>"). Replace the quoted
  # hash on the indented line that follows `evidence.reference_commit,`.
  $configText = $configText -replace
    '(evidence\.reference_commit,\r?\n[ \t]*)"[0-9a-f]{40}"',
    ('${1}"' + $Commit + '"')
  # .contains("\"windui_reference_commit\":\"<hash>\"") - the Rust source
  # stores the escaped quote pair (backslash-quote), so the regex must match
  # \"...\" and keep that escaped form around the new commit.
  $configText = $configText -replace
    '("\\"windui_reference_commit\\":\\")[0-9a-f]{40}(\\")',
    ('${1}' + $Commit + '${2}')
  [System.IO.File]::WriteAllText($configPocMain, $configText,
    [System.Text.UTF8Encoding]::new($false))

  Write-Output "wind-ui-rust synced to $Commit (upstream version $upstreamVersion)."

  # Run the affected consumer tests.
  Invoke-Checked -FilePath $cargo -Arguments @('+1.98.0', 'test', '--locked',
    '-p', 'fcitx5-config-poc', '-p', 'fcitx5-config-qa',
    '--target', 'x86_64-pc-windows-msvc') -Name 'cargo test config consumers'
  Write-Output 'config-poc/config-qa tests passed.'
} finally {
  if (-not $cloneRemoved -and (Test-Path -LiteralPath $cloneDir)) {
    Remove-Item -LiteralPath $cloneDir -Recurse -Force -ErrorAction SilentlyContinue
  }
}
