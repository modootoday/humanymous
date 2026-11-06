param([Parameter(Mandatory = $true)][string]$Destination)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$target = [IO.Path]::GetFullPath($Destination)
if (Test-Path -LiteralPath $target) { throw 'Destination must not exist; no shared or prior workspace is overwritten' }
$contexts = @('signals', 'behavior', 'model', 'scoring', 'test-support', 'attacker', 'defender')
New-Item -ItemType Directory -Path (Join-Path $target 'crates') -Force | Out-Null
foreach ($context in $contexts) {
    Copy-Item -LiteralPath (Join-Path $repo "crates/$context") -Destination (Join-Path $target 'crates') -Recurse
}
$manifest = Get-Content -LiteralPath (Join-Path $repo 'Cargo.toml') -Raw
$members = ($contexts | ForEach-Object { '"crates/' + $_ + '"' }) -join ', '
$manifest = [regex]::Replace($manifest, '(?ms)^members\s*=\s*\[.*?\]', "members = [$members]", 1)
[IO.File]::WriteAllText((Join-Path $target 'Cargo.toml'), $manifest)
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Dockerfile') -Destination $target
[IO.File]::WriteAllText((Join-Path $target '.dockerignore'), "target/`n.git/`n")
Write-Output $target
