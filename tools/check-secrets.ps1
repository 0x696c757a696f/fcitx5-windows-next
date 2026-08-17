[CmdletBinding()]
param([switch] $SelfTest)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$patterns = @(
  [regex]::new('-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----'),
  [regex]::new('github_pat_[A-Za-z0-9_]{70,}'),
  [regex]::new('gh[pousr]_[A-Za-z0-9]{36,}'),
  [regex]::new('AKIA[0-9A-Z]{16}')
)

function Test-SecretText([string] $Text) {
  foreach ($pattern in $patterns) {
    if ($pattern.IsMatch($Text)) {
      return $true
    }
  }
  return $false
}

if ($SelfTest) {
  $badCase = '-----BEGIN ' + 'PRIVATE KEY-----'
  $goodCase = 'github_pat_example_is_documentation_not_a_token'
  if (-not (Test-SecretText $badCase) -or (Test-SecretText $goodCase)) {
    throw 'Secret scanner paired self-test failed.'
  }
  Write-Host 'Secret scanner paired self-test passed.'
  return
}

$repoRoot = [System.IO.Path]::GetFullPath((Split-Path -Parent $PSScriptRoot))
$relativeFiles = & git -C $repoRoot ls-files --cached --others --exclude-standard
if ($LASTEXITCODE -ne 0) {
  throw 'Unable to enumerate repository files with git.'
}

$findings = [System.Collections.Generic.List[string]]::new()
foreach ($relativeFile in $relativeFiles) {
  $path = Join-Path $repoRoot $relativeFile
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
    continue
  }
  $bytes = [System.IO.File]::ReadAllBytes($path)
  if ($bytes -contains 0) {
    continue
  }
  $lineNumber = 0
  foreach ($line in [System.IO.File]::ReadLines($path)) {
    $lineNumber++
    if (Test-SecretText $line) {
      $findings.Add("${relativeFile}:${lineNumber}")
    }
  }
}

if ($findings.Count -gt 0) {
  $locations = $findings -join [Environment]::NewLine
  throw "High-confidence secret material found:`n$locations"
}

Write-Host "Secret scan passed ($($relativeFiles.Count) files)."
