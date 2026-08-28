[CmdletBinding()]
param([switch] $VerifyOnly, [switch] $VerifyPatchesOnly)

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
  @{ Name = 'fcitx5'; Url = 'https://github.com/fcitx/fcitx5.git'; Commit = 'cdd0b9d900770d1ad1229d759213215d5dc23a90' },
  @{ Name = 'libime'; Url = 'https://github.com/fcitx/libime.git'; Commit = '92bf7144d31d42549d35e5db348dc79100cb2074' },
  @{ Name = 'fcitx5-chinese-addons'; Url = 'https://github.com/fcitx/fcitx5-chinese-addons.git'; Commit = 'bc84e3acb022f5b6b5bed254b14ba19d05023645' },
  @{ Name = 'fcitx5-rime'; Url = 'https://github.com/fcitx/fcitx5-rime.git'; Commit = '4e996319edea790495edc2c91893e9af4c4e6d6a' },
  @{ Name = 'fcitx5-lua'; Url = 'https://github.com/fcitx/fcitx5-lua.git'; Commit = '05db9ee519d448a64ccbe216044e8e0342e8c536' },
  @{ Name = 'fcitx5-unikey'; Url = 'https://github.com/fcitx/fcitx5-unikey.git'; Commit = '53f82a1e01dc0484f46dc8ed419d586cebd2f114' },
  @{ Name = 'librime'; Url = 'https://github.com/rime/librime.git'; Commit = '33e78140250125871856cdc5b42ddc6a5fcd3cd4' },
  @{ Name = 'librime-lua'; Url = 'https://github.com/hchunhui/librime-lua.git'; Commit = '68f9c364a2d25a04c7d4794981d7c796b05ab627' },
  @{ Name = 'librime-octagram'; Url = 'https://github.com/lotem/librime-octagram.git'; Commit = 'dfcc15115788c828d9dd7b4bff68067d3ce2ffb8' },
  @{ Name = 'librime-proto'; Url = 'https://github.com/lotem/librime-proto.git'; Commit = '657a923cd4c333e681dc943e6894e6f6d42d25b4' },
  @{ Name = 'librime-predict'; Url = 'https://github.com/rime/librime-predict.git'; Commit = '920bd41ebf6f9bf6855d14fbe80212e54e749791' }
)

$patchPins = @(
  @{ Source = 'fcitx5'; File = 'fcitx5-windows-core-portability.patch' },
  @{ Source = 'fcitx5'; File = 'fcitx5-windows-user-data-root.patch' },
  @{ Source = 'libime'; File = 'libime-windows-model-dirs.patch' },
  @{ Source = 'fcitx5-chinese-addons'; File = 'fcitx5-chinese-addons-msys2-clang-libcxx.patch' },
  @{ Source = 'fcitx5-rime'; File = 'fcitx5-rime-windows-paths.patch' },
  @{ Source = 'fcitx5-lua'; File = 'fcitx5-lua-windows-lua54.patch' },
  @{ Source = 'librime'; File = 'librime-msys2-clang-windows.patch' }
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
  'mingw-w64-clang-x86_64-capnproto=1.4.0-3',
  'mingw-w64-clang-x86_64-gflags=2.3.0-1',
  'mingw-w64-clang-x86_64-glog=0.7.1-10',
  'mingw-w64-clang-x86_64-leveldb=1.23-3',
  'mingw-w64-clang-x86_64-librime=1.17.0-1',
  'mingw-w64-clang-x86_64-librime-data=0.0.0.20251229-1',
  'mingw-w64-clang-x86_64-lua54=5.4.8-1',
  'mingw-w64-clang-x86_64-marisa=0.2.7-1',
  'mingw-w64-clang-x86_64-opencc=1.3.1-1',
  'mingw-w64-clang-x86_64-rime-bopomofo=0.0.0.20260106-1',
  'mingw-w64-clang-x86_64-rime-cangjie=0.0.0.20240325-1',
  'mingw-w64-clang-x86_64-rime-essay=0.0.0.20260106-1',
  'mingw-w64-clang-x86_64-rime-luna-pinyin=0.0.0.20260106-1',
  'mingw-w64-clang-x86_64-rime-prelude=0.0.0.20251229-1',
  'mingw-w64-clang-x86_64-rime-stroke=0.0.0.20250923-1',
  'mingw-w64-clang-x86_64-rime-terra-pinyin=0.0.0.20251206-1',
  'mingw-w64-clang-x86_64-yaml-cpp=0.9.0-1',
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
    Invoke-Checked $bash @('-lc', "set -e; $Command")
  } finally {
    [Environment]::SetEnvironmentVariable('MSYSTEM', $previousMsystem, 'Process')
    [Environment]::SetEnvironmentVariable('CHERE_INVOKING', $previousChere, 'Process')
  }
}

function Get-RepositoryGitPrefix([string] $Repository) {
  $safeDirectory = [IO.Path]::GetFullPath($Repository).Replace('\', '/')
  return @('-c', "safe.directory=$safeDirectory", '-C', $Repository)
}

function Apply-PinnedPatch([string] $Repository, [string] $Patch) {
  $gitPrefix = Get-RepositoryGitPrefix $Repository
  & git @gitPrefix apply --check $Patch 2>$null
  if ($LASTEXITCODE -eq 0) {
    Invoke-Checked git ($gitPrefix + @('apply', $Patch))
    return
  }

  & git @gitPrefix apply --reverse --check $Patch 2>$null
  if ($LASTEXITCODE -eq 0) {
    Write-Host "Pinned patch already applied: $([IO.Path]::GetFileName($Patch))"
    return
  }

  throw "Pinned patch does not apply cleanly: $Patch"
}

function Assert-PatchCompatibility {
  foreach ($pin in $patchPins) {
    $repository = Join-Path $sources $pin.Source
    $patch = Join-Path $repoRoot "third_party/patches/$($pin.File)"
    if (-not (Test-Path -LiteralPath $patch -PathType Leaf)) {
      throw "Missing pinned patch: $patch"
    }
    $gitPrefix = Get-RepositoryGitPrefix $repository
    $forward = (& git @gitPrefix apply --check $patch 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -eq 0) {
      Write-Host "Patch compatibility: APPLY-CLEAN $($pin.File)"
      continue
    }
    $reverse = (& git @gitPrefix apply --reverse --check $patch 2>&1 | Out-String).Trim()
    if ($LASTEXITCODE -eq 0) {
      Write-Host "Patch compatibility: ALREADY-APPLIED $($pin.File)"
      continue
    }
    throw "Patch compatibility failed for $($pin.Source)/$($pin.File): $forward`n$reverse"
  }
}

function Ensure-PluginJunction([string] $Link, [string] $Target) {
  if (Test-Path -LiteralPath $Link) {
    $item = Get-Item -LiteralPath $Link -Force
    if ($item.LinkType -eq 'Junction' -and
        [IO.Path]::GetFullPath($item.Target) -eq [IO.Path]::GetFullPath($Target)) {
      return
    }
    throw "Unexpected file at pinned Rime plugin path: $Link"
  }
  New-Item -ItemType Junction -Path $Link -Target $Target | Out-Null
}

function Assert-SourcePins {
  foreach ($pin in $sourcePins) {
    $path = Join-Path $sources $pin.Name
    if (-not (Test-Path -LiteralPath (Join-Path $path '.git'))) {
      throw "Missing pinned source checkout: $($pin.Name)"
    }
    $gitPrefix = Get-RepositoryGitPrefix $path
    $actual = (& git @gitPrefix rev-parse HEAD).Trim()
    if ($LASTEXITCODE -ne 0 -or $actual -ne $pin.Commit) {
      throw "Unexpected $($pin.Name) commit: $actual"
    }
  }
}

function Assert-Pins {
  $archive = Join-Path $downloads $archiveName
  if (-not (Test-Path -LiteralPath $archive -PathType Leaf) -or
      (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ne $archiveHash) {
    throw 'Pinned MSYS2 archive is missing or has the wrong SHA-256.'
  }
  Assert-SourcePins
  Assert-PatchCompatibility
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

if ($VerifyPatchesOnly) {
  Assert-SourcePins
  Assert-PatchCompatibility
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
  $gitPrefix = Get-RepositoryGitPrefix $path
  Invoke-Checked git ($gitPrefix + @('fetch', '--depth', '1', 'origin', $pin.Commit))
  Invoke-Checked git ($gitPrefix + @('checkout', '--detach', $pin.Commit))
  Invoke-Checked git ($gitPrefix + @('submodule', 'update', '--init', '--recursive'))
}

$libime = Join-Path $sources 'libime'
$chinese = Join-Path $sources 'fcitx5-chinese-addons'
$rime = Join-Path $sources 'fcitx5-rime'
$lua = Join-Path $sources 'fcitx5-lua'
$unikey = Join-Path $sources 'fcitx5-unikey'
$fcitx = Join-Path $sources 'fcitx5'
$librime = Join-Path $sources 'librime'
Apply-PinnedPatch $fcitx `
  (Join-Path $repoRoot 'third_party/patches/fcitx5-windows-core-portability.patch')
Apply-PinnedPatch $fcitx `
  (Join-Path $repoRoot 'third_party/patches/fcitx5-windows-user-data-root.patch')
Apply-PinnedPatch $libime `
  (Join-Path $repoRoot 'third_party/patches/libime-windows-model-dirs.patch')
Apply-PinnedPatch $chinese `
  (Join-Path $repoRoot 'third_party/patches/fcitx5-chinese-addons-msys2-clang-libcxx.patch')
Apply-PinnedPatch $rime `
  (Join-Path $repoRoot 'third_party/patches/fcitx5-rime-windows-paths.patch')
Apply-PinnedPatch $lua `
  (Join-Path $repoRoot 'third_party/patches/fcitx5-lua-windows-lua54.patch')
Apply-PinnedPatch $librime `
  (Join-Path $repoRoot 'third_party/patches/librime-msys2-clang-windows.patch')
foreach ($plugin in @('librime-lua', 'librime-octagram', 'librime-proto', 'librime-predict')) {
  Ensure-PluginJunction (Join-Path $librime "plugins/$plugin") `
    (Join-Path $sources $plugin)
}

$msysRepo = Convert-ToMsysPath $repoRoot
$msysSources = Convert-ToMsysPath $sources
$msysStage = Convert-ToMsysPath $stage
$common = "-G Ninja -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX='$msysStage'"
Invoke-Msys "cmake -S '$msysSources/fcitx5' -B '$msysRepo/out/build/fcitx5-core' $common -DCMAKE_CXX_FLAGS=-fexperimental-library -DENABLE_DBUS=OFF -DENABLE_X11=OFF -DENABLE_WAYLAND=OFF -DENABLE_KEYBOARD=OFF -DENABLE_SERVER=OFF -DENABLE_TEST=OFF -DENABLE_TESTING_ADDONS=OFF -DBUILD_SPELL_DICT=OFF -DENABLE_DOC=OFF -DENABLE_LIBUUID=OFF -DENABLE_ENCHANT=OFF -DENABLE_EMOJI=OFF -DENABLE_XDGAUTOSTART=OFF; cmake --build '$msysRepo/out/build/fcitx5-core' --parallel; cmake --install '$msysRepo/out/build/fcitx5-core'"
Invoke-Msys "cmake -S '$msysSources/libime' -B '$msysRepo/out/build/libime' $common -DCMAKE_PREFIX_PATH='$msysStage' -DENABLE_TEST=OFF -DENABLE_DOC=OFF; cmake --build '$msysRepo/out/build/libime' --parallel; cmake --install '$msysRepo/out/build/libime'"
Invoke-Msys "cmake -S '$msysSources/fcitx5-chinese-addons' -B '$msysRepo/out/build/fcitx5-chinese-addons' $common -DCMAKE_PREFIX_PATH='$msysStage' -DENABLE_TEST=OFF -DENABLE_GUI=OFF -DENABLE_BROWSER=OFF -DENABLE_CLOUDPINYIN=OFF -DENABLE_OPENCC=OFF; cmake --build '$msysRepo/out/build/fcitx5-chinese-addons' --parallel; cmake --install '$msysRepo/out/build/fcitx5-chinese-addons'"
Invoke-Msys "cmake -S '$msysSources/librime' -B '$msysRepo/out/build/librime' $common -DCMAKE_PREFIX_PATH='$msysStage;/clang64' -DCMAKE_DLL_NAME_WITH_SOVERSION=ON -DBUILD_TEST=OFF -DENABLE_LOGGING=OFF -DLUA_VERSION=lua5.4; cmake --build '$msysRepo/out/build/librime' --parallel; cmake --install '$msysRepo/out/build/librime'"
Invoke-Msys "cmake -S '$msysSources/fcitx5-rime' -B '$msysRepo/out/build/fcitx5-rime' $common -DCMAKE_PREFIX_PATH='$msysStage;/clang64' -DRIME_DATA_DIR='$msysStage/share/rime-data'; cmake --build '$msysRepo/out/build/fcitx5-rime' --parallel; cmake --install '$msysRepo/out/build/fcitx5-rime'"
Invoke-Msys "cmake -S '$msysSources/fcitx5-lua' -B '$msysRepo/out/build/fcitx5-lua' $common -DCMAKE_PREFIX_PATH='$msysStage;/clang64' -DUSE_DLOPEN=OFF -DENABLE_TEST=OFF; cmake --build '$msysRepo/out/build/fcitx5-lua' --parallel; cmake --install '$msysRepo/out/build/fcitx5-lua'"
Invoke-Msys "cmake -S '$msysSources/fcitx5-unikey' -B '$msysRepo/out/build/fcitx5-unikey' $common -DCMAKE_PREFIX_PATH='$msysStage;/clang64' -DENABLE_TEST=OFF; cmake --build '$msysRepo/out/build/fcitx5-unikey' --parallel; cmake --install '$msysRepo/out/build/fcitx5-unikey'"
Invoke-Msys "cmake -S '$msysRepo/native-engine' -B '$msysRepo/out/build/native-engine' $common -DCMAKE_PREFIX_PATH='$msysStage'; cmake --build '$msysRepo/out/build/native-engine' --parallel; cmake --install '$msysRepo/out/build/native-engine'"

$runtimeDlls = @('libc++.dll', 'libzstd.dll', 'libdl.dll', 'libintl-8.dll',
  'libwinpthread-1.dll', 'libuv-1.dll', 'libiconv-2.dll',
  'libyaml-cpp.dll', 'libleveldb.dll', 'libmarisa-0.dll',
  'libopencc-1.3.dll', 'lua54.dll', 'libcapnp.dll', 'libkj.dll', 'libunwind.dll')
foreach ($dll in $runtimeDlls) {
  Copy-Item -LiteralPath (Join-Path $toolchain "clang64/bin/$dll") `
    -Destination (Join-Path $stage 'bin') -Force
}
$rimeDataSource = Join-Path $toolchain 'clang64/share/rime-data'
$rimeDataDestination = Join-Path $stage 'share/rime-data'
$openccSource = Join-Path $toolchain 'clang64/share/opencc'
$openccDestination = Join-Path $stage 'share/opencc'
New-Item -ItemType Directory -Force -Path $rimeDataDestination, $openccDestination | Out-Null
Get-ChildItem -LiteralPath $rimeDataSource -Force |
  Copy-Item -Destination $rimeDataDestination -Recurse -Force
Get-ChildItem -LiteralPath $openccSource -Force |
  Copy-Item -Destination $openccDestination -Recurse -Force
foreach ($requiredRimeFile in @('default.yaml', 'luna_pinyin.schema.yaml',
    'luna_pinyin.dict.yaml', 'essay.txt', 'fcitx5.yaml')) {
  if (-not (Test-Path -LiteralPath (Join-Path $rimeDataDestination $requiredRimeFile) `
      -PathType Leaf)) {
    throw "Incomplete staged Rime data: missing $requiredRimeFile"
  }
}
Assert-Pins
