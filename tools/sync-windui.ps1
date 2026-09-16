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
# than a git submodule.
#
# Vendoring convention:
#   * In scope:  src/, examples/, Cargo.toml, README.md, README.en.md,
#                LICENSE-APACHE, LICENSE-MIT
#   * Excluded:  build.rs, assets/, CHANGELOG.md, docs/, scripts/, .github/,
#                .githooks/, AGENTS.md, and other upstream meta files.
#                build.rs/assets only embed an icon into upstream example exes;
#                the path dependency builds the library, which needs neither.
#
# The pin is recorded in third_party/dependencies.json and in the constant
# WIND_UI_RUST_REFERENCE_COMMIT in rust/config-poc/src/main.rs.

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
New-Item -ItemType Directory -Force -Path $tempParent | Out-Null
$stamp = [System.Guid]::NewGuid().ToString('N')
$script:SyncLogPath = Join-Path $tempParent "wind-ui-sync-$stamp.log"
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

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
  # Drain both redirected streams concurrently. Reading one stream to EOF
  # before the other can deadlock when Cargo fills the warning/error pipe.
  $stdoutTask = $process.StandardOutput.ReadToEndAsync()
  $stderrTask = $process.StandardError.ReadToEndAsync()
  $process.WaitForExit()
  $stdout = $stdoutTask.GetAwaiter().GetResult()
  $stderr = $stderrTask.GetAwaiter().GetResult()
  $exitCode = $process.ExitCode
  $log = @(
    "=== $Name ==="
    "EXE: $FilePath"
    "ARGS: $($Arguments -join ' ')"
    "EXIT: $exitCode"
    '--- STDOUT ---'
    $stdout
    '--- STDERR ---'
    $stderr
    ''
  ) -join "`n"
  [System.IO.File]::AppendAllText($script:SyncLogPath, $log, $utf8NoBom)
  if ($stderr) {
    Write-Host $stderr -NoNewline
  }
  if ($exitCode -ne 0) {
    throw "$Name failed with exit code $exitCode. Full output: $script:SyncLogPath"
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

  # The vendored tree carries the two temporary Windows compatibility fixes for
  # upstream issue #17. The failure was discovered on i686, but the source
  # patches are intentionally architecture-neutral. Check and apply them
  # against the clean official checkout before copying anything into the
  # repository. If an upstream commit contains either fix, the check must fail
  # loudly so the queue can be reviewed and removed deliberately instead of
  # being silently skipped.
  $patches = @(
    (Join-Path $patchDir 'win32-window-user-data.patch'),
    (Join-Path $patchDir 'win32-tray-unaligned.patch')
  )
  foreach ($patch in $patches) {
    if (-not (Test-Path -LiteralPath $patch -PathType Leaf)) {
      throw "Required local WindUI patch is missing: $patch"
    }
  }
  try {
    foreach ($patch in $patches) {
      Invoke-Checked -FilePath 'git.exe' -Arguments @(
        '-C', $cloneDir, 'apply', '--check', '--whitespace=nowarn', $patch
      ) -Name "git apply --check $([System.IO.Path]::GetFileName($patch))"
    }
    foreach ($patch in $patches) {
      Invoke-Checked -FilePath 'git.exe' -Arguments @(
        '-C', $cloneDir, 'apply', '--whitespace=nowarn', $patch
      ) -Name "git apply $([System.IO.Path]::GetFileName($patch))"
    }
  } catch {
    throw "local WindUI patch no longer applies; verify upstream contains issue #17 fix before dropping patch. $($_.Exception.Message)"
  }

  $inScope = @('src', 'examples', 'Cargo.toml', 'README.md', 'README.en.md',
    'LICENSE-APACHE', 'LICENSE-MIT')

  # Normalize the disposable official checkout before touching the vendor tree.
  # The previous order copied first and then rewrote vendor files in place; that
  # can fail with ERROR_USER_MAPPED_FILE when an IDE or analyzer has a section
  # mapped from an existing vendor example. The clone is not a live consumer.
  foreach ($name in @('Cargo.toml', 'README.md', 'README.en.md', 'LICENSE-APACHE', 'LICENSE-MIT')) {
    $path = Join-Path $cloneDir $name
    if (Test-Path -LiteralPath $path) { Normalize-Lf $path }
  }
  $allText = @(Get-ChildItem -LiteralPath (Join-Path $cloneDir 'src') -Recurse -File)
  $allText += @(Get-ChildItem -LiteralPath (Join-Path $cloneDir 'examples') -Recurse -File)
  foreach ($file in $allText) { Normalize-Lf $file.FullName }

  # Validate the patched upstream crate before copying it into the vendor tree.
  # WindUI intentionally has no committed Cargo.lock and is excluded from the
  # root workspace, so these isolated checks are unlocked. Cargo may create a
  # temporary lockfile in the disposable clone; neither it nor the target
  # directory is copied into the repository.
  $windUiManifest = Join-Path $cloneDir 'Cargo.toml'
  $windUiTargetDir = Join-Path $tempParent "wind-ui-check-$stamp"
  $windUiManifestText = [System.IO.File]::ReadAllText($windUiManifest)
  $windUiHadLockfile = Test-Path -LiteralPath (Join-Path $cloneDir 'Cargo.lock')
  $windUiWorkspaceMarkerAdded = $false
  try {
    # The checkout lives below this repository's root Cargo workspace. Add a
    # disposable workspace root marker only for these isolated checks; never
    # copy it into the vendor tree.
    if (-not [regex]::IsMatch($windUiManifestText, '(?m)^\[workspace\]\s*$')) {
      $workspaceText = $windUiManifestText.TrimEnd("`r", "`n") +
        "`n`n[workspace]`nresolver = `"2`"`n"
      [System.IO.File]::WriteAllText($windUiManifest, $workspaceText,
        [System.Text.UTF8Encoding]::new($false))
      $windUiWorkspaceMarkerAdded = $true
    }
    Invoke-Checked -FilePath $cargo -Arguments @('+1.98.0', 'check',
      '--manifest-path', $windUiManifest,
      '--target', 'i686-pc-windows-msvc',
      '--target-dir', $windUiTargetDir,
      '--no-default-features') -Name 'cargo check windui i686'
    Write-Output 'patched official wind-ui-rust i686 check passed.'
    Invoke-Checked -FilePath $cargo -Arguments @('+1.98.0', 'check',
      '--manifest-path', $windUiManifest,
      '--target', 'aarch64-pc-windows-msvc',
      '--target-dir', $windUiTargetDir,
      '--no-default-features') -Name 'cargo check windui arm64'
    Write-Output 'patched official wind-ui-rust arm64 check passed.'
  } finally {
    if ($windUiWorkspaceMarkerAdded) {
      [System.IO.File]::WriteAllText($windUiManifest, $windUiManifestText,
        [System.Text.UTF8Encoding]::new($false))
    }
    $temporaryLock = Join-Path $cloneDir 'Cargo.lock'
    if (-not $windUiHadLockfile -and (Test-Path -LiteralPath $temporaryLock)) {
      Remove-Item -LiteralPath $temporaryLock -Force
    }
  }

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

  # Run the affected consumer tests.
  Invoke-Checked -FilePath $cargo -Arguments @('+1.98.0', 'test', '--locked',
    '-p', 'fcitx5-config-poc', '-p', 'fcitx5-config-qa',
    '--target', 'x86_64-pc-windows-msvc') -Name 'cargo test config consumers'
  Write-Output 'config-poc/config-qa tests passed.'
  Write-Output "wind-ui-rust synced to $Commit (upstream version $upstreamVersion)."
  Write-Output "Full sync log: $script:SyncLogPath"

} finally {
  if (-not $cloneRemoved -and (Test-Path -LiteralPath $cloneDir)) {
    Remove-Item -LiteralPath $cloneDir -Recurse -Force -ErrorAction SilentlyContinue
  }
}
