[CmdletBinding()]
param(
  [switch] $InstallForCI
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Add-PathForCurrentAndFutureSteps {
  param([Parameter(Mandatory)] [string] $Directory)
  if (-not (Test-Path -LiteralPath $Directory -PathType Container)) { return }
  $env:PATH = "$Directory;$env:PATH"
  if (-not [string]::IsNullOrWhiteSpace($env:GITHUB_PATH)) {
    $Directory | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
  }
}

Add-PathForCurrentAndFutureSteps (Join-Path $env:ProgramFiles 'LLVM/bin')
Add-PathForCurrentAndFutureSteps (Join-Path $env:ProgramData 'chocolatey/bin')

if ($InstallForCI) {
  $missing = @('clang-cl', 'lld-link', 'ninja') |
    Where-Object { -not (Get-Command $_ -ErrorAction SilentlyContinue) }
  if ($missing.Count -ne 0) {
    $choco = Get-Command choco -ErrorAction SilentlyContinue
    if (-not $choco) {
      throw "Chocolatey is required on CI to install missing fast toolchain tools: $($missing -join ', ')."
    }
    if ($missing -contains 'clang-cl' -or $missing -contains 'lld-link') {
      & $choco.Source install llvm -y --no-progress
      if ($LASTEXITCODE -ne 0) { throw 'Failed to install LLVM via Chocolatey.' }
      Add-PathForCurrentAndFutureSteps (Join-Path $env:ProgramFiles 'LLVM/bin')
    }
    if ($missing -contains 'ninja') {
      & $choco.Source install ninja -y --no-progress
      if ($LASTEXITCODE -ne 0) { throw 'Failed to install Ninja via Chocolatey.' }
      Add-PathForCurrentAndFutureSteps (Join-Path $env:ProgramData 'chocolatey/bin')
    }
  }
}

$stillMissing = @('clang-cl', 'lld-link', 'ninja') |
  Where-Object { -not (Get-Command $_ -ErrorAction SilentlyContinue) }
if ($stillMissing.Count -ne 0) {
  throw "Fast Windows toolchain is missing: $($stillMissing -join ', '). Install LLVM clang-cl/lld-link and Ninja."
}

Write-Host 'Fast Windows toolchain ready:'
foreach ($tool in @('clang-cl', 'lld-link', 'ninja')) {
  $command = Get-Command $tool -ErrorAction Stop
  Write-Host "  $tool -> $($command.Source)"
}
