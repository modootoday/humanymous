# Standard-user process: receives no runner credentials or runner environment.
param([string]$Manifest, [string]$Scratch, [string]$ExpectedSid, [string]$Workspace)
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false
$result = @{ status = 'failed'; harnesses = @() }
try {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    $adminSid = [Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
    if ($identity.User.Value -ne $ExpectedSid -or $identity.Groups.Value -contains $adminSid.Value -or $principal.IsInRole($adminSid)) { throw 'Unexpected child SID/admin membership' }
    New-Item -ItemType Directory -Path $env:TEMP -ErrorAction Stop | Out-Null
    if ((Get-Acl -LiteralPath $env:TEMP).Owner -ne $identity.Name) { throw 'Temporary directory not owned by child' }
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ProbeToken {
 [DllImport("advapi32.dll", SetLastError=true)] static extern bool GetTokenInformation(IntPtr token, int kind, out int value, int size, out int length);
 public static bool Elevated(IntPtr token) { int value, length; if (!GetTokenInformation(token,20,out value,4,out length)) throw new System.ComponentModel.Win32Exception(); return value != 0; }
}
'@
    $elevated = [ProbeToken]::Elevated($identity.Token)
    if ($elevated) { throw 'Elevated child token forbidden' }
    $result.token = @{ sid = $identity.User.Value; elevated = $elevated; administrator_group = $false; groups = @($identity.Groups.Value) }
    Set-Location -LiteralPath $Workspace
    $entries = @(Get-Content -LiteralPath $Manifest -Raw | ConvertFrom-Json)
    for ($index = 0; $index -lt $entries.Count; $index++) {
        $entry = $entries[$index]
        if ((Get-FileHash -LiteralPath $entry.executable -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) { throw 'Executable identity changed' }
        & $entry.executable --list --format terse *> (Join-Path $Scratch "harness-$index-list.log")
        $listExit = $LASTEXITCODE
        & $entry.executable --format pretty *> (Join-Path $Scratch "harness-$index-run.log")
        $runExit = $LASTEXITCODE
        $result.harnesses += @{ executable = $entry.executable; list_exit = $listExit; run_exit = $runExit }
    }
    $result.status = 'executed'
} catch { $result.error = $_.Exception.Message }
finally { $result | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $Scratch 'launcher-result.json') -Encoding utf8 }
if ($result.status -ne 'executed') { exit 1 }
