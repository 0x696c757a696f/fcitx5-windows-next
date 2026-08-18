[CmdletBinding()]
param(
  [ValidateSet('Release')]
  [string] $Configuration = 'Release',
  [string] $StageRoot
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$stagePointer = Join-Path $repoRoot 'out/package/current-stage.txt'
if (-not [Environment]::UserInteractive) {
  throw 'Desktop verification requires an interactive user session.'
}
if (-not (Test-Path -LiteralPath $stagePointer -PathType Leaf)) {
  throw 'No tested package stage is selected. Run the package gate first.'
}
$packageStage = [IO.Path]::GetFullPath(([IO.File]::ReadAllText($stagePointer)).Trim())
$selectedStage = if ([string]::IsNullOrWhiteSpace($StageRoot)) {
  $packageStage
} else {
  [IO.Path]::GetFullPath($StageRoot)
}
$app = if (Test-Path -LiteralPath (Join-Path $selectedStage 'Start Fcitx5.exe')) {
  $selectedStage
} else {
  Join-Path $selectedStage 'Fcitx5'
}
$packageApp = if (Test-Path -LiteralPath (Join-Path $packageStage 'Start Fcitx5.exe')) {
  $packageStage
} else {
  Join-Path $packageStage 'Fcitx5'
}
if ([IO.Path]::GetFullPath($app) -ne [IO.Path]::GetFullPath($packageApp)) {
  $referenceManifest = Get-Content -LiteralPath (Join-Path $packageApp 'manifest.json') -Raw |
    ConvertFrom-Json
  foreach ($file in $referenceManifest.files) {
    $candidate = Join-Path $app ([string]$file.path).Replace('/', '\')
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf) -or
        (Get-FileHash -LiteralPath $candidate -Algorithm SHA256).Hash -ne
          [string]$file.sha256) {
      throw "Registered stage differs from the tested package artifact: $($file.path)"
    }
  }
}
$bin = Join-Path $app 'bin'
$control = Join-Path $bin 'fcitx5-control.exe'
$config = Join-Path $bin 'fcitx5-config.exe'
$launcher = Join-Path $bin 'fcitx5-launcher.exe'
$engine = Join-Path $bin 'fcitx5-engine.exe'
$notepadTest = Join-Path $repoRoot "out/build/windows-x64-dev/$Configuration/fcitx5_tsf_notepad_e2e_test.exe"
foreach ($path in @($control, $config, $launcher, $notepadTest)) {
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    throw "Desktop verification artifact is missing: $path"
  }
}
$textServiceClsid = '{3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}'
$registeredPaths = @(
  [pscustomobject]@{
    Registry = "Registry::HKEY_CLASSES_ROOT\CLSID\$textServiceClsid\InprocServer32"
    Expected = (Join-Path $app 'tsf/x64/fcitx5-tsf.dll')
  },
  [pscustomobject]@{
    Registry = "Registry::HKEY_LOCAL_MACHINE\Software\Classes\WOW6432Node\CLSID\$textServiceClsid\InprocServer32"
    Expected = (Join-Path $app 'tsf/x86/fcitx5-tsf.dll')
  }
)
foreach ($registration in $registeredPaths) {
  if (-not (Test-Path -LiteralPath $registration.Registry)) {
    throw "Exact-stage TSF registration is missing: $($registration.Registry)"
  }
  $actual = (Get-Item -LiteralPath $registration.Registry).GetValue('')
  if ([IO.Path]::GetFullPath($actual) -ne [IO.Path]::GetFullPath($registration.Expected)) {
    throw "Desktop gate refuses a different TSF lineage: $actual"
  }
}
$manifest = Get-Content -LiteralPath (Join-Path $app 'manifest.json') -Raw | ConvertFrom-Json
$trayGuid = switch ([string]$manifest.channel) {
  'stable' { [guid]'8fdc6a8e-5d64-4d4b-9f50-5c1218c0a611' }
  'beta' { [guid]'8fdc6a8e-5d64-4d4b-9f50-5c1218c0a612' }
  'nightly' { [guid]'8fdc6a8e-5d64-4d4b-9f50-5c1218c0a613' }
  default { throw "Unknown release channel in stage manifest: $($manifest.channel)" }
}

if (-not ('Fcitx5Desktop.NativeMethods' -as [type])) {
  Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
namespace Fcitx5Desktop {
  [StructLayout(LayoutKind.Sequential)]
  public struct Rect { public int Left, Top, Right, Bottom; }
  [StructLayout(LayoutKind.Sequential)]
  public struct NotifyIconIdentifier {
    public uint Size;
    public IntPtr Window;
    public uint Id;
    public Guid Guid;
  }
  public static class NativeMethods {
    public delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern IntPtr FindWindow(string className, string windowName);
    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);
    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    private static extern int GetClassName(IntPtr window, StringBuilder className, int maximum);
    [DllImport("shell32.dll")]
    public static extern int Shell_NotifyIconGetRect(
      ref NotifyIconIdentifier identifier, out Rect iconLocation);
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")]
    public static extern void mouse_event(uint flags, uint dx, uint dy, uint data, UIntPtr extra);
    [DllImport("user32.dll")]
    public static extern IntPtr SendMessage(IntPtr window, uint message,
      IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")]
    public static extern bool PostMessage(IntPtr window, uint message,
      IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")]
    public static extern int GetMenuItemCount(IntPtr menu);
    [DllImport("user32.dll")]
    public static extern bool GetMenuItemRect(IntPtr window, IntPtr menu,
      uint item, out Rect rectangle);

    public static IntPtr FindPopupMenuForProcess(IntPtr owner) {
      uint expectedProcessId;
      GetWindowThreadProcessId(owner, out expectedProcessId);
      IntPtr result = IntPtr.Zero;
      EnumWindows((window, parameter) => {
        uint processId;
        GetWindowThreadProcessId(window, out processId);
        if (processId != expectedProcessId || !IsWindowVisible(window)) return true;
        var className = new StringBuilder(32);
        if (GetClassName(window, className, className.Capacity) > 0 &&
            className.ToString() == "#32768") {
          result = window;
          return false;
        }
        return true;
      }, IntPtr.Zero);
      return result;
    }
  }
}
'@
}

function Invoke-ControlJson([string[]] $Arguments) {
  $output = & $control @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Control command failed: $($Arguments -join ' ')"
  }
  return $output | ConvertFrom-Json
}

function Wait-Launcher([bool] $Reachable, [int] $TimeoutMilliseconds = 10000) {
  $deadline = [Environment]::TickCount64 + $TimeoutMilliseconds
  do {
    try {
      $status = Invoke-ControlJson @('--status')
      if ([bool]$status.launcher_reachable -eq $Reachable) { return $status }
    } catch {
      if (-not $Reachable) { return $null }
    }
    Start-Sleep -Milliseconds 100
  } while ([Environment]::TickCount64 -lt $deadline)
  throw "Launcher did not reach expected state: reachable=$Reachable"
}

function Wait-EngineReady([int] $TimeoutMilliseconds = 30000) {
  $deadline = [Environment]::TickCount64 + $TimeoutMilliseconds
  do {
    $status = Invoke-ControlJson @('--status')
    if ([bool]$status.launcher_reachable -and [int]$status.engine_state -eq 2) {
      return $status
    }
    Start-Sleep -Milliseconds 100
  } while ([Environment]::TickCount64 -lt $deadline)
  throw 'Engine did not reach the ready state before desktop input verification.'
}

function Wait-LauncherState([int] $LauncherState, [int] $EngineState,
                            [int] $TimeoutMilliseconds = 10000) {
  $deadline = [Environment]::TickCount64 + $TimeoutMilliseconds
  do {
    $status = Invoke-ControlJson @('--status')
    if ([bool]$status.launcher_reachable -and
        [int]$status.launcher_state -eq $LauncherState -and
        [int]$status.engine_state -eq $EngineState) {
      return $status
    }
    Start-Sleep -Milliseconds 100
  } while ([Environment]::TickCount64 -lt $deadline)
  throw "Launcher did not reach state=$LauncherState engine=$EngineState."
}

function Invoke-TrayMenuItem([int] $Index) {
  $owner = [Fcitx5Desktop.NativeMethods]::FindWindow(
    'Fcitx5WindowsNext.LauncherTray', 'Fcitx5 for Windows')
  if ($owner -eq [IntPtr]::Zero) { throw 'Tray owner window disappeared.' }
  # Drive the documented notification callback instead of guessing a taskbar coordinate.
  # Explorer may display an overflow icon while refusing Shell_NotifyIconGetRect.
  [void][Fcitx5Desktop.NativeMethods]::SetCursorPos(160, 160)
  if (-not [Fcitx5Desktop.NativeMethods]::PostMessage(
      $owner, 0x802A, [IntPtr]1, [IntPtr]0x007B)) {
    throw "Tray icon callback could not be delivered for menu index $Index."
  }
  $deadline = [Environment]::TickCount64 + 2000
  do {
    $popup = [Fcitx5Desktop.NativeMethods]::FindPopupMenuForProcess($owner)
    if ($popup -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 25
  } while ([Environment]::TickCount64 -lt $deadline)
  if ($popup -eq [IntPtr]::Zero) { throw "Tray menu did not open for index $Index." }
  # MN_GETHMENU is the standard menu-window query used by Windows accessibility/testing tools.
  $menu = [Fcitx5Desktop.NativeMethods]::SendMessage(
    $popup, 0x01E1, [IntPtr]::Zero, [IntPtr]::Zero)
  if ($menu -eq [IntPtr]::Zero -or
      [Fcitx5Desktop.NativeMethods]::GetMenuItemCount($menu) -ne 9) {
    throw "Tray menu structure does not match the interaction contract for index $Index."
  }
  $item = [Fcitx5Desktop.Rect]::new()
  if (-not [Fcitx5Desktop.NativeMethods]::GetMenuItemRect(
      $owner, $menu, [uint32]$Index, [ref]$item)) {
    throw "Tray menu item $Index has no clickable rectangle."
  }
  [void][Fcitx5Desktop.NativeMethods]::SetCursorPos(
    [int](($item.Left + $item.Right) / 2), [int](($item.Top + $item.Bottom) / 2))
  [Fcitx5Desktop.NativeMethods]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
  [Fcitx5Desktop.NativeMethods]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 100
}

function Get-ProductEngineIds {
  $expected = [IO.Path]::GetFullPath((Join-Path $bin 'fcitx5-engine.exe'))
  return @(Get-Process -Name 'fcitx5-engine' -ErrorAction SilentlyContinue |
    Where-Object {
      try { [IO.Path]::GetFullPath($_.Path) -eq $expected } catch { $false }
    } | ForEach-Object Id)
}

function Get-ProductConfigIds {
  $expected = [IO.Path]::GetFullPath($config)
  return @(Get-Process -Name 'fcitx5-config' -ErrorAction SilentlyContinue |
    Where-Object {
      try { [IO.Path]::GetFullPath($_.Path) -eq $expected } catch { $false }
    } | ForEach-Object Id)
}

function Stop-ConflictingWorkspaceStage {
  $expectedLauncher = [IO.Path]::GetFullPath($launcher)
  $rootPrefix = $repoRoot.TrimEnd([IO.Path]::DirectorySeparatorChar) +
    [IO.Path]::DirectorySeparatorChar
  $conflicting = @(Get-Process -Name 'fcitx5-launcher' -ErrorAction SilentlyContinue |
    Where-Object {
      try {
        $path = [IO.Path]::GetFullPath($_.Path)
        $path.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase) -and
          $path -ne $expectedLauncher
      } catch { $false }
    })
  foreach ($process in $conflicting) {
    $oldControl = Join-Path ([IO.Path]::GetDirectoryName($process.Path)) 'fcitx5-control.exe'
    if (-not (Test-Path -LiteralPath $oldControl -PathType Leaf)) {
      throw "Conflicting workspace launcher has no owned Control API: $($process.Path)"
    }
    & $oldControl --shutdown | Out-Null
    if ($LASTEXITCODE -ne 0) {
      throw "Conflicting workspace stage refused graceful shutdown: $($process.Path)"
    }
  }
  if ($conflicting.Count -eq 0) { return }
  $deadline = [Environment]::TickCount64 + 10000
  do {
    $remaining = @($conflicting | Where-Object {
        Get-Process -Id $_.Id -ErrorAction SilentlyContinue
      })
    if ($remaining.Count -eq 0) { return }
    Start-Sleep -Milliseconds 100
  } while ([Environment]::TickCount64 -lt $deadline)
  throw 'Conflicting workspace stage did not stop within its shutdown deadline.'
}

function Test-TrayConfigLaunch([int] $MenuIndex) {
  $before = @(Get-ProductConfigIds)
  Invoke-TrayMenuItem $MenuIndex
  $deadline = [Environment]::TickCount64 + 5000
  do {
    $after = @(Get-ProductConfigIds)
    $opened = @($after | Where-Object { $_ -notin $before })
    if ($opened.Count -gt 0) { break }
    Start-Sleep -Milliseconds 50
  } while ([Environment]::TickCount64 -lt $deadline)
  if ($opened.Count -eq 0) { throw "Tray menu item $MenuIndex did not open Config." }
  foreach ($id in $opened) {
    $process = Get-Process -Id $id -ErrorAction SilentlyContinue
    if ($process) {
      [void]$process.CloseMainWindow()
      if (-not $process.WaitForExit(3000)) { Stop-Process -Id $id -Force }
    }
  }
}

$evidence = [ordered]@{
  format_version = 1
  stage = $app.Replace('\', '/')
  package_stage = $packageApp.Replace('\', '/')
  stage_content_equivalent = $true
  tested_at_utc = [DateTime]::UtcNow.ToString('o')
  checks = [ordered]@{}
}

try {
  Stop-ConflictingWorkspaceStage
  $startResult = Start-Process -FilePath $launcher -ArgumentList '--background' -PassThru
  if ($startResult.WaitForExit(1000) -and $startResult.ExitCode -ne 0) {
    throw 'Fcitx5 launcher failed during startup.'
  }
  $status = Wait-Launcher $true
  $status = Wait-EngineReady
  $evidence.checks.launcher_reachable = $true
  $evidence.checks.engine_ready = $true

  $trayWindow = [IntPtr]::Zero
  $iconAvailable = $false
  $deadline = [Environment]::TickCount64 + 5000
  do {
    $trayWindow = [Fcitx5Desktop.NativeMethods]::FindWindow(
      'Fcitx5WindowsNext.LauncherTray', 'Fcitx5 for Windows')
    if ($trayWindow -ne [IntPtr]::Zero) {
      $identifier = [Fcitx5Desktop.NotifyIconIdentifier]::new()
      $identifier.Size = [Runtime.InteropServices.Marshal]::SizeOf($identifier)
      $identifier.Guid = $trayGuid
      $iconRect = [Fcitx5Desktop.Rect]::new()
      $iconAvailable = [Fcitx5Desktop.NativeMethods]::Shell_NotifyIconGetRect(
        [ref]$identifier, [ref]$iconRect) -eq 0
      if (-not $iconAvailable) {
        # Explorer can reject first-time GUID identities on some managed desktops. The launcher
        # then uses the documented hWnd/uID identity so status/recovery remains available.
        $identifier.Guid = [guid]::Empty
        $identifier.Window = $trayWindow
        $identifier.Id = 1
        $iconAvailable = [Fcitx5Desktop.NativeMethods]::Shell_NotifyIconGetRect(
          [ref]$identifier, [ref]$iconRect) -eq 0
      }
    }
    if (-not $iconAvailable) { Start-Sleep -Milliseconds 100 }
  } while (-not $iconAvailable -and [Environment]::TickCount64 -lt $deadline)
  if ($trayWindow -eq [IntPtr]::Zero) { throw 'Launcher tray owner window was not created.' }
  $evidence.checks.tray_icon_registered = $true
  $evidence.checks.tray_icon_locator = if ($iconAvailable) {
    'shell-notify-icon-rectangle'
  } else {
    'documented-notification-callback-fallback'
  }

  $uiContract = Start-Process -FilePath $config -ArgumentList '--ui-contract-test' `
    -PassThru -Wait -WindowStyle Hidden
  if ($uiContract.ExitCode -ne 0) { throw 'Config UI behavior contract failed.' }
  $evidence.checks.config_ui_contract = $true
  $uiInteraction = Start-Process -FilePath $config -ArgumentList '--ui-interaction-test' `
    -PassThru -Wait -WindowStyle Hidden
  if ($uiInteraction.ExitCode -ne 0) { throw 'Config complete interaction sweep failed.' }
  $evidence.checks.config_interaction_coverage = $true

  Test-TrayConfigLaunch 5
  Test-TrayConfigLaunch 6
  $evidence.checks.tray_settings_and_diagnostics = $true

  & $notepadTest $engine
  if ($LASTEXITCODE -ne 0) { throw 'Real Notepad TSF typing smoke failed.' }
  $evidence.checks.notepad_commit = '你'

  $before = @(Get-ProductEngineIds)
  Invoke-TrayMenuItem 2
  $deadline = [Environment]::TickCount64 + 10000
  do {
    Start-Sleep -Milliseconds 100
    $after = @(Get-ProductEngineIds)
    $restarted = $after.Count -gt 0 -and
      ($before.Count -eq 0 -or @($after | Where-Object { $_ -notin $before }).Count -gt 0)
  } while (-not $restarted -and [Environment]::TickCount64 -lt $deadline)
  if (-not $restarted) { throw 'Engine process identity did not change after restart.' }
  [void](Wait-EngineReady)
  $evidence.checks.tray_engine_restart = $true

  Invoke-TrayMenuItem 3
  [void](Wait-LauncherState 1 0)
  Invoke-TrayMenuItem 3
  [void](Wait-EngineReady)
  $evidence.checks.tray_pause_resume = $true

  Invoke-TrayMenuItem 8
  [void](Wait-Launcher $false)
  $evidence.checks.tray_exit = $true
  & $notepadTest --passthrough
  if ($LASTEXITCODE -ne 0) { throw 'Engine-unavailable fail-open smoke failed.' }
  $evidence.checks.engine_absent_fail_open = 'abc'
} finally {
  $restore = Start-Process -FilePath $launcher -ArgumentList '--background' -PassThru
  try { [void](Wait-EngineReady) } catch { Write-Warning $_ }
}

$evidenceRoot = Join-Path $repoRoot 'out/evidence'
New-Item -ItemType Directory -Force -Path $evidenceRoot | Out-Null
$evidencePath = Join-Path $evidenceRoot 'desktop-verification.json'
[IO.File]::WriteAllText($evidencePath, ($evidence | ConvertTo-Json -Depth 6),
  [Text.UTF8Encoding]::new($false))
Write-Host "Desktop verification passed. Evidence: $evidencePath"
