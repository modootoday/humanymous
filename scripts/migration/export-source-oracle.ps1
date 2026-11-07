param(
    [string[]]$SourceFiles = @('cmd/server/h2monitor.go'),
    [string]$Harness = 'scripts/migration/monitor/main.go.txt',
    [string]$OutputPath = 'crates/sdk/humanymous-runtime/tests/fixtures/network/monitor.json'
)
$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) ('hmn-source-oracle-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $temporaryDirectory
$copiedSources = @()
$harnessPath = Join-Path $temporaryDirectory 'main.go'
try {
    # Compile the actual frozen source, not a hand-transcribed Go oracle.
    foreach ($sourceFile in $SourceFiles) {
        $sourcePath = Join-Path $temporaryDirectory ([IO.Path]::GetFileName($sourceFile))
        if (Test-Path -LiteralPath $sourcePath) { throw 'Duplicate oracle source filename' }
        $sourceText = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot $sourceFile)
        # Only change package ownership in the temporary copy; functions remain exact.
        $sourceText = [regex]::Replace($sourceText, '(?m)^package \w+\s*$', 'package main')
        [IO.File]::WriteAllText($sourcePath, $sourceText, [Text.UTF8Encoding]::new($false))
        $copiedSources += $sourcePath
    }
    Copy-Item -LiteralPath (Join-Path $repositoryRoot $Harness) -Destination $harnessPath
    $result = & go run @copiedSources $harnessPath
    if ($LASTEXITCODE -ne 0) { throw 'Frozen source oracle failed' }
    [IO.File]::WriteAllText((Join-Path $repositoryRoot $OutputPath), ($result -join "`n") + "`n", [Text.UTF8Encoding]::new($false))
} finally {
    foreach ($temporaryFile in @($copiedSources) + @($harnessPath)) {
        if (Test-Path -LiteralPath $temporaryFile) { Remove-Item -LiteralPath $temporaryFile }
    }
    Remove-Item -LiteralPath $temporaryDirectory
}
