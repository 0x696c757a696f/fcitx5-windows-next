[CmdletBinding()]
param(
  # An extracted portable package: the directory holding portable.flag, bin/ and themes/.
  [Parameter(Mandatory)] [string] $PortableRoot,
  [Parameter(Mandatory)] [string] $OutputDirectory
)

# Captures the real candidate window (fcitx5-ui.exe --demo) in the layouts the
# README shows, in light and dark mode, and composes one image per mode:
# candidate-layouts-light.png and candidate-layouts-dark.png. Each window is
# shown over a solid backdrop so its rounded corners and shadow come out
# against the README card color instead of whatever the desktop shows.

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = [IO.Path]::GetFullPath($PortableRoot)
$ui = Join-Path $root 'bin/fcitx5-ui.exe'
foreach ($required in @($ui, (Join-Path $root 'portable.flag'), (Join-Path $root 'themes/default/theme.toml'))) {
  if (-not (Test-Path -LiteralPath $required)) { throw "Portable package is missing: $required" }
}
$output = [IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Force -Path $output | Out-Null
$dataRoot = Join-Path $root 'data'
New-Item -ItemType Directory -Force -Path $dataRoot | Out-Null

Add-Type -AssemblyName System.Drawing, System.Windows.Forms
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class ReadmeCaptureNative {
    public delegate bool EnumWindowsProc(IntPtr window, IntPtr parameter);

    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }

    [DllImport("user32.dll")] private static extern bool EnumWindows(EnumWindowsProc callback, IntPtr parameter);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll")] private static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out RECT rectangle);
    [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);

    public static IntPtr FindVisibleWindow(uint expectedProcessId) {
        IntPtr result = IntPtr.Zero;
        EnumWindows((window, parameter) => {
            uint processId;
            GetWindowThreadProcessId(window, out processId);
            if (processId == expectedProcessId && IsWindowVisible(window)) { result = window; return false; }
            return true;
        }, IntPtr.Zero);
        return result;
    }
}
'@
[void][ReadmeCaptureNative]::SetProcessDpiAwarenessContext([IntPtr](-4))

$modes = [ordered]@{
  light = @{ backdrop = [Drawing.Color]::FromArgb(0xF6, 0xF8, 0xFA); caption = [Drawing.Color]::FromArgb(0x0F, 0x76, 0x6E) }
  dark = @{ backdrop = [Drawing.Color]::FromArgb(0x16, 0x1B, 0x22); caption = [Drawing.Color]::FromArgb(0x34, 0xD3, 0x99) }
}
$layouts = @(
  @{ name = 'scroll'; settings = "layout_type = `"scroll`"`nscroll_direction = `"horizontal`"" }
  @{ name = 'stacked'; settings = 'layout_type = "stacked"' }
  @{ name = 'vertical_text'; settings = "layout_type = `"vertical_text`"`nvertical_text_column_direction = `"right_to_left`"" }
)
$margin = 20

function Get-StableRectangle([IntPtr] $window) {
  $last = $null; $stable = 0
  for ($attempt = 0; $attempt -lt 200; $attempt++) {
    $rectangle = [ReadmeCaptureNative+RECT]::new()
    if (-not [ReadmeCaptureNative]::GetWindowRect($window, [ref]$rectangle)) { throw 'GetWindowRect failed.' }
    $signature = "$($rectangle.Left),$($rectangle.Top),$($rectangle.Right),$($rectangle.Bottom)"
    if ($signature -eq $last) { $stable++ } else { $last = $signature; $stable = 0 }
    if ($stable -ge 5) { return $rectangle }
    Start-Sleep -Milliseconds 25
  }
  throw 'The candidate window did not settle.'
}

function Get-Capture([string] $mode, [hashtable] $layout, [Drawing.Color] $backdropColor) {
  $config = "format_version = 1`n`n[appearance]`nmode = `"$mode`"`ntheme = `"builtin:default`"`n`n[candidate]`n$($layout.settings)`n"
  [IO.File]::WriteAllText((Join-Path $dataRoot 'config.toml'), $config, [Text.UTF8Encoding]::new($false))

  $backdrop = [Windows.Forms.Form]::new()
  $backdrop.FormBorderStyle = 'None'
  $backdrop.StartPosition = 'Manual'
  $backdrop.Bounds = [Windows.Forms.Screen]::PrimaryScreen.Bounds
  $backdrop.BackColor = $backdropColor
  $backdrop.ShowInTaskbar = $false
  $backdrop.Show()
  [Windows.Forms.Application]::DoEvents()
  $process = Start-Process -FilePath $ui -ArgumentList '--demo' -WorkingDirectory (Split-Path $ui) -PassThru
  try {
    $window = [IntPtr]::Zero
    for ($attempt = 0; $attempt -lt 200 -and $window -eq [IntPtr]::Zero; $attempt++) {
      $process.Refresh()
      if ($process.HasExited) { throw "fcitx5-ui.exe --demo exited with $($process.ExitCode)." }
      $window = [ReadmeCaptureNative]::FindVisibleWindow([uint32]$process.Id)
      [Windows.Forms.Application]::DoEvents()
      Start-Sleep -Milliseconds 25
    }
    if ($window -eq [IntPtr]::Zero) { throw 'The candidate window was not shown.' }
    # The window becomes visible before the synthetic model finishes its first reflow.
    Start-Sleep -Milliseconds 500
    [Windows.Forms.Application]::DoEvents()
    $rectangle = Get-StableRectangle $window
    $screen = [Windows.Forms.Screen]::PrimaryScreen.Bounds
    $left = [Math]::Max($screen.Left, $rectangle.Left - $margin)
    $top = [Math]::Max($screen.Top, $rectangle.Top - $margin)
    $width = [Math]::Min($screen.Right, $rectangle.Right + $margin) - $left
    $height = [Math]::Min($screen.Bottom, $rectangle.Bottom + $margin) - $top
    $bitmap = [Drawing.Bitmap]::new($width, $height, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
      $graphics.CopyFromScreen($left, $top, 0, 0, [Drawing.Size]::new($width, $height),
        [Drawing.CopyPixelOperation]::SourceCopy -bor [Drawing.CopyPixelOperation]::CaptureBlt)
    } finally {
      $graphics.Dispose()
    }
    $bitmap.Save((Join-Path $output "candidate-$($layout.name)-$mode.png"), [Drawing.Imaging.ImageFormat]::Png)
    return $bitmap
  } finally {
    if (-not $process.HasExited) { $process.Kill($true); $process.WaitForExit(5000) | Out-Null }
    $backdrop.Close()
    $backdrop.Dispose()
  }
}

foreach ($mode in $modes.Keys) {
  $style = $modes[$mode]
  $captures = @($layouts | ForEach-Object { Get-Capture $mode $_ $style.backdrop })
  $gap = 24; $padding = 28; $captionHeight = 34
  $contentWidth = ($captures | Measure-Object -Property Width -Sum).Sum + $gap * ($captures.Count - 1)
  $contentHeight = ($captures | Measure-Object -Property Height -Maximum).Maximum
  $canvas = [Drawing.Bitmap]::new($contentWidth + 2 * $padding, $contentHeight + $captionHeight + 2 * $padding,
    [Drawing.Imaging.PixelFormat]::Format32bppArgb)
  $graphics = [Drawing.Graphics]::FromImage($canvas)
  try {
    $graphics.SmoothingMode = 'AntiAlias'
    $graphics.TextRenderingHint = 'AntiAliasGridFit'
    $graphics.Clear([Drawing.Color]::Transparent)
    $card = [Drawing.Drawing2D.GraphicsPath]::new()
    $radius = 24; $w = $canvas.Width - 1; $h = $canvas.Height - 1
    $card.AddArc(0, 0, $radius, $radius, 180, 90); $card.AddArc($w - $radius, 0, $radius, $radius, 270, 90)
    $card.AddArc($w - $radius, $h - $radius, $radius, $radius, 0, 90); $card.AddArc(0, $h - $radius, $radius, $radius, 90, 90)
    $card.CloseFigure()
    $brush = [Drawing.SolidBrush]::new($style.backdrop)
    $graphics.FillPath($brush, $card)
    $font = [Drawing.Font]::new('Consolas', 13, [Drawing.GraphicsUnit]::Pixel)
    $captionBrush = [Drawing.SolidBrush]::new($style.caption)
    $format = [Drawing.StringFormat]::new(); $format.Alignment = 'Center'
    $x = $padding
    for ($index = 0; $index -lt $captures.Count; $index++) {
      $capture = $captures[$index]
      $y = $padding + [Math]::Floor(($contentHeight - $capture.Height) / 2)
      $graphics.DrawImageUnscaled($capture, $x, $y)
      $graphics.DrawString($layouts[$index].name, $font, $captionBrush,
        [Drawing.RectangleF]::new($x, $padding + $contentHeight + 10, $capture.Width, $captionHeight), $format)
      $x += $capture.Width + $gap
    }
  } finally {
    $graphics.Dispose()
  }
  $canvas.Save((Join-Path $output "candidate-layouts-$mode.png"), [Drawing.Imaging.ImageFormat]::Png)
  $canvas.Dispose()
  $captures | ForEach-Object { $_.Dispose() }
}
Remove-Item -LiteralPath (Join-Path $dataRoot 'config.toml') -Force
Write-Host "README candidate images written to $output"
