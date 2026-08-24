[CmdletBinding()]
param(
  [string]$BuildRoot = 'out/build/windows-x64-dev/Release',
  [string]$EvidenceRoot = 'out/evidence'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$build = [IO.Path]::GetFullPath((Join-Path $repoRoot $BuildRoot))
$evidence = [IO.Path]::GetFullPath((Join-Path $repoRoot $EvidenceRoot))
$uiSource = Join-Path $build 'fcitx5-ui.exe'
$controlSource = Join-Path $build 'fcitx5-control.exe'
$resourcesSource = Join-Path $build 'resources'
foreach ($required in @($uiSource, $controlSource, $resourcesSource)) {
  if (-not (Test-Path -LiteralPath $required)) {
    throw "Candidate evidence input is missing: $required"
  }
}

$fixtureRoot = Join-Path $repoRoot ('out/candidate-evidence-' + [guid]::NewGuid().ToString('N'))
$appRoot = Join-Path $fixtureRoot 'Fcitx5'
$bin = Join-Path $appRoot 'bin'
$processes = [Collections.Generic.List[Diagnostics.Process]]::new()

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class CandidateEvidenceNative {
    public delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT {
        public int Left;
        public int Top;
        public int Right;
        public int Bottom;
    }

    [DllImport("user32.dll")]
    private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);

    [DllImport("user32.dll")]
    private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);

    [DllImport("user32.dll")]
    private static extern bool IsWindowVisible(IntPtr window);

    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr window, out RECT rectangle);

    [DllImport("user32.dll")]
    public static extern bool SetProcessDpiAwarenessContext(IntPtr context);

    [DllImport("user32.dll")]
    public static extern IntPtr GetWindowDC(IntPtr window);

    [DllImport("user32.dll")]
    public static extern IntPtr GetDC(IntPtr window);

    [DllImport("user32.dll")]
    public static extern int ReleaseDC(IntPtr window, IntPtr deviceContext);

    [DllImport("gdi32.dll")]
    public static extern bool BitBlt(IntPtr destination, int x, int y, int width, int height,
        IntPtr source, int sourceX, int sourceY, uint rasterOperation);

    public static IntPtr FindVisibleWindow(uint expectedProcessId) {
        IntPtr result = IntPtr.Zero;
        EnumWindows((window, parameter) => {
            uint processId;
            GetWindowThreadProcessId(window, out processId);
            if (processId == expectedProcessId && IsWindowVisible(window)) {
                result = window;
                return false;
            }
            return true;
        }, IntPtr.Zero);
        return result;
    }
}
'@

[void][CandidateEvidenceNative]::SetProcessDpiAwarenessContext([IntPtr](-4))

function Invoke-Control {
  param([Parameter(Mandatory)][string[]]$Arguments)
  & (Join-Path $bin 'fcitx5-control.exe') @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Control command failed with exit code ${LASTEXITCODE}: $($Arguments -join ' ')"
  }
}

function Wait-CandidateWindow {
  param([Parameter(Mandatory)][Diagnostics.Process]$Process)
  for ($attempt = 0; $attempt -lt 200; $attempt++) {
    $Process.Refresh()
    if ($Process.HasExited) {
      throw 'Candidate UI exited before showing its preview.'
    }
    $window = [CandidateEvidenceNative]::FindVisibleWindow([uint32]$Process.Id)
    if ($window -ne [IntPtr]::Zero) {
      return $window
    }
    Start-Sleep -Milliseconds 25
  }
  throw 'Candidate preview window was not shown.'
}

function Get-StableRectangle {
  param([Parameter(Mandatory)][IntPtr]$Window)
  $last = $null
  $stable = 0
  for ($attempt = 0; $attempt -lt 200; $attempt++) {
    $rectangle = [CandidateEvidenceNative+RECT]::new()
    if (-not [CandidateEvidenceNative]::GetWindowRect($Window, [ref]$rectangle)) {
      throw 'Candidate preview rectangle query failed.'
    }
    $signature = "$($rectangle.Left),$($rectangle.Top),$($rectangle.Right),$($rectangle.Bottom)"
    if ($signature -eq $last) { $stable++ } else { $last = $signature; $stable = 0 }
    if ($stable -ge 5) { return $rectangle }
    Start-Sleep -Milliseconds 25
  }
  throw 'Candidate preview rectangle did not stabilize.'
}

function Save-WindowPng {
  param(
    [Parameter(Mandatory)][IntPtr]$Window,
    [Parameter(Mandatory)][string]$Path
  )
  $rectangle = Get-StableRectangle -Window $Window
  $width = $rectangle.Right - $rectangle.Left
  $height = $rectangle.Bottom - $rectangle.Top
  if ($width -le 0 -or $height -le 0) { throw 'Candidate preview size is invalid.' }
  $bitmap = [Drawing.Bitmap]::new($width, $height, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
  try {
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
      $deviceContext = $graphics.GetHdc()
      try {
        $sourceContext = [CandidateEvidenceNative]::GetDC([IntPtr]::Zero)
        if ($sourceContext -eq [IntPtr]::Zero) {
          throw 'Could not acquire the candidate preview device context.'
        }
        try {
          if (-not [CandidateEvidenceNative]::BitBlt(
              $deviceContext, 0, 0, $width, $height, $sourceContext,
              $rectangle.Left, $rectangle.Top, 0x40CC0020)) {
            throw 'BitBlt could not capture the candidate preview.'
          }
        } finally {
          [void][CandidateEvidenceNative]::ReleaseDC([IntPtr]::Zero, $sourceContext)
        }
      } finally {
        $graphics.ReleaseHdc($deviceContext)
      }
    } finally {
      $graphics.Dispose()
    }
    $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
  } finally {
    $bitmap.Dispose()
  }
  return [ordered]@{ left = $rectangle.Left; top = $rectangle.Top; width = $width; height = $height }
}

function Start-CandidateDemo {
  param([Parameter(Mandatory)][string]$Argument)
  $process = Start-Process -FilePath (Join-Path $bin 'fcitx5-ui.exe') `
    -ArgumentList $Argument -WorkingDirectory $bin -PassThru
  $processes.Add($process)
  return $process
}

try {
  New-Item -ItemType Directory -Force -Path $bin, $evidence | Out-Null
  Copy-Item -LiteralPath $uiSource, $controlSource -Destination $bin
  Copy-Item -LiteralPath $resourcesSource -Destination $bin -Recurse
  New-Item -ItemType File -Path (Join-Path $appRoot 'portable.flag') | Out-Null

  Invoke-Control @('--set-presentation', 'light', 'builtin:default', 'vertical', 'disabled',
    '5', 'Segoe UI')
  $demo = Start-CandidateDemo '--demo'
  $window = Wait-CandidateWindow -Process $demo
  # The HWND becomes visible before the synthetic CandidateModel finishes its first reflow.
  Start-Sleep -Milliseconds 300
  $vertical = Save-WindowPng -Window $window -Path (Join-Path $evidence 'candidate-live-vertical.png')

  Invoke-Control @('--set-presentation', 'dark', 'builtin:default', 'horizontal', 'enabled',
    '6', 'Microsoft YaHei')
  Start-Sleep -Milliseconds 200
  $horizontal = Save-WindowPng -Window $window `
    -Path (Join-Path $evidence 'candidate-live-horizontal-dark.png')

  Invoke-Control @('--set-presentation', 'light', 'builtin:default', 'vertical', 'disabled',
    '5', 'Segoe UI')
  Start-Sleep -Milliseconds 200
  $restored = Save-WindowPng -Window $window -Path (Join-Path $evidence 'candidate-live-restored.png')
  $demo.Kill($true)
  $demo.WaitForExit(5000) | Out-Null

  Invoke-Control @('--set-presentation', 'light', 'builtin:default', 'vertical', 'enabled',
    '5', 'Microsoft YaHei')
  $scroll = Start-CandidateDemo '--scroll-demo'
  $scrollWindow = Wait-CandidateWindow -Process $scroll
  Start-Sleep -Milliseconds 300
  $scrollRectangle = Save-WindowPng -Window $scrollWindow `
    -Path (Join-Path $evidence 'candidate-scroll-mode-current.png')
  $scroll.Kill($true)
  $scroll.WaitForExit(5000) | Out-Null

  Invoke-Control @('--set-presentation', 'light', 'builtin:default', 'horizontal', 'enabled',
    '6', 'Microsoft YaHei')
  $scrollHorizontal = Start-CandidateDemo '--scroll-demo'
  $scrollHorizontalWindow = Wait-CandidateWindow -Process $scrollHorizontal
  Start-Sleep -Milliseconds 300
  $scrollHorizontalRectangle = Save-WindowPng -Window $scrollHorizontalWindow `
    -Path (Join-Path $evidence 'candidate-scroll-mode-horizontal-current-row.png')
  $scrollHorizontal.Kill($true)
  $scrollHorizontal.WaitForExit(5000) | Out-Null

  $record = [ordered]@{
    format_version = 1
    generated_at = [DateTimeOffset]::UtcNow.ToString('O')
    build_root = $build
    vertical = $vertical
    horizontal_dark = $horizontal
    restored_vertical = $restored
    scroll_mode = $scrollRectangle
    scroll_mode_horizontal = $scrollHorizontalRectangle
  }
  $record | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath `
    (Join-Path $evidence 'candidate-presentation-evidence.json') -Encoding utf8NoBOM
  Write-Host "Candidate evidence captured in $evidence"
} finally {
  foreach ($process in $processes) {
    try {
      $process.Refresh()
      if (-not $process.HasExited) { $process.Kill($true) }
      $process.Dispose()
    } catch {}
  }
  $resolvedFixture = [IO.Path]::GetFullPath($fixtureRoot)
  $expectedPrefix = [IO.Path]::GetFullPath((Join-Path $repoRoot 'out/candidate-evidence-'))
  if ($resolvedFixture.StartsWith($expectedPrefix, [StringComparison]::OrdinalIgnoreCase) -and
      (Test-Path -LiteralPath $resolvedFixture)) {
    Remove-Item -LiteralPath $resolvedFixture -Recurse -Force
  }
}
