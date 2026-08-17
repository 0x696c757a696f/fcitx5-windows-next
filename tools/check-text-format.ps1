[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$textExtensions = @(
  '.c', '.cc', '.cmake', '.cpp', '.cxx', '.def', '.h', '.hpp',
  '.json', '.md', '.ps1', '.toml', '.txt', '.xml', '.yaml', '.yml'
)
$textNames = @('.clang-format', '.editorconfig', '.gitattributes', '.gitignore')
$utf8 = [System.Text.UTF8Encoding]::new($false, $true)
$violations = [Collections.Generic.List[string]]::new()

Push-Location $repoRoot
try {
  $files = & git ls-files --cached --others --exclude-standard
  if ($LASTEXITCODE -ne 0) { throw 'git ls-files failed.' }
  foreach ($relativePath in $files) {
    $name = [System.IO.Path]::GetFileName($relativePath)
    $extension = [System.IO.Path]::GetExtension($relativePath).ToLowerInvariant()
    if ($textNames -notcontains $name -and $textExtensions -notcontains $extension) { continue }
    $path = Join-Path $repoRoot $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { continue }
    $bytes = [System.IO.File]::ReadAllBytes($path)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and
        $bytes[2] -eq 0xBF) {
      $violations.Add("$relativePath`: UTF-8 BOM is not allowed")
    }
    try {
      [void] $utf8.GetString($bytes)
    } catch {
      $violations.Add("$relativePath`: not valid UTF-8")
      continue
    }
    for ($index = 0; $index -lt $bytes.Length; ++$index) {
      if ($bytes[$index] -eq 13) {
        $violations.Add("$relativePath`: CR/CRLF found; LF is required")
        break
      }
    }
  }
} finally {
  Pop-Location
}

if ($violations.Count -ne 0) {
  $violations | ForEach-Object { Write-Error $_ }
  throw "Text-format check failed with $($violations.Count) violation(s)."
}
Write-Host "Text-format check passed ($($files.Count) repository files considered): UTF-8 without BOM, LF."
