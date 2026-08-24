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
$contentType = 'application/x.slipcase+zip'
$progId = 'Excelano.Slipcase'
$exeName = 'slipcase-desktop.exe'

# The .NET API for the same reason install.ps1 uses it: PowerShell's registry
# provider reads the forward slash in the media type as a path separator, so it
# would look for the wrong key here and leave the right one behind.
function Remove-Key {
    param([string] $Path)
    try {
        [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($Path, $false)
    } catch {
        Write-Verbose "nothing at $Path"
    }
}

$classes = 'Software\Classes'

Remove-Key "$classes\$progId"
Remove-Key "$classes\$extension"
Remove-Key "$classes\MIME\Database\Content Type\$contentType"
Remove-Key "$classes\Applications\$exeName"
Remove-Key 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Slipcase'

# The one that is easy to miss. Choosing "always open with" writes a UserChoice
# here, and a UserChoice outranks everything removed above: leaving it behind
# leaves the extension pointing at a ProgID that no longer exists, which is the
# dead association this script exists to prevent. Windows treats such a choice
# as no association at all rather than falling back to the machine-wide one —
# measured, and the reason `src/opens_with.rs` does not fall back either.
Remove-Key "Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$extension"

$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Slipcase.lnk'
if (Test-Path -LiteralPath $shortcut) { Remove-Item -LiteralPath $shortcut -Force -Confirm:$false }

if (-not $KeepFiles) {
    foreach ($name in $exeName, 'slipcase.ico') {
        $path = Join-Path $Prefix $name
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force -Confirm:$false }
    }
    # This script is running from inside the directory it is emptying, so it
    # cannot delete itself here. It is left, and the directory with it, and the
    # next install overwrites both; a directory holding one script is not worth
    # a scheduled deletion to be rid of.
    Write-Output "left $(Join-Path $Prefix 'uninstall.ps1') behind: it is the script now running"
}

Add-Type -Namespace SlipcaseUninstall -Name Shell -MemberDefinition @'
[DllImport("shell32.dll", CharSet=CharSet.Unicode)]
public static extern void SHChangeNotify(int eventId, uint flags, System.IntPtr item1, System.IntPtr item2);
'@
[SlipcaseUninstall.Shell]::SHChangeNotify(0x08000000, 0, [System.IntPtr]::Zero, [System.IntPtr]::Zero)

Write-Output "removed the slipcase association, the Start menu entry, and the uninstall entry"
