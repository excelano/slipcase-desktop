# Install the Windows integration DESIGN.md §8 describes: the extension, the
# media type, the icon, and the entry that opens a container. Optionally the
# executable alongside them.
#
# Per-user, under HKCU and %LOCALAPPDATA%, which is the counterpart of the
# Linux script's default of ~/.local: no administrator, and nothing written
# that another account can see. There is no all-users variant because the
# machine-wide half of every key here needs elevation, and a packaging script
# that sometimes needs it and sometimes does not is worse than one that never
# does.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)

[CmdletBinding()]
param(
    # Where to install. The default is the per-user location Windows names for
    # applications that do not go through an installer service.
    [string] $Prefix = (Join-Path $env:LOCALAPPDATA 'Programs\Slipcase'),
    # The executable to install. With neither this nor -NoBinary, a built one
    # is looked for.
    [string] $Binary,
    # Install the integration only.
    [switch] $NoBinary
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path

# SPEC 4 names both of these and this repository neither restates nor amends
# it. They are the only two identifiers here that were not chosen.
$extension = '.slpc'
$contentType = 'application/x.slipcase+zip'

# Chosen here. `Vendor.Component` is the shape Windows documents for a ProgID;
# no version suffix, because a `CurVer` indirection buys nothing until there is
# a second version to point at and costs a key that has to be got right.
$progId = 'Excelano.Slipcase'
$typeName = 'Slipcase Container'
$appName = 'Slipcase'
$exeName = 'slipcase-desktop.exe'
$iconName = 'slipcase.ico'

# --- writing to the registry ------------------------------------------------

# The .NET API rather than PowerShell's registry provider, because of one key
# here: the media type is `application/x.slipcase+zip`, and the provider reads
# the forward slash as a path separator and silently creates `application` with
# a child `x.slipcase+zip` instead of the single key that was asked for.
# Measured, not guessed. The .NET API takes the whole string as one name, which
# is what the MIME database wants. An empty $Name is the key's default value.
function Set-RegistryValue {
    param([string] $Path, [string] $Name, $Value, [string] $Kind = 'String')
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($Path)
    try {
        $key.SetValue($Name, $Value, [Microsoft.Win32.RegistryValueKind] $Kind)
    } finally {
        $key.Close()
    }
}

# --- the executable ---------------------------------------------------------

# Cargo is asked where its target directory is rather than guessed at, because
# `[build] target-dir` in a Cargo configuration file moves it and no
# environment variable then says so. The Linux script learned this the hard way
# and this one inherits the lesson rather than repeating it.
function Find-Binary {
    $targetDir = $null
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Push-Location (Join-Path $here '..\..')
        try {
            $meta = cargo metadata --format-version 1 --no-deps 2>$null | ConvertFrom-Json
            if ($meta) { $targetDir = $meta.target_directory }
        } catch { }
        finally { Pop-Location }
    }
    if (-not $targetDir) { $targetDir = Join-Path $here '..\..\target' }

    foreach ($built in 'release', 'debug') {
        $candidate = Join-Path $targetDir "$built\$exeName"
        if (Test-Path -LiteralPath $candidate) { return (Resolve-Path -LiteralPath $candidate).Path }
    }
    return $null
}

$foundBinary = $null
if ($NoBinary) {
    # Nothing to find.
} elseif ($Binary) {
    if (-not (Test-Path -LiteralPath $Binary)) { throw "install.ps1: $Binary is not there" }
    $foundBinary = (Resolve-Path -LiteralPath $Binary).Path
} else {
    $foundBinary = Find-Binary
}

# --- the files --------------------------------------------------------------

New-Item -ItemType Directory -Force -Path $Prefix | Out-Null

$iconSource = Join-Path $here $iconName
if (-not (Test-Path -LiteralPath $iconSource)) {
    throw "install.ps1: $iconName is not beside this script; run make-ico first"
}
$installedIcon = Join-Path $Prefix $iconName
Copy-Item -LiteralPath $iconSource -Destination $installedIcon -Force

# The uninstaller is copied in rather than run from the repository, because the
# Add/Remove Programs entry below points at it and a checkout is not something
# that has to still be there a year later.
Copy-Item -LiteralPath (Join-Path $here 'uninstall.ps1') `
          -Destination (Join-Path $Prefix 'uninstall.ps1') -Force

$installedExe = Join-Path $Prefix $exeName
if ($foundBinary) {
    Copy-Item -LiteralPath $foundBinary -Destination $installedExe -Force
    Write-Output "installed $installedExe from $foundBinary"
} elseif (-not (Test-Path -LiteralPath $installedExe)) {
    Write-Warning "no executable installed; the association will point at $installedExe, which is not there yet"
}

# --- the registry -----------------------------------------------------------

$classes = 'Software\Classes'

# The type itself. `FriendlyTypeName` is what Explorer's Type column shows and
# it is written as a plain string: the usual form is a reference into a
# binary's resource table, which needs SHLoadIndirectString to read back, and
# this application's own type query refuses those rather than show a person one.
Set-RegistryValue "$classes\$progId" '' $typeName
Set-RegistryValue "$classes\$progId" 'FriendlyTypeName' $typeName
Set-RegistryValue "$classes\$progId\DefaultIcon" '' "$installedIcon,0"
Set-RegistryValue "$classes\$progId\shell\open\command" '' "`"$installedExe`" `"%1`""

# The application behind the type. `ApplicationName` is the first place the
# shell looks for a name a person recognises, and the first place this
# application's own type query looks: installing this is what makes a slipcase
# report that it opens with Slipcase.
Set-RegistryValue "$classes\$progId\Application" 'ApplicationName' $appName
Set-RegistryValue "$classes\$progId\Application" 'ApplicationCompany' 'Excelano'
Set-RegistryValue "$classes\$progId\Application" 'ApplicationDescription' `
    'Open a slipcase container and see what is in it'
Set-RegistryValue "$classes\$progId\Application" 'ApplicationIcon' "$installedIcon,0"

# The extension, and the media type SPEC 4 names. `Content Type` here and the
# MIME database entry below are the two halves of the same statement, and
# Windows uses each in a different direction: name to type, and type to name.
Set-RegistryValue "$classes\$extension" '' $progId
Set-RegistryValue "$classes\$extension" 'Content Type' $contentType
Set-RegistryValue "$classes\$extension\OpenWithProgids" $progId ''
Set-RegistryValue "$classes\MIME\Database\Content Type\$contentType" 'Extension' $extension

# The Open With list, so a person can reach this application from a file it was
# not registered for, and so the shell has a name for the executable itself.
$applications = "$classes\Applications\$exeName"
Set-RegistryValue $applications 'FriendlyAppName' $appName
Set-RegistryValue "$applications\shell\open\command" '' "`"$installedExe`" `"%1`""
Set-RegistryValue "$applications\SupportedTypes" $extension ''

# --- the Start menu ---------------------------------------------------------

# The counterpart of the `.desktop` entry: what puts the application in front of
# a person who has not got a container to double-click yet. The shortcut carries
# the icon, which matters more here than on Linux — see the note about the
# window icon in README.md.
$startMenu = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs'
$shortcut = Join-Path $startMenu 'Slipcase.lnk'
if (Test-Path -LiteralPath $installedExe) {
    $shell = New-Object -ComObject WScript.Shell
    $link = $shell.CreateShortcut($shortcut)
    $link.TargetPath = $installedExe
    $link.WorkingDirectory = $Prefix
    $link.IconLocation = "$installedIcon,0"
    $link.Description = 'Open a slipcase container and see what is in it'
    $link.Save()
    # Deliberately no AppUserModelID on this shortcut. Setting one would need
    # the running process to declare the same identity through
    # SetCurrentProcessExplicitAppUserModelID, which is a raw call this
    # application cannot make under `#![forbid(unsafe_code)]`. With neither
    # side declaring one, Windows derives both from the executable path, they
    # agree, and pinning and taskbar grouping work. README.md has the whole of
    # it.
}

# --- Add/Remove Programs ----------------------------------------------------

$version = '0.1.0'
$cargoToml = Join-Path $here '..\..\Cargo.toml'
if (Test-Path -LiteralPath $cargoToml) {
    $line = Select-String -LiteralPath $cargoToml -Pattern '^version = "([^"]+)"' | Select-Object -First 1
    if ($line) { $version = $line.Matches[0].Groups[1].Value }
}

$uninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Slipcase'
$uninstallCommand = "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$(Join-Path $Prefix 'uninstall.ps1')`""
Set-RegistryValue $uninstallKey 'DisplayName' 'Slipcase'
Set-RegistryValue $uninstallKey 'DisplayVersion' $version
Set-RegistryValue $uninstallKey 'Publisher' 'Excelano'
Set-RegistryValue $uninstallKey 'DisplayIcon' "$installedIcon,0"
Set-RegistryValue $uninstallKey 'InstallLocation' $Prefix
Set-RegistryValue $uninstallKey 'UninstallString' $uninstallCommand
Set-RegistryValue $uninstallKey 'QuietUninstallString' $uninstallCommand
Set-RegistryValue $uninstallKey 'NoModify' 1 'DWord'
Set-RegistryValue $uninstallKey 'NoRepair' 1 'DWord'

# --- tell the shell ---------------------------------------------------------

# Without this the icon and the type description appear at the next logon
# rather than now, which reads as the association not having worked.
Add-Type -Namespace Slipcase -Name Shell -MemberDefinition @'
[DllImport("shell32.dll", CharSet=CharSet.Unicode)]
public static extern void SHChangeNotify(int eventId, uint flags, System.IntPtr item1, System.IntPtr item2);
'@
[Slipcase.Shell]::SHChangeNotify(0x08000000, 0, [System.IntPtr]::Zero, [System.IntPtr]::Zero)

Write-Output ""
Write-Output "registered $extension as $contentType, opened by $progId, under $Prefix"
Write-Output ""
Write-Output "check it with:"
# Not `assoc` and `ftype`. Both report this extension as having no association
# at all after a successful install: they read and write the machine-wide half
# of the class root only, and everything above is per-user. Measured, after
# they were put here first and printed exactly the message a failed install
# would have.
Write-Output "  reg query `"HKCU\Software\Classes$extension`" /s"
Write-Output "  cargo run --example opens-with -- some$extension      # Slipcase"
Write-Output "and by double-clicking a $extension, which should open it rather than an empty window."
