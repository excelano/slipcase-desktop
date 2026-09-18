# Remove what install.ps1 put in place, and tell the shell it is gone.
#
# The whole of it, because a file association that outlives its executable is
# worse than none: Explorer keeps drawing the icon and offering the type, and
# double-clicking fails with a message about a missing file rather than the
# dialog that would have let a person pick something else.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)

[CmdletBinding()]
param(
    [string] $Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\Slipcase'),
    # Leave the installed executable and icon where they are.
    [switch] $KeepFiles
)

$ErrorActionPreference = 'Stop'

$extension = '.slpc'
$contentType = 'application/vnd.excelano.slipcase+zip'

# What install.ps1 wrote before IANA registered the one above on 2026-09-16.
# Named here as well as there because an uninstall can meet either: an upgrade
# removes this key, and a machine that never upgraded still holds it.
$supersededContentType = 'application/x.slipcase+zip'
$progId = 'Excelano.Slipcase'
$exeName = 'slipcase-desktop.exe'

function Test-OurKey {
    param([string] $Path)
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Path, $false)
    if (-not $key) { return $false }
    $key.Close()
    return $true
}

# The .NET API for the same reason install.ps1 uses it: PowerShell's registry
# provider reads the forward slash in the media type as a path separator, so it
# would look for the wrong key here and leave the right one behind.
#
# A key that is not there is nothing to do; a key that is there and will not go
# is a failure, and catching every exception cannot tell the two apart. Both
# are read back, so the only thing passed over is the absence.
function Remove-Key {
    param([string] $Path)
    if (-not (Test-OurKey $Path)) { return }
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($Path, $false)
    if (Test-OurKey $Path) { throw "uninstall.ps1: HKCU\$Path is still there after being deleted" }
}

# The same, for a key that cannot be opened for writing. `DeleteSubKeyTree` and
# `reg delete` both open the key itself with write access before deleting it,
# and Explorer writes a *Deny SetValue* rule on UserChoice so that no
# application can quietly take an extension over. That deny makes the write
# open fail, and then `reg delete` says *Access is denied* while
# `DeleteSubKeyTree` reads the failure as the key being missing and returns
# quietly. Deleting the name from the parent needs DELETE on the child and
# nothing else, which the rule beside the deny allows, unelevated.
function Remove-Subkey {
    param([string] $Parent, [string] $Name)
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Parent, $true)
    if (-not $key) { return }
    try { $key.DeleteSubKey($Name, $false) } finally { $key.Close() }
    if (Test-OurKey "$Parent\$Name") {
        throw "uninstall.ps1: HKCU\$Parent\$Name is still there after being deleted"
    }
}

# A value, or the key's default when $Name is empty, removed only if it holds
# what install.ps1 wrote, so that another application's registration on the same
# extension is not touched.
function Remove-OurValue {
    param([string] $Path, [string] $Name, [string] $Ours)
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($Path, $true)
    if (-not $key) { return }
    try {
        $held = $key.GetValue($Name, $null)
        if ($null -ne $held -and ($Ours -eq '' -or $held -eq $Ours)) {
            $key.DeleteValue($Name, $false)
        }
    } finally {
        $key.Close()
    }
}

$classes = 'Software\Classes'

Remove-Key "$classes\$progId"
# The extension's own values rather than the whole key. `OpenWithProgids`
# belongs to the extension and to every application that has ever offered to
# open one, so removing the tree takes somebody else's offer with it — which is
# true even of a format that is this product's own, because nothing stops an
# archive tool offering to open a .slpc. This file removed the tree until the
# shared install check planted a neighbour and watched it go; segler was made
# surgical the same day and this is the same rule.
Remove-OurValue "$classes\$extension\OpenWithProgids" $progId ''
Remove-OurValue "$classes\$extension" '' $progId
Remove-OurValue "$classes\$extension" 'Content Type' $contentType
Remove-Key "$classes\MIME\Database\Content Type\$contentType"
# Both names, because this script has to clear what any version of install.ps1
# wrote and not only the current one. `Remove-OurValue` compares before it
# deletes, so the extension's `Content Type` is touched only if it still holds
# the superseded string — on an upgraded machine it holds the registered one and
# the line above has already taken it.
Remove-OurValue "$classes\$extension" 'Content Type' $supersededContentType
Remove-Key "$classes\MIME\Database\Content Type\$supersededContentType"
Remove-Key "$classes\Applications\$exeName"
Remove-Key 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Slipcase'

# The one that is easy to miss. Choosing "always open with" writes a UserChoice
# here, and so does opening a file through the association; a UserChoice
# outranks everything removed above, so leaving it behind leaves the extension
# pointing at a ProgID that no longer exists, which is the dead association
# this script exists to prevent. Windows treats such a choice as no association
# at all rather than falling back to the machine-wide one, which is also why
# `src/opens_with.rs` does not fall back.
#
# The UserChoice key by name through `Remove-Subkey`, and not the
# `FileExts\.slpc` tree above it: that tree holds other applications' entries
# for the extension, and the deny rule on UserChoice defeats a tree delete
# silently. Removed only when it names this application.
$exts = "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$extension"
$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("$exts\UserChoice", $false)
if ($key) {
    $chosen = $key.GetValue('ProgId', $null)
    $key.Close()
    if ($chosen -eq $progId) { Remove-Subkey $exts 'UserChoice' }
}

$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Slipcase.lnk'
if (Test-Path -LiteralPath $shortcut) { Remove-Item -LiteralPath $shortcut -Force -Confirm:$false }

if (-not $KeepFiles) {
    foreach ($name in $exeName, 'slipcase.ico') {
        $path = Join-Path $Prefix $name
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force -Confirm:$false }
    }
    # Add/Remove Programs points at the copy inside the directory, so the usual
    # run is a script emptying the directory it is itself in, and a running
    # script cannot delete itself. Run from a checkout it is not that file, and
    # then the copy is an ordinary file that can go with the rest.
    #
    # Both branches were one line until 2026-08-26, when a run from the
    # checkout left the copy behind and said it was the script now running.
    # That was untrue and it left a directory this script says it removes, so
    # the two cases are told apart rather than assumed to be the same one.
    $copy = Join-Path $Prefix 'uninstall.ps1'
    $self = $MyInvocation.MyCommand.Path
    if (Test-Path -LiteralPath $copy) {
        $same = $self -and
            ([System.IO.Path]::GetFullPath($self) -ieq [System.IO.Path]::GetFullPath($copy))
        if ($same) {
            Write-Output "left ${copy} behind: it is the script now running"
        } else {
            Remove-Item -LiteralPath $copy -Force -Confirm:$false
        }
    }
    # And the directory, where emptying it emptied it. Left alone if anything
    # else is in there, because this script installed none of it.
    if ((Test-Path -LiteralPath $Prefix) -and
        -not (Get-ChildItem -LiteralPath $Prefix -Force)) {
        Remove-Item -LiteralPath $Prefix -Force -Confirm:$false
    }
}

Add-Type -Namespace SlipcaseUninstall -Name Shell -MemberDefinition @'
[DllImport("shell32.dll", CharSet=CharSet.Unicode)]
public static extern void SHChangeNotify(int eventId, uint flags, System.IntPtr item1, System.IntPtr item2);
'@
[SlipcaseUninstall.Shell]::SHChangeNotify(0x08000000, 0, [System.IntPtr]::Zero, [System.IntPtr]::Zero)

Write-Output "removed the Slipcase association, the Start menu entry, and the uninstall entry"
