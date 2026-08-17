[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$localeFiles = Get-ChildItem -LiteralPath (Join-Path $repoRoot 'locales') -Filter '*.json' -File
if ($localeFiles.Count -lt 2) { throw 'At least en-US.json and zh-CN.json are required.' }

$required = @(
  'app.title', 'status.label', 'action.refresh', 'action.restart',
  'action.reset_config', 'language.hint', 'error.control_missing', 'error.command'
)
$referenceKeys = $null
foreach ($file in $localeFiles) {
  $bytes = [IO.File]::ReadAllBytes($file.FullName)
  if ($bytes.Length -gt 2MB) { throw "Locale exceeds 2 MiB: $($file.Name)" }
  if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
    throw "UTF-8 BOM is forbidden: $($file.Name)"
  }
  $options = [Text.Json.JsonDocumentOptions]::new()
  $options.AllowTrailingCommas = $false
  $options.CommentHandling = [Text.Json.JsonCommentHandling]::Disallow
  $utf8 = [Text.UTF8Encoding]::new($false, $true)
  $jsonText = $utf8.GetString($bytes)
  $document = [Text.Json.JsonDocument]::Parse($jsonText, $options)
  try {
    $root = $document.RootElement
    if ($root.ValueKind -ne [Text.Json.JsonValueKind]::Object) { throw "Locale root must be an object: $($file.Name)" }
    $keys = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
    foreach ($property in $root.EnumerateObject()) {
      if (-not $keys.Add($property.Name)) { throw "Duplicate locale key '$($property.Name)': $($file.Name)" }
      if ($property.Name -eq 'format_version') {
        if ($property.Value.ValueKind -ne [Text.Json.JsonValueKind]::Number -or $property.Value.GetInt32() -ne 1) {
          throw "format_version must be exactly 1: $($file.Name)"
        }
      } elseif ($property.Value.ValueKind -ne [Text.Json.JsonValueKind]::String) {
        throw "Locale value must be a string: $($file.Name):$($property.Name)"
      }
    }
    foreach ($key in $required) { if (-not $keys.Contains($key)) { throw "Missing locale key '$key': $($file.Name)" } }
    $ordered = @($keys | Where-Object { $_ -ne 'format_version' } | Sort-Object)
    if ($null -eq $referenceKeys) { $referenceKeys = $ordered }
    elseif (Compare-Object $referenceKeys $ordered) { throw "Locale key set differs: $($file.Name)" }
  } finally {
    $document.Dispose()
  }
}
Write-Host "Locale validation passed ($($localeFiles.Count) files, strict JSON, matching keys)."
