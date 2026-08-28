# Assemble the MSIX package the Microsoft Store distributes: the release
# executable, the manifest with the identity and the version substituted into
# it, and the five images the manifest names.
#
# `packaging/macos/build-app.sh` is the model, and the rule it works to is the
# one that matters here: refuse loudly rather than produce something subtly
# wrong. A bundle quietly missing an architecture, or a package quietly built
# from a debug binary, says nothing at the moment it is made and everything
# days later. Every check below exists because the thing it checks cannot be
# seen by looking at the finished package.
#
#   powershell -ExecutionPolicy Bypass -File packaging\windows\build-msix.ps1
#   ...\build-msix.ps1 -SelfSign            # sign it, so it can be installed here
#   ...\build-msix.ps1 -SelfSign -Certify   # and run the certification kit
#
# WHAT THIS DOES NOT DO
#
# It does not submit, and it does not produce a signature that goes anywhere
# near a submission. The Store signs what it distributes, so `-SelfSign` exists
# only so that a package can be installed on this machine and looked at. The
# certificate it makes is a throwaway, and `README.md` beside this file records
# that the measurement one before it was too.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)

[CmdletBinding()]
param(
    # The executable to package. With neither this nor a release build present
    # this refuses rather than building one: `cargo build --release` is the
    # caller's to run, the way it is for every other packaging script here.
    [string] $Binary,
    # Where to write the package and its staging tree. Defaulted in the body
    # rather than here: $PSScriptRoot is empty while parameters are being bound
    # in Windows PowerShell 5.1, so a default built from it is a refusal before
    # the script has run a line. `install.ps1` reads its own path in the body
    # for the same reason.
    [string] $OutDir,
    # Sign with a throwaway certificate whose subject is the manifest's
    # Publisher, so that the package can be installed here. Not for submission.
    [switch] $SelfSign,
    # Run the Windows App Certification Kit and fail on it. Needs elevation, and
    # needs the package to be installable, so it needs -SelfSign as well.
    [switch] $Certify
)

$ErrorActionPreference = 'Stop'
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Split-Path -Parent (Split-Path -Parent $here)
if (-not $OutDir) { $OutDir = Join-Path $root 'dist' }

function Refuse([string] $message) {
    Write-Error "build-msix.ps1: $message"
}

# --- the identity, from one place -------------------------------------------

# `identity.psd1` holds what Partner Center assigned. The manifest keeps its
# placeholders, so that nothing has to be edited per build and so that the one
# file a person might mistype lives beside a comment saying where its values
# came from.
#
# It is not committed — `.gitignore` says why — so a fresh checkout does not
# have one, and the refusal names the template rather than just the missing
# path. A build script whose first failure is "no such file" teaches nothing.
$identityFile = Join-Path $here 'identity.psd1'
if (-not (Test-Path $identityFile)) {
    Refuse "no identity at $identityFile - copy identity.psd1.example beside it and fill in what Partner Center shows under Product management, Product identity"
}
$identity = Import-PowerShellDataFile $identityFile
foreach ($field in 'Name', 'Publisher', 'PublisherDisplayName') {
    if (-not $identity.$field) { Refuse "identity.psd1 has no $field" }
}
# The one value with a shape worth checking. `Publisher` is an X.500 string and
# the display name is what gets put there by mistake; a package whose Publisher
# does not match the reservation is rejected at upload, which is the most
# expensive place to find out.
if ($identity.Publisher -notmatch '^CN=') {
    Refuse "identity.psd1's Publisher is '$($identity.Publisher)', which is not an X.500 string - Partner Center's Package/Identity/Publisher begins CN="
}

# --- the version, from the one parser ---------------------------------------

# `packaging/version.sh` is the only thing that reads Cargo.toml's version, and
# it is asked here rather than copied, which is the whole reason it takes an
# argument. It is POSIX sh, so it needs a shell, and Git for Windows ships one.
#
# Not `bash` off PATH. On a machine with WSL that name resolves to
# C:\Windows\System32\bash.exe, which runs inside a Linux distribution where
# this checkout is at a different path, so version.sh would read a Cargo.toml
# that is not this one — or nothing at all. Measured on this machine, where
# PATH resolves `bash` to exactly that and `sh` to nothing.
$git = Get-Command git -ErrorAction SilentlyContinue
if (-not $git) { Refuse 'git is not on PATH, and version.sh needs the shell Git for Windows ships' }
$gitRoot = Split-Path -Parent (Split-Path -Parent $git.Source)
$sh = Join-Path $gitRoot 'bin\bash.exe'
if (-not (Test-Path $sh)) { $sh = Join-Path $gitRoot 'usr\bin\sh.exe' }
if (-not (Test-Path $sh)) {
    Refuse "no shell found beside $($git.Source) - version.sh is POSIX sh and needs the one Git for Windows installs"
}
$versionScript = (Join-Path $here '..\version.sh').Replace('\', '/')
$version = & $sh $versionScript --appx
if ($LASTEXITCODE -ne 0 -or -not $version) {
    Refuse 'version.sh --appx would not answer'
}
$version = ($version | Select-Object -First 1).Trim()
# The Store requires four parts with the fourth 0, and version.sh says so too.
# This is the check that shelling out produced what was asked for rather than a
# message on standard output.
if ($version -notmatch '^\d+\.\d+\.\d+\.0$') {
    Refuse "version.sh --appx said '$version', which is not four parts ending in 0"
}

# --- the executable ---------------------------------------------------------

# Cargo is asked where its target directory is rather than guessed at, because
# `[build] target-dir` in a Cargo configuration file moves it and no
# environment variable then says so. Every packaging script here asks.
if (-not $Binary) {
    $targetDir = $null
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Push-Location $root
        try {
            $meta = cargo metadata --format-version 1 --no-deps 2>$null | ConvertFrom-Json
            if ($meta) { $targetDir = $meta.target_directory }
        } finally { Pop-Location }
    }
    if (-not $targetDir) { $targetDir = Join-Path $root 'target' }
    $Binary = Join-Path $targetDir 'release\slipcase-desktop.exe'
}
if (-not (Test-Path $Binary)) {
    Refuse "no executable at $Binary - run 'cargo build --release' first"
}
$Binary = (Resolve-Path $Binary).Path

# Two things read straight out of the PE header, because neither is visible in
# a finished package and both are shipping defects.
#
# The architecture, because the manifest declares x64, and a package whose
# declaration disagrees with its executable installs and then fails to launch.
#
# The subsystem, because `src/main.rs` carries `windows_subsystem = "windows"`
# only when `debug_assertions` is off — so a debug binary packaged by mistake is
# a console subsystem one, and a console window behind the application is a
# defect this project has already found by hand once. It is the cheapest check
# in this file and it guards the one thing here that cost an eye to notice.
$pe = [System.IO.File]::ReadAllBytes($Binary)
$peOffset = [BitConverter]::ToInt32($pe, 0x3C)
$machine = [BitConverter]::ToUInt16($pe, $peOffset + 4)
$subsystem = [BitConverter]::ToUInt16($pe, $peOffset + 92)
if ($machine -ne 0x8664) {
    Refuse ("$Binary is machine 0x{0:X4}, and AppxManifest declares x64" -f $machine)
}
if ($subsystem -ne 2) {
    Refuse "$Binary is not a Windows GUI subsystem executable (subsystem $subsystem) - a debug build is a console one, and packaging that puts a console window behind the application"
}

# --- the tools --------------------------------------------------------------

# Neither is on PATH on a stock machine and both are stock in the SDK. The
# newest SDK is taken, and the x64 build of the tool because that is the
# architecture everything else here is.
function Find-SdkTool([string] $name) {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (-not (Test-Path $kits)) { return $null }
    Get-ChildItem $kits -Directory |
        Where-Object { $_.Name -match '^10\.' } |
        Sort-Object { [version] $_.Name } -Descending |
        ForEach-Object { Join-Path $_.FullName "x64\$name" } |
        Where-Object { Test-Path $_ } |
        Select-Object -First 1
}
$makeappx = Find-SdkTool 'makeappx.exe'
if (-not $makeappx) { Refuse 'no makeappx.exe in any Windows SDK - install the Windows 10/11 SDK' }

# --- the staging tree -------------------------------------------------------

if (-not (Test-Path $OutDir)) { New-Item -ItemType Directory -Path $OutDir -Force | Out-Null }
$OutDir = (Resolve-Path $OutDir).Path
$stage = Join-Path $OutDir 'msix-stage'
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Path (Join-Path $stage 'Assets') -Force | Out-Null

Copy-Item $Binary (Join-Path $stage 'slipcase-desktop.exe')

# The five images the manifest names. They are committed artifacts built by
# `make-ico` from the same drawing every platform's icon comes from, and they
# are checked here rather than trusted: a manifest naming an image that is not
# there is a makeappx failure with a worse message than this one.
$assets = Join-Path $here 'assets'
foreach ($image in 'StoreLogo.png', 'Square150x150Logo.png', 'Square44x44Logo.png',
                   'Wide310x150Logo.png', 'slipcase.png') {
    $from = Join-Path $assets $image
    if (-not (Test-Path $from)) {
        Refuse "no $image in $assets - run 'cargo run --release' in packaging/windows/make-ico"
    }
    Copy-Item $from (Join-Path $stage "Assets\$image")
}

# --- the manifest -----------------------------------------------------------

$manifest = Get-Content (Join-Path $here 'AppxManifest.xml.in') -Raw
$manifest = $manifest.
    Replace('@IDENTITY_NAME@', $identity.Name).
    Replace('@PUBLISHER@', $identity.Publisher).
    Replace('@PUBLISHER_DISPLAY_NAME@', $identity.PublisherDisplayName).
    Replace('@VERSION_APPX@', $version)

# A placeholder that survived substitution is a package that installs and is
# wrong, so it is looked for rather than assumed away. This catches a
# placeholder added to the template and not to this script, which is the
# realistic way the two part company.
$left = [regex]::Matches($manifest, '@[A-Z_]+@') |
    ForEach-Object { $_.Value } | Sort-Object -Unique
if ($left) {
    Refuse "AppxManifest.xml.in has placeholders this script does not substitute: $($left -join ', ')"
}

# UTF-8 with no byte order mark. `Out-File -Encoding utf8` in Windows
# PowerShell 5.1 writes one, and a manifest beginning with a BOM is malformed
# XML as far as makeappx is concerned.
[System.IO.File]::WriteAllText(
    (Join-Path $stage 'AppxManifest.xml'),
    $manifest,
    (New-Object System.Text.UTF8Encoding $false))

# --- the package ------------------------------------------------------------

$package = Join-Path $OutDir "Slipcase-$version-x64.msix"
& $makeappx pack /d $stage /p $package /o
if ($LASTEXITCODE -ne 0) { Refuse "makeappx pack failed ($LASTEXITCODE)" }

Write-Host ''
Write-Host "built $package"
Write-Host "  identity  $($identity.Name)"
Write-Host "  publisher $($identity.Publisher)"
Write-Host "  version   $version"
Write-Host "  from      $Binary"

# --- signing, for a local install and nothing else --------------------------

if ($SelfSign) {
    $signtool = Find-SdkTool 'signtool.exe'
    if (-not $signtool) { Refuse 'no signtool.exe in any Windows SDK' }

    # signtool refuses a package whose manifest Publisher and whose certificate
    # subject differ, so the subject is built from the identity rather than
    # typed a second time. The certificate left in LocalMachine\TrustedPeople by
    # the 2026-08-26 measurement carries a different subject and cannot sign
    # this one: that package predates the reservation and had an invented
    # identity.
    $cert = Get-ChildItem Cert:\CurrentUser\My |
        Where-Object { $_.Subject -eq $identity.Publisher -and $_.HasPrivateKey } |
        Sort-Object NotAfter -Descending |
        Select-Object -First 1
    if (-not $cert) {
        Write-Host "making a throwaway signing certificate for $($identity.Publisher)"
        $cert = New-SelfSignedCertificate -Type CodeSigningCert `
            -Subject $identity.Publisher `
            -KeyUsage DigitalSignature `
            -FriendlyName 'Slipcase MSIX test signing (throwaway)' `
            -CertStoreLocation Cert:\CurrentUser\My `
            -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3')
    }
    & $signtool sign /fd SHA256 /sha1 $cert.Thumbprint $package
    if ($LASTEXITCODE -ne 0) { Refuse "signtool failed ($LASTEXITCODE)" }
    Write-Host "signed with $($cert.Thumbprint) - a throwaway, and not what the Store distributes"

    # Deployment reads LocalMachine\TrustedPeople and not the per-user store:
    # importing into CurrentUser\TrustedPeople leaves Add-AppxPackage failing
    # 0x800B0109 just the same. Measured on 2026-08-26 and recorded in
    # CHECKLIST.md. That import is the one administrator action in this whole
    # path, so it is printed rather than attempted.
    $trusted = Get-ChildItem Cert:\LocalMachine\TrustedPeople -ErrorAction SilentlyContinue |
        Where-Object { $_.Thumbprint -eq $cert.Thumbprint }
    Write-Host ''
    if ($trusted) {
        Write-Host 'install it:'
        Write-Host "  Add-AppxPackage $package"
    } else {
        Write-Host 'to install it, this certificate has to be trusted, which needs administrator once:'
        Write-Host "  Export-Certificate -Cert Cert:\CurrentUser\My\$($cert.Thumbprint) -FilePath `$env:TEMP\slipcase-test.cer"
        Write-Host '  # then, from an elevated prompt:'
        Write-Host "  Import-Certificate -FilePath `$env:TEMP\slipcase-test.cer -CertStoreLocation Cert:\LocalMachine\TrustedPeople"
        Write-Host "  Add-AppxPackage $package"
    }
}

# --- the certification kit --------------------------------------------------

if ($Certify) {
    if (-not $SelfSign) {
        Refuse '-Certify needs -SelfSign: the kit installs the package it tests, and an unsigned one will not install'
    }
    $elevated = ([Security.Principal.WindowsPrincipal] `
            [Security.Principal.WindowsIdentity]::GetCurrent()
        ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $elevated) {
        Refuse 'the Windows App Certification Kit needs an elevated session - rerun this from an administrator prompt'
    }
    $appcert = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\App Certification Kit\appcert.exe'
    if (-not (Test-Path $appcert)) {
        Refuse "no appcert.exe at $appcert - the App Certification Kit is a separate SDK feature"
    }

    $report = Join-Path $OutDir "wack-$version.xml"
    & $appcert reset | Out-Null
    & $appcert test -appxpackagepath $package -reportoutputpath $report
    if (-not (Test-Path $report)) {
        Refuse "the certification kit wrote no report to $report"
    }

    # The verdict is read out of the report rather than out of an exit code. A
    # kit that ran and failed and a kit that never ran are different things, and
    # this must never call the second one a pass: a missing verdict is a refusal
    # too.
    [xml] $xml = Get-Content $report
    $overall = $xml.REPORT.OVERALL_RESULT
    if (-not $overall) {
        Refuse "the certification kit's report at $report has no OVERALL_RESULT - read it rather than trusting this script"
    }
    foreach ($failed in $xml.SelectNodes('//*[@OVERALL_RESULT="FAIL"]')) {
        $title = $failed.GetAttribute('TITLE')
        if (-not $title) { $title = $failed.Name }
        Write-Host "FAIL  $title"
    }
    Write-Host "certification kit: $overall  ($report)"
    if ($overall -ne 'PASS') {
        Refuse "the Windows App Certification Kit says $overall - certification runs it too, so this comes back"
    }
}
