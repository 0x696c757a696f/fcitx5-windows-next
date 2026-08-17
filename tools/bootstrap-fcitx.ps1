[CmdletBinding()]
param([switch] $VerifyOnly)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$outRoot = Join-Path $repoRoot 'out'
$downloads = Join-Path $outRoot 'downloads'
$toolchain = Join-Path $outRoot 'toolchains/msys64'
$sources = Join-Path $outRoot 'sources'
$stage = Join-Path $outRoot 'stage/fcitx5'
$archiveName = 'msys2-base-x86_64-20260611.tar.xz'
$archiveHash = 'A2D047E8EE213C3C6A49A8DE427EB1069DF12207C0422FF1B3CBB5C905C34221'
$signingFingerprint = '0EBF782C5D53F7E5FB02A66746BD761F7A49B0EC'

$sourcePins = @(
  @{ Name = 'fcitx5'; Url = 'https://github.com/gaboolic/fcitx5.git'; Commit = '50a3069a2f1bb8647abef713d98ad10d0713b752' },
  @{ Name = 'libime'; Url = 'https://github.com/fcitx/libime.git'; Commit = '92bf7144d31d42549d35e5db348dc79100cb2074' },
  @{ Name = 'fcitx5-chinese-addons'; Url = 'https://github.com/fcitx/fcitx5-chinese-addons.git'; Commit = 'bc84e3acb022f5b6b5bed254b14ba19d05023645' }
)

$packagePins = @(
  'mingw-w64-clang-x86_64-clang=22.1.8-2',
  'mingw-w64-clang-x86_64-cmake=4.4.2-2',
  'mingw-w64-clang-x86_64-ninja=1.13.2-1',
  'mingw-w64-clang-x86_64-extra-cmake-modules=6.29.0-1',
  'mingw-w64-clang-x86_64-boost=1.91.0-3',
  'mingw-w64-clang-x86_64-boost-libs=1.91.0-3',
  'mingw-w64-clang-x86_64-icu=78.3-4',
  'mingw-w64-clang-x86_64-libuv=1.52.1-1',
  'mingw-w64-clang-x86_64-dlfcn=1.4.2-1',
  'mingw-w64-clang-x86_64-gettext-tools=1.0-1',
  'mingw-w64-clang-x86_64-gettext-runtime=1.0-1',
  'mingw-w64-clang-x86_64-zstd=1.5.7-2',
  'mingw-w64-clang-x86_64-libiconv=1.19-1',
  'mingw-w64-clang-x86_64-libc++=22.1.8-1',
  'mingw-w64-clang-x86_64-libwinpthread=14.0.0.r262.g5ea8e9fac-1',
  'mingw-w64-clang-x86_64-pkgconf=1~3.0.5-1',
  'gettext=0.22.5-1'
)

function Invoke-Checked([string] $Executable, [string[]] $Arguments) {
  & $Executable @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed ($LASTEXITCODE): $Executable $($Arguments -join ' ')"
  }
}

function Convert-ToMsysPath([string] $Path) {
  $full = [IO.Path]::GetFullPath($Path).Replace('\', '/')
  if ($full -notmatch '^(?<drive>[A-Za-z]):(?<tail>/.*)$') {
    throw "Cannot convert path for MSYS2: $full"
  }
  return "/$($Matches.drive.ToLowerInvariant())$($Matches.tail)"
}

function Invoke-Msys([string] $Command) {
  $bash = Join-Path $toolchain 'usr/bin/bash.exe'
  $previousMsystem = [Environment]::GetEnvironmentVariable('MSYSTEM', 'Process')
  $previousChere = [Environment]::GetEnvironmentVariable('CHERE_INVOKING', 'Process')
  try {
    # Enter the real CLANG64 subsystem so CMake never mixes MSYS /usr headers
    # into native Windows binaries.
    [Environment]::SetEnvironmentVariable('MSYSTEM', 'CLANG64', 'Process')
    [Environment]::SetEnvironmentVariable('CHERE_INVOKING', '1', 'Process')
    Invoke-Checked $bash @('-lc', $Command)
  } finally {
    [Environment]::SetEnvironmentVariable('MSYSTEM', $previousMsystem, 'Process')
    [Environment]::SetEnvironmentVariable('CHERE_INVOKING', $previousChere, 'Process')
  }
}

function Apply-PinnedPatch([string] $Repository, [string] $Patch) {
  & git -C $Repository apply --check $Patch 2>$null
  if ($LASTEXITCODE -eq 0) {
    Invoke-Checked git @('-C', $Repository, 'apply', $Patch)
    return
  }

  & git -C $Repository apply --reverse --check $Patch 2>$null
  if ($LASTEXITCODE -eq 0) {
    Write-Host "Pinned patch already applied: $([IO.Path]::GetFileName($Patch))"
    return
  }

  throw "Pinned patch does not apply cleanly: $Patch"
}

function Assert-Pins {
  $archive = Join-Path $downloads $archiveName
  if (-not (Test-Path -LiteralPath $archive -PathType Leaf) -or
      (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $archiveHash) {
    throw 'Pinned MSYS2 archive is missing or has the wrong SHA-256.'
  }
  foreach ($pin in $sourcePins) {
    $path = Join-Path $sources $pin.Name
    if (-not (Test-Path -LiteralPath (Join-Path $path '.git'))) {
      throw "Missing pinned source checkout: $($pin.Name)"
    }
    $actual = (& git -C $path rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $actual -ne $pin.Commit) {
      throw "Unexpected $($pin.Name) commit: $actual"
    }
  }
  $actualPackages = Invoke-Msys 'pacman -Q'
  foreach ($pin in $packagePins) {
    $parts = $pin.Split('=', 2)
    if ($actualPackages -notcontains "$($parts[0]) $($parts[1])") {
      throw "Missing exact package pin: $pin"
    }
  }
  Write-Host 'Fcitx toolchain, package and source pins verified.'
}

if ($VerifyOnly) {
  Assert-Pins
  return
}

New-Item -ItemType Directory -Force -Path $downloads, $sources | Out-Null
$archive = Join-Path $downloads $archiveName
$signature = "$archive.sig"
$baseUrl = "https://repo.msys2.org/distrib/x86_64/$archiveName"
if (-not (Test-Path -LiteralPath $archive)) {
  Invoke-WebRequest -Uri $baseUrl -OutFile $archive
}
if (-not (Test-Path -LiteralPath $signature)) {
  Invoke-WebRequest -Uri "$baseUrl.sig" -OutFile $signature
}
if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $archiveHash) {
  throw 'MSYS2 archive SHA-256 mismatch.'
}

$gpg = (Get-Command gpg.exe -ErrorAction Stop).Source
$gpgHome = Join-Path $outRoot 'gnupg'
New-Item -ItemType Directory -Force -Path $gpgHome | Out-Null
Invoke-Checked $gpg @('--homedir', $gpgHome, '--batch', '--keyserver',
  'hkps://keyserver.ubuntu.com', '--recv-keys', $signingFingerprint)
$status = & $gpg --homedir $gpgHome --batch --status-fd 1 --verify $signature $archive 2>$null
if ($LASTEXITCODE -ne 0 -or $status -notmatch "VALIDSIG $signingFingerprint") {
  throw 'MSYS2 signature or primary signing fingerprint verification failed.'
}

if (-not (Test-Path -LiteralPath (Join-Path $toolchain 'usr/bin/bash.exe'))) {
  $xz = (Get-Command xz.exe -ErrorAction Stop).Source
  $tarFile = Join-Path $downloads 'msys2-base-x86_64-20260611.tar'
  Invoke-Checked $xz @('-d', '-k', '-f', $archive)
  Invoke-Checked tar.exe @('-xf', $tarFile, '-C', (Join-Path $outRoot 'toolchains'))
}

# MSYS2's first-start hook otherwise copies the Windows hosts file into the
# portable tree. Create bounded local files before any MSYS process starts.
$etc = Join-Path $toolchain 'etc'
[IO.File]::WriteAllText((Join-Path $etc 'hosts'), "127.0.0.1 localhost`n::1 localhost`n",
  [Text.UTF8Encoding]::new($false))
foreach ($name in @('protocols', 'services', 'networks')) {
  $path = Join-Path $etc $name
  if (-not (Test-Path -LiteralPath $path)) {
    [IO.File]::WriteAllText($path, "", [Text.UTF8Encoding]::new($false))
  }
}

$packageNames = $packagePins | ForEach-Object { $_.Split('=', 2)[0] }
Invoke-Msys "pacman -Syu --noconfirm"
Invoke-Msys "pacman -S --needed --noconfirm $($packageNames -join ' ')"

foreach ($pin in $sourcePins) {
  $path = Join-Path $sources $pin.Name
  if (-not (Test-Path -LiteralPath (Join-Path $path '.git'))) {
    Invoke-Checked git @('clone', '--filter=blob:none', $pin.Url, $path)
  }
  Invoke-Checked git @('-C', $path, 'fetch', '--depth', '1', 'origin', $pin.Commit)
  Invoke-Checked git @('-C', $path, 'checkout', '--detach', $pin.Commit)
  Invoke-Checked git @('-C', $path, 'submodule', 'update', '--init', '--recursive')
}

$libime = Join-Path $sources 'libime'
$chinese = Join-Path $sources 'fcitx5-chinese-addons'
$fcitx = Join-Path $sources 'fcitx5'
Apply-PinnedPatch $fcitx `
  (Join-Path $repoRoot 'third_party/patches/fcitx5-windows-user-data-root.patch')
Apply-PinnedPatch $libime `
  (Join-Path $repoRoot 'third_party/patches/libime-windows-model-dirs.patch')
Apply-PinnedPatch $chinese `
  (Join-Path $repoRoot 'third_party/patches/fcitx5-chinese-addons-msys2-clang-libcxx.patch')

$msysRepo = Convert-ToMsysPath $repoRoot
$msysSources = Convert-ToMsysPath $sources
$msysStage = Convert-ToMsysPath $stage
$common = "-G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX='$msysStage'"
Invoke-Msys "cmake -S '$msysSources/fcitx5' -B '$msysRepo/out/build/fcitx5-core' $common -DCMAKE_CXX_FLAGS=-fexperimental-library -DENABLE_DBUS=OFF -DENABLE_X11=OFF -DENABLE_WAYLAND=OFF -DENABLE_KEYBOARD=OFF -DENABLE_SERVER=OFF -DENABLE_TEST=OFF -DENABLE_TESTING_ADDONS=OFF -DBUILD_SPELL_DICT=OFF -DENABLE_DOC=OFF -DENABLE_LIBUUID=OFF -DENABLE_ENCHANT=OFF -DENABLE_EMOJI=OFF -DENABLE_XDGAUTOSTART=OFF; cmake --build '$msysRepo/out/build/fcitx5-core' --parallel; cmake --install '$msysRepo/out/build/fcitx5-core'"
Invoke-Msys "cmake -S '$msysSources/libime' -B '$msysRepo/out/build/libime' $common -DCMAKE_PREFIX_PATH='$msysStage' -DENABLE_TEST=OFF -DENABLE_DOC=OFF; cmake --build '$msysRepo/out/build/libime' --parallel; cmake --install '$msysRepo/out/build/libime'"
Invoke-Msys "cmake -S '$msysSources/fcitx5-chinese-addons' -B '$msysRepo/out/build/fcitx5-chinese-addons' $common -DCMAKE_PREFIX_PATH='$msysStage' -DENABLE_TEST=OFF -DENABLE_GUI=OFF -DENABLE_BROWSER=OFF -DENABLE_CLOUDPINYIN=OFF -DENABLE_OPENCC=OFF; cmake --build '$msysRepo/out/build/fcitx5-chinese-addons' --parallel; cmake --install '$msysRepo/out/build/fcitx5-chinese-addons'"
Invoke-Msys "cmake -S '$msysRepo/native-engine' -B '$msysRepo/out/build/native-engine' $common -DCMAKE_PREFIX_PATH='$msysStage'; cmake --build '$msysRepo/out/build/native-engine' --parallel; cmake --install '$msysRepo/out/build/native-engine'"

$runtimeDlls = @('libc++.dll', 'libzstd.dll', 'libdl.dll', 'libintl-8.dll',
  'libwinpthread-1.dll', 'libuv-1.dll', 'libiconv-2.dll')
foreach ($dll in $runtimeDlls) {
  Copy-Item -LiteralPath (Join-Path $toolchain "clang64/bin/$dll") `
    -Destination (Join-Path $stage 'bin') -Force
}
Assert-Pins
