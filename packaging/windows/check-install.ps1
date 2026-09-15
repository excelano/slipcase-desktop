<#
.SYNOPSIS
    Install the Windows integration, take it away again, and refuse if anything
    of ours is left behind.

.DESCRIPTION
    `check-imports.ps1` beside this checks the artefact the Store distributes.
    This checks the other half: that the install scripts themselves go on and
    come off cleanly, which nothing else reaches.

    The state it guards against is a `UserChoice` left behind naming a ProgID
    that has just been deleted, which kills the extension outright. Explorer
    writes a *Deny SetValue* rule on that key, so every delete that opens it for
    writing fails, and an exception caught wholesale makes that failure look
    exactly like the key never having existed.

    What is checked, in the order a person would look:

      * the ProgID, the media type and the extension are written
      * a `UserChoice` naming this application is taken away with them
      * a `UserChoice` naming another application is left where it is

    This registers the real ProgID under HKCU and takes it away again, so it is
    for a build agent or a machine where losing an existing `.slpc` association
    does not matter. It installs the executable nowhere: `-NoBinary` covers the
    registry, which is all that is in question here.

.PARAMETER Foreign
    The ProgID planted in a UserChoice to stand for another application's
    choice. Nothing should ever remove it.

.NOTES
    Author: David M. Anderson
    Built with AI assistance (Claude, Anthropic)
#>
[CmdletBinding()]
param(
    [string] $Foreign = 'Check.NotOurs.Slpc'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$extension = '.slpc'
$contentType = 'application/x.slipcase+zip'
$progId = 'Excelano.Slipcase'
$classes = 'Software\Classes'
$exts = "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$extension"

$failures = @()
function Expect([bool] $ok, [string] $what) {
    if ($ok) { Write-Host "  ok    $what" }
    else { Write-Host "  FAIL  $what" -ForegroundColor Red; $script:failures += $what }
}

function Get-Key([string] $Path) {
    [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Path, $false)
}

function Test-Key([string] $Path) {
    $k = Get-Key $Path
    if (-not $k) { return $false }
    $k.Close()
    return $true
}

function Get-Value([string] $Path, [string] $Name) {
    $k = Get-Key $Path
    if (-not $k) { return $null }
    try { return $k.GetValue($Name, $null) } finally { $k.Close() }
}

# Explorer's own key, as far as it can be reproduced: the ProgId value and the
# deny rule that stops it being opened for writing. Without the rule this check
# passes against the defect it was written for, which is worse than not having
# it, so the rule is the point of the function.
function New-ProtectedUserChoice([string] $Chosen) {
    # Taken away first, because the deny rule on a key left by a failed run
    # makes `CreateSubKey` throw *Access to the registry key is denied* and that
    # exception, arriving in the middle of the checks, hides which one failed.
    Remove-UserChoice
    $k = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey("$exts\UserChoice")
    $k.SetValue('ProgId', $Chosen)
    $acl = $k.GetAccessControl()
    $me = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
    $acl.AddAccessRule((New-Object System.Security.AccessControl.RegistryAccessRule(
        $me, 'SetValue', 'None', 'None', 'Deny')))
    $k.SetAccessControl($acl)
    $k.Close()
}

function Remove-UserChoice {
    $parent = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($exts, $true)
    if (-not $parent) { return }
    try { $parent.DeleteSubKey('UserChoice', $false) } finally { $parent.Close() }
}

try {
    Write-Host 'check-install: install.ps1 -NoBinary'
    & powershell -ExecutionPolicy Bypass -File (Join-Path $here 'install.ps1') -NoBinary | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "install.ps1 exited $LASTEXITCODE" }

    Expect (Test-Key "$classes\$progId") "the ProgID is written"
    Expect ((Get-Value "$classes\$extension" '') -eq $progId) "the extension names this application"
    Expect (Test-Key "$classes\MIME\Database\Content Type\$contentType") "the media type is written"

    # A person who chose "always open with" before uninstalling. Written the way
    # Explorer writes it, deny rule and all.
    New-ProtectedUserChoice $progId
    Expect ((Get-Value "$exts\UserChoice" 'ProgId') -eq $progId) "a UserChoice naming this application is in place"

    Write-Host 'check-install: uninstall.ps1'
    & powershell -ExecutionPolicy Bypass -File (Join-Path $here 'uninstall.ps1') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "uninstall.ps1 exited $LASTEXITCODE" }

    Expect (-not (Test-Key "$classes\$progId")) "the ProgID is gone"
    Expect (-not (Test-Key "$classes\$extension")) "the extension is gone"
    Expect (-not (Test-Key "$classes\MIME\Database\Content Type\$contentType")) "the media type is gone"
    Expect (-not (Test-Key "$exts\UserChoice")) "the UserChoice naming this application is gone"

    # And the other half of the rule: somebody else's choice is theirs to keep.
    New-ProtectedUserChoice $Foreign
    & powershell -ExecutionPolicy Bypass -File (Join-Path $here 'uninstall.ps1') | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "uninstall.ps1 exited $LASTEXITCODE on the second run" }
    Expect ((Get-Value "$exts\UserChoice" 'ProgId') -eq $Foreign) "a UserChoice naming another application is left where it is"
} finally {
    Remove-UserChoice
}

if ($failures.Count -gt 0) {
    Write-Host "check-install: $($failures.Count) of the checks above failed" -ForegroundColor Red
    exit 1
}
Write-Host 'check-install: the integration goes on and comes off cleanly'
