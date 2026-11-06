param([string]$OutputPath = 'crates/network/tests/fixtures/monitor.json')
$ErrorActionPreference = 'Stop'
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$temporaryDirectory = Join-Path ([IO.Path]::GetTempPath()) ('hmn-monitor-oracle-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $temporaryDirectory
$sourcePath = Join-Path $temporaryDirectory 'monitor.go'
$harnessPath = Join-Path $temporaryDirectory 'main.go'
try {
    # Compile the actual frozen source, not a hand-transcribed Go oracle.
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'cmd/server/h2monitor.go') -Destination $sourcePath
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'monitor/main.go.txt') -Destination $harnessPath
    $result = & go run $sourcePath $harnessPath
    if ($LASTEXITCODE -ne 0) { throw 'Frozen monitor oracle failed' }
    [IO.File]::WriteAllText((Join-Path $repositoryRoot $OutputPath), ($result -join "`n") + "`n", [Text.UTF8Encoding]::new($false))
} finally {
    foreach ($temporaryFile in @($sourcePath, $harnessPath)) {
        if (Test-Path -LiteralPath $temporaryFile) { Remove-Item -LiteralPath $temporaryFile }
    }
    Remove-Item -LiteralPath $temporaryDirectory
}
