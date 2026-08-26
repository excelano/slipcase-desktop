# Windows packaging

`DESIGN.md` §8. The extension and the media type registered by the installer,
on the platform that has no freedesktop database to put them in.

    powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1
    powershell -ExecutionPolicy Bypass -File packaging\windows\uninstall.ps1

`install.ps1 -NoBinary` registers the association without copying an executable,
and `-Prefix DIR` puts the files somewhere else. `uninstall.ps1 -KeepFiles`
removes the association and leaves them.

## What is here

| File | What it is |
| --- | --- |
| `install.ps1` | Writes the registry keys, copies the files, makes the Start menu shortcut |
| `uninstall.ps1` | Removes all of it. Copied into the install directory, because Add/Remove Programs points at it and a checkout may be gone |
| `slipcase.ico` | The icon, nine sizes. Built from the Linux SVG, not drawn separately |
| `make-ico/` | The tool that builds it |

## Two scripts rather than an installer

The lightest thing that registers an extension properly and uninstalls cleanly,
which is what the application needs: one executable, one icon, and no runtime
files of its own. MSI through WiX, Inno Setup, and NSIS were all considered and
all rejected for the same reason — each needs a toolchain that is not on a
stock Windows and not in this repository's build, to produce a package that
would do what forty lines of registry writes do. `packaging/linux` is a pair of
shell scripts for the same reason, and these two are its counterpart, argument
for argument.

An MSI becomes worth building when there is a channel to ship it through, the
way `packaging/debian` exists because the Excelano apt repository does. There
is no such channel for Windows yet, and building a package with nowhere to send
it would be guessing at what that channel will want.

**Amended: there is a channel now, and it is the Microsoft Store.** The
paragraph above is left standing because its reasoning was right and it is what
chose the two scripts; what changed is the premise. The Store was taken for the
reason macOS took the Mac App Store: a person who has been sent a container
double-clicks it, Windows offers to search the Store by file type, and outside
the Store that search returns nothing. `packaging/macos/README.md` has the same
argument written out at length.

That decides the format against MSI rather than for it. The Store takes MSIX,
WiX builds MSI, and the two are not steps on one path — so WiX stays rejected,
by the paragraph above and now also by the channel. The two scripts stay as
well. They are the per-user, no-toolchain, no-account route, and a Store
listing is not a reason to take that away from somebody who would rather not
have one.

Three things have to be measured before any of it is built, and none of them is
paperwork. macOS is the reason to say that plainly: the App Sandbox was assumed
to be a formality there, and it turned out to need a new module, a rewritten
save path, and a reopened section of `DESIGN.md`. MSIX is a container too. The
questions are in `CHECKLIST.md` under Windows; the short version is that this
application reads the registry to answer what would open a payload, hands files
to the shell, and registers a file type — and MSIX has its own opinion about
all three.

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
| `Software\Classes\.slpc` → `Content Type` | `application/x.slipcase+zip` | SPEC §4's media type, name to type |
| `Software\Classes\MIME\Database\Content Type\application/x.slipcase+zip` → `Extension` | `.slpc` | The same statement, type to name |
| `Software\Classes\Excelano.Slipcase` | `Slipcase Container` | What Explorer's Type column shows |
| `…\DefaultIcon` | `slipcase.ico,0` | What Explorer draws |
| `…\shell\open\command` | `"…\slipcase-desktop.exe" "%1"` | What a double-click runs |
| `…\Application` → `ApplicationName` | `Slipcase` | The name a person recognises |
| `Software\Classes\Applications\slipcase-desktop.exe` | `FriendlyAppName` | The Open With list |
| `Software\Microsoft\Windows\CurrentVersion\Uninstall\Slipcase` | — | Add/Remove Programs |

`Excelano.Slipcase` is chosen here; `.slpc` and `application/x.slipcase+zip`
are not, and come from `SPEC.md` §4, which is the authority. SPEC §4 reserves
no magic bytes, so the extension is the only identification Windows has: there
is no content type to sniff and no `sub-class-of` to fall back on the way
shared-mime-info has one.

`FriendlyTypeName` is written as a plain string. The usual form is a reference
into a binary's resource table — `@C:\path\thing.dll,-123` — which needs
`SHLoadIndirectString` to read back, and this application's own type query
refuses those rather than show a person the reference. Registering one would
have meant `src/opens_with.rs` could not read what the installer wrote.

## Three things that were measured rather than assumed

**`assoc` and `ftype` do not see any of this.** They report `.slpc` as having no
association at all after a successful install, because they read and write the
machine-wide half of the class root only. They were in this README and in the
script's closing advice first, and printed exactly the message a failed install
would have. `reg query HKCU\Software\Classes\.slpc /s` is the check that works,
and `cargo run --example opens-with -- some.slpc` is the better one: it answers
`Slipcase`, which is this application's own type query reading the registration
this directory just made.

**PowerShell's registry provider cannot write the media type key.** The name is
`application/x.slipcase+zip`, and the provider reads the forward slash as a
path separator: it creates `application` with a child `x.slipcase+zip` and
reports success. Both scripts use `[Microsoft.Win32.Registry]` instead, which
takes the whole string as one name.

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

It was checked at 16, 32, 48, and 256 before it was committed, which is what
`packaging/README.md` asks of any change to the drawing. At 16 the outline goes
grey and the card's rounded corners disappear, and the silhouette still reads.

## The window's own icon, and the taskbar

**Neither egui, eframe, nor winit sets an AppUserModelID.** `APP_ID` in
`src/main.rs` is `with_app_id`, which is Wayland's `xdg_toplevel.set_app_id`
and does nothing at all on Windows. Measured by reading all three crates.

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
the window at startup: 64 is a whole multiple of every size Windows draws it
at, so each is an integer downsample rather than a resample of a resample.

This is why `slipcase.ico` is a committed artifact in a repository that
otherwise holds only sources: the executable references it at compile time, and
Windows has no step that would rasterize the SVG for either purpose.
