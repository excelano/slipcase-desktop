# Windows packaging

`DESIGN.md` §8. The extension and the media type registered by the installer,
on the platform that has no freedesktop database to put them in.

    powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1
    powershell -ExecutionPolicy Bypass -File packaging\windows\uninstall.ps1

`install.ps1 -NoBinary` registers the association without copying an executable,
and `-Prefix DIR` puts the files somewhere else. `uninstall.ps1 -KeepFiles`
removes the association and leaves them.

**If you install this way and later install Slipcase from the Microsoft Store,
run `uninstall.ps1` first.** With both registered Windows chooses neither and
puts up its *how do you want to open this file* picker, so a package installed
over a live script installation turns a working association into a prompt. Worse
if the files are deleted by hand instead: a `UserChoice` left pointing at a
`ProgID` whose executable is gone kills the extension outright — *Application not
found*, the package ignored, and no picker offering a way out. `uninstall.ps1`
removes that key, which is the whole reason it is the thing to run.

Neither state is repairable from inside a package: an MSIX runs no code at
install time, and one running later cannot write the key back, because a
package's registry writes are virtualized.

## What is here

| File | What it is |
| --- | --- |
| `install.ps1` | Writes the registry keys, copies the files, makes the Start menu shortcut |
| `uninstall.ps1` | Removes all of it. Copied into the install directory, because Add/Remove Programs points at it and a checkout may be gone |
| `slipcase.ico` | The icon, nine sizes. Built from the Linux SVG, not drawn separately |
| `assets/` | The five PNGs `AppxManifest.xml` names, from the same SVG. Committed for the same reason the `.ico` is |
| `make-ico/` | The tool that builds both |
| `AppxManifest.xml.in` | The MSIX manifest, with the identity and the version left as placeholders |
| `identity.psd1` | What Partner Center assigned when the name was reserved. The one place those values live |
| `build-msix.ps1` | Builds the package from a release binary, and optionally signs it and runs the certification kit |

## Two scripts rather than an installer

The lightest thing that registers an extension properly and uninstalls cleanly,
which is what the application needs: one executable, one icon, and no runtime
files of its own. MSI through WiX, Inno Setup, and NSIS are all
rejected for the same reason — each needs a toolchain that is not on a
stock Windows and not in this repository's build, to produce a package that
would do what forty lines of registry writes do. `packaging/linux` is a pair of
shell scripts for the same reason, and these two are its counterpart, argument
for argument.

**The channel is the Microsoft Store**, taken for the reason macOS took the Mac
App Store: a person who has been sent a container double-clicks it, Windows
offers to search the Store by file type, and outside the Store that search
returns nothing. The Store takes MSIX and WiX builds MSI, so the channel does
not revive WiX. The two scripts stay as well: they are the per-user,
no-toolchain, no-account route, and a Store listing is not a reason to take that
away from somebody who would rather not have one.

**MSIX changes nothing about how this application reads the registry, hands
files to the shell, or registers a file type**, measured against a signed
package built from a release binary and installed. `opens_with` gets the same
answers inside the container as outside, because MSIX virtualizes what a
package writes and not what it reads. `opener::open` reaches the shell from
inside and the handler starts. And the association declared in
`AppxManifest.xml` works on its own: a double-clicked container launches the
packaged binary with no registry key written by anybody.

What is not free is the overlap. With the package and these scripts both
registered, Windows chooses neither and puts up its *how do you want to open
this file* picker, so installing the package over a live script installation
turns a working association into a prompt. Whether a `UserChoice` a person
actually chose then outranks the manifest is unmeasured: that key is
hash-validated and write-denied, so it cannot be forged and a person has to
make the choice.

**What a stale `UserChoice` does depends on what it points at**, measured by
hand with the choice made through Explorer:

| `UserChoice` names | What a double-click does |
| --- | --- |
| A ProgID that exists, whose command names a deleted executable | **Refused: *Application not found***. The package is ignored and no picker appears |
| A ProgID that no longer exists at all | The package wins and launches from `WindowsApps` |
| Nothing | The package wins |

**The first row is the trap**, and an MSIX cannot clear it: a package runs no code at install time, so it cannot remove a
`UserChoice`. Running `uninstall.ps1` before installing the package is the
remedy, which is why the top of this file says so.

In any check of a packaged install, `AssocQueryString` answers `ERROR_NO_APPLICATION_ASSOCIATED` for the executable and the command
line of a packaged handler while still returning its friendly names, because
there is no command line — activation goes through the app model. That is the
same trap as `assoc` and `ftype` below, and it means a script that verifies an
install by looking for an executable path will report a correct package as no
association at all.

Getting a package installed at all costs exactly one administrator action:
`makeappx`, `New-SelfSignedCertificate`, `signtool` and `Add-AppxPackage` all
run as an ordinary user, but the signing certificate has to reach
`LocalMachine\TrustedPeople`, and the per-user store is not read for this —
importing there leaves deployment failing `0x800B0109` just the same. The
Developer Mode route is the other way in and needs the same elevation.

**Per-user, under `HKCU` and `%LOCALAPPDATA%`**, which is the counterpart of the
Linux script's default of `~/.local`. There is no all-users variant: the
machine-wide half of every key here needs elevation, and a script that
sometimes needs administrator and sometimes does not is worse than one that
never does.

## What gets written

Everything under `HKEY_CURRENT_USER`.

| Key | Value | Why |
| --- | --- | --- |
| `Software\Classes\.slpc` | `Excelano.Slipcase` | The extension names the type |
| `Software\Classes\.slpc` → `Content Type` | `application/vnd.excelano.slipcase+zip` | SPEC §4's media type, name to type |
| `Software\Classes\MIME\Database\Content Type\application/vnd.excelano.slipcase+zip` → `Extension` | `.slpc` | The same statement, type to name |
| `Software\Classes\Excelano.Slipcase` | `Slipcase Container` | What Explorer's Type column shows |
| `…\DefaultIcon` | `slipcase.ico,0` | What Explorer draws |
| `…\shell\open\command` | `"…\slipcase-desktop.exe" "%1"` | What a double-click runs |
| `…\Application` → `ApplicationName` | `Slipcase` | The name a person recognizes |
| `Software\Classes\Applications\slipcase-desktop.exe` | `FriendlyAppName` | The Open With list |
| `Software\Microsoft\Windows\CurrentVersion\Uninstall\Slipcase` | — | Add/Remove Programs |

`Excelano.Slipcase` is chosen here; `.slpc` and
`application/vnd.excelano.slipcase+zip` are not, and come from `SPEC.md` §4,
which is the authority. SPEC §4 reserves no magic bytes, so the extension is
the only identification Windows has: there is no content type to sniff and no
`sub-class-of` to fall back on the way shared-mime-info has one.

**The superseded name is removed rather than kept.** `install.ps1` writes the
registered name and deletes the old `MIME\Database` key, which an upgrade would
otherwise leave behind mapping `.slpc` to a type this application no longer
claims. `uninstall.ps1` clears both names, because it has to answer for what any
version of `install.ps1` wrote. `DESIGN.md` §8 holds the decision and the
argument behind it.

`FriendlyTypeName` is written as a plain string. The usual form is a reference
into a binary's resource table — `@C:\path\thing.dll,-123` — which needs
`SHLoadIndirectString` to read back, and this application's own type query
refuses those rather than show a person the reference. Registering one would
have meant `src/opens_with.rs` could not read what the installer wrote.

## Measured rather than assumed

**Windows PowerShell 5.1 reads a script with no byte order mark as ANSI, and an
em dash in a string literal is then a syntax error.** The UTF-8 bytes of `—`
decode under Windows-1252 to `â€"`, and that last character is `U+201D`, which
PowerShell accepts as a *string delimiter* — so the string ends in the middle of
a sentence and the parser reports a missing terminator far below it.

So every string a script prints is ASCII: these messages go to a console whose
code page is nobody's to predict, and an em dash there is mojibake even when it
parses. Comments may carry one, because a comment is never parsed as a string.
No script here carries a byte order mark, and one file that differed would be a
trap of its own.

**`assoc` and `ftype` do not see any of this.** They report `.slpc` as having no
association at all after a successful install, because they read and write the
machine-wide half of the class root only, and print the message a failed
install would. `reg query HKCU\Software\Classes\.slpc /s` is the check that works,
and `cargo run --example opens-with -- some.slpc` is the better one: it answers
`Slipcase`, which is this application's own type query reading the registration
this directory just made.

**PowerShell's registry provider cannot write the media type key.** The name is
`application/vnd.excelano.slipcase+zip`, and the provider reads the forward
slash as a path separator: it creates `application` with a child
`vnd.excelano.slipcase+zip` and reports success. Both scripts use
`[Microsoft.Win32.Registry]` instead, which takes the whole string as one name.

**A stale `UserChoice` is the dead association to worry about.** Choosing
"always open with" writes `Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.slpc\UserChoice`,
and it outranks every key in the table above. Removing the class keys and
leaving that one behind leaves the extension pointing at a ProgID that no
longer exists — and Windows does not fall back to the machine-wide association
then, it treats the extension as having none. `uninstall.ps1` removes it.

## The icon

`slipcase.ico` is built from `packaging/linux/icons/slipcase-desktop.svg`, which
is the source for every platform's icon and is not duplicated here:

    cd packaging/windows/make-ico && cargo run --release

Nine sizes — 16, 20, 24, 32, 40, 48, 64, 128, 256. The three the shell asks for
are 16, 32, and 48; the rest are those again at the display scalings Windows
offers, plus 256 for the extra-large view. Entries above 48 are stored as PNG
and the rest as bitmaps, which is the convention and saves a quarter of a
megabyte on the 256 alone.

`make-ico` is its own package rather than a member of the application's, so
nothing it depends on reaches the shipped binary. It renders with `resvg` and
assembles with `ico`, both pure Rust; `cargo tree -i cc` finds nothing in it
either.

`packaging/README.md` asks that any change to the drawing be checked at small
sizes. At 16 the outline goes gray and the card's rounded corners disappear, and
the silhouette still reads.

## The window's own icon, and the taskbar

**Neither egui, eframe, nor winit sets an AppUserModelID.** `APP_ID` in
`src/main.rs` is `with_app_id`, which is Wayland's `xdg_toplevel.set_app_id`
and does nothing on Windows.

That is left alone deliberately, and the Start menu shortcut carries no
AppUserModelID either. Setting one on the shortcut without the process
declaring the same identity through `SetCurrentProcessExplicitAppUserModelID`
would break the pairing rather than fix it — and that call is raw FFI, which
`#![forbid(unsafe_code)]` puts out of reach. With neither side declaring one,
Windows derives both from the executable's path, they agree, and pinning and
taskbar grouping work.

**The window icon is embedded as bytes, not compiled into a resource.** Windows
takes a window's icon from a resource in the executable, and building one needs
`rc.exe` or `windres` — a build step `DESIGN.md` §2 keeps out. So `main.rs`
carries `slipcase.ico` through `include_bytes!` and hands the 64-pixel entry to
the window at startup: 64 is a whole multiple of the sizes a display at 100% or
200% asks for — 16 and 32 in the title bar, 32 and 64 in the task bar — so each
of those is an integer downsample rather than a resample of a resample.

**Not of every size.** 125% asks for 20 and 40 and 150% for 24 and 48, and 64
divides none of them, so those are resampled. Both read cleanly, and 64 is the
largest entry no scaling has to enlarge.

This is why `slipcase.ico` is a committed artifact in a repository that
otherwise holds only sources: the executable references it at compile time, and
Windows has no step that would rasterize the SVG for either purpose.

## What a Store build is

`build-msix.ps1` produces it and the release apps in excelano/shipping submit
it. The reasons for MSIX and for keeping the two scripts are above.

**Signing stops being optional.** The shell will not accept an unsigned MSIX, so
unlike macOS there is no unsigned build-and-test loop here. The Store signs what
it distributes, so the package that goes up is unsigned and the throwaway-signed
copy is only for installing locally — and the two must come from one staging
tree with no rebuild between, because a rebuild of identical source produces a
different file.

**The manifest declares `runFullTrust` and nothing else.** A capability asked for
and unused is a question at certification with no good answer, and the
justification field caps at 500 characters and truncates silently at the paste.
