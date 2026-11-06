# Provisioning is permitted only on an authorized disposable GitHub-hosted VM.
param([string]$Manifest, [string]$ReportDirectory, [string]$Workspace)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted' -or !$IsWindows) { throw 'Disposable GitHub-hosted Windows required' }
$runnerIdentity = [Security.Principal.WindowsIdentity]::GetCurrent()
if (!([Security.Principal.WindowsPrincipal]::new($runnerIdentity)).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'Provisioning requires administrator runner' }
$name = 'hmp' + [Guid]::NewGuid().ToString('N').Substring(0, 12)
$password = [Security.SecureString]::new()
# The credential never enters logs, argv, environment or files.
foreach ($character in ('aA1!' + [Convert]::ToBase64String([Security.Cryptography.RandomNumberGenerator]::GetBytes(36))).ToCharArray()) { $password.AppendChar($character) }
$password.MakeReadOnly()
$scratch = Join-Path $env:ProgramData $name
$created = $false
$process = $null
$failure = $null
try {
    $user = New-LocalUser -Name $name -Password $password -AccountExpires (Get-Date).AddHours(2) -UserMayNotChangePassword
    $created = $true
    $users = Get-LocalGroup -SID 'S-1-5-32-545'
    Add-LocalGroupMember -Group $users -Member $user
    $memberships = @(Get-LocalGroup | Where-Object { @(Get-LocalGroupMember -Group $_ -ErrorAction Stop | Where-Object SID -eq $user.SID).Count -gt 0 })
    if ($memberships.Count -ne 1 -or $memberships[0].SID.Value -ne 'S-1-5-32-545') { throw 'Account must belong only to Users' }
    New-Item -ItemType Directory -Path $scratch | Out-Null
    $acl = [Security.AccessControl.DirectorySecurity]::new()
    $acl.SetAccessRuleProtection($true, $false)
    foreach ($sid in @($user.SID, $runnerIdentity.User, [Security.Principal.SecurityIdentifier]::new('S-1-5-18'))) {
        $acl.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','ContainerInherit,ObjectInherit','None','Allow'))
    }
    Set-Acl -LiteralPath $scratch -AclObject $acl
    # Add this SID only; preserve existing workspace permissions. No write grant.
    & icacls.exe $Workspace /grant "*$($user.SID.Value):(OI)(CI)(RX)" /T /Q | Out-Null
    if ($LASTEXITCODE) { throw 'Workspace read/execute grant failed' }
    # A hosted image can inherit broad Users write permissions. Restrict this
    # disposable SID explicitly without changing any preexisting principal.
    # Specific write bits preserve READ_CONTROL/SYNCHRONIZE shared by generic
    # read and write masks; denying generic W would also break script reads.
    & icacls.exe $Workspace /deny "*$($user.SID.Value):(OI)(CI)(WD,AD,WEA,WA,DE,DC,WDAC,WO)" /T /Q | Out-Null
    if ($LASTEXITCODE) { throw 'Workspace write restriction failed' }
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = (Get-Command pwsh).Source
    $start.UseShellExecute = $false
    $start.UserName = $name
    $start.Domain = $env:COMPUTERNAME
    $start.Password = $password
    $start.LoadUserProfile = $true
    $start.CreateNoWindow = $true
    $start.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
    $start.WorkingDirectory = $Workspace
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.Environment.Clear()
    $start.Environment['SystemRoot'] = $env:SystemRoot
    $start.Environment['WINDIR'] = $env:SystemRoot
    $start.Environment['COMSPEC'] = Join-Path $env:SystemRoot 'System32\cmd.exe'
    $start.Environment['PATH'] = "$env:SystemRoot\System32;$env:SystemRoot;$env:SystemRoot\System32\WindowsPowerShell\v1.0"
    $start.Environment['TEMP'] = Join-Path $scratch 'tmp'
    $start.Environment['TMP'] = Join-Path $scratch 'tmp'
    foreach ($argument in @('-NoProfile','-NonInteractive','-File',(Join-Path $PSScriptRoot 'windows-child.ps1'),'-Manifest',$Manifest,'-Scratch',$scratch,'-ExpectedSid',$user.SID.Value,'-Workspace',$Workspace)) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    if (!$process.WaitForExit(1200000)) { $process.Kill($true); $process.WaitForExit(); throw 'Child execution timed out' }
    $stdout.GetAwaiter().GetResult() | Set-Content (Join-Path $ReportDirectory 'child-stdout.log')
    $stderr.GetAwaiter().GetResult() | Set-Content (Join-Path $ReportDirectory 'child-stderr.log')
    # Copy only expected evidence; never copy account profiles/temp contents.
    $entries = @(Get-Content -LiteralPath $Manifest -Raw | ConvertFrom-Json)
    foreach ($file in @('launcher-result.json') + @(0..($entries.Count-1) | ForEach-Object { "harness-$_-list.log"; "harness-$_-run.log" })) {
        $source = Join-Path $scratch $file
        if (Test-Path -LiteralPath $source) { Copy-Item -LiteralPath $source -Destination (Join-Path $ReportDirectory $file) }
    }
    if ($process.ExitCode) { throw 'Standard-user child failed' }
} catch { $failure = $_.Exception.Message }
finally {
    if ($process -and !$process.HasExited) { $process.Kill($true); $process.WaitForExit() }
    if ($created) {
        Remove-LocalUser -Name $name -ErrorAction Stop
        if (Get-LocalUser -Name $name -ErrorAction SilentlyContinue) { throw 'Account cleanup failed' }
    }
    $password.Dispose()
    # Generated directory under ProgramData; verify containment before deletion.
    $expected = [IO.Path]::GetFullPath((Join-Path $env:ProgramData $name))
    if ([IO.Path]::GetFullPath($scratch) -ne $expected -or $name -notmatch '^hmp[0-9a-f]{12}$') { throw 'Unsafe scratch cleanup path' }
    if (Test-Path -LiteralPath $scratch) { Remove-Item -LiteralPath $scratch -Recurse -Force }
}
@{ account_deleted = $true; scratch_deleted = $true } | ConvertTo-Json | Set-Content (Join-Path $ReportDirectory 'cleanup.json')
if ($failure) { throw $failure }
