# Handoff: what each platform found

Everything here was written and first built on Linux. `DESIGN.md` §7 stage 4 is
file association per platform, and two thirds of it could not be done on the
machine the rest was done on. Windows was then done on Windows and macOS on a
Mac, so nothing is waiting on a platform. This file says what each found and
where the detail is.

**Read `DESIGN.md` first, then `SPEC.md` in `excelano/slipcase` §4.** The design
document is amended in place as building contradicts it, and every amendment
says what was measured. Do the same.

## What already works

Linux has the whole of `§8`. `packaging/linux` holds the media type, the desktop
entry, two icons, and an installer; `packaging/debian` builds the `.deb` the
Excelano apt repository ships. Both are checked: after installing, a `.slpc`
reports as `application/x.slipcase+zip` rather than `application/zip`, and
`xdg-mime query default` names `slipcase-desktop.desktop`.

`opens_with` answers on Linux by asking `xdg-mime`, never by carrying a table of
filenames mapped to types. `DESIGN.md` §3 forbids that table, and it is the one
rule in this area with no exceptions.

## Windows is done

Both tasks in `HANDOFF-windows.md`. `src/opens_with.rs` answers by reading the
registry along the path the shell takes, rather than calling
`AssocQueryString`, which is a raw call this crate cannot make; measured
against that API across 260 extensions, the two never disagreed and the
registry answered 18 times where the API declined. `packaging/windows` holds
two PowerShell scripts, the icon, and the tool that builds it from the Linux
SVG. Installing registers `.slpc` and `application/x.slipcase+zip`; Explorer
draws the icon, calls the type `Slipcase Container`, and double-clicking opens
a container with it loaded. Uninstalling was checked with a `UserChoice` in
place — the key that outranks the rest and the one an uninstaller forgets — and
leaves nothing.

## macOS is done

Both tasks in `HANDOFF-macos.md`. `opens_with` answers through Launch Services:
`UTType` turns the payload's extension into a type, `NSWorkspace` names the
application for it, and that application's `CFBundleDisplayName` gives the name
a person recognises. It needs no file on disk, so the placeholder trick the
Linux arm needs has no equivalent here. `packaging/macos` holds the property
list, the build script, and a README.

**The icon needed no tool.** Windows wrote `make-ico` because nothing on that
platform rasterizes an SVG; macOS has `sips`, which reads SVG, and `iconutil`,
which builds the `.icns`, both stock. `build-app.sh` renders the ten sizes and
checks each one, because `sips` rasterizes at the size the document declares and
would otherwise upscale a 64-pixel bitmap in silence.

Two things there are measured and unresolved rather than fixed, both in
`DESIGN.md` §8. **A double-clicked container is refused with an error dialog**,
because macOS delivers an opened document as an Apple Event rather than as
`argv[1]` and winit 0.30.13 exposes no hook for one. This is the one place the
three platforms differ on the argument path: Linux and Windows both hand the
path over as `argv[1]` and open the container. And **`mdls` reports the
synthesised type** for a `.slpc` after registration while Launch Services
reports the declared one, most likely because an unsigned bundle's exported type
is flagged `untrusted`.

`DESIGN.md` §2, §3, and §8 carry every amendment and the measurements behind
them.

## What is missing

Nothing is waiting on a platform. The one thing outstanding is the macOS Apple
Event above, which is not a platform's task but a constraint: receiving the
document needs an `NSApplicationDelegate` method, and that needs unsafe code in
this crate.

## Something this file used to be wrong about

`CHECKLIST.md` was named by `CLAUDE.md` and by both briefs and had never been in
this repository — `git log --diff-filter=A` finds no commit that added it. It is
now at the root, started with the Windows walkthrough. The Linux section is a
placeholder: that walkthrough happened and its seven defects are recorded in
`git log` rather than in the file.

## The briefs

`HANDOFF-macos.md` and `HANDOFF-windows.md` are both records now rather than
tasks. They were written together and pose the same problems, and reading the
two answers side by side is the most useful thing in them: both platforms were
told `AssocQueryString` and Launch Services were raw calls this crate could not
make, and both found a safe route instead, one through a registry crate and one
through objc2. Where they differ is worth as much. Windows needed its own icon
converter and macOS needed none; Windows delivers a double-clicked document as
`argv[1]` and macOS will not deliver it at all.

## Conventions

In `CLAUDE.md`, which a Claude Code session started in this directory loads by
itself. Read it before writing anything. The short version is that this
repository measures rather than assumes, records what it got wrong instead of
smoothing it away, and treats `git log` as a document.

## Before you start

    cargo test                    # 59 on Linux, 61 on Windows, 63 on macOS;
                                  # the difference is each platform's own tests
                                  # inside its own arm of opens_with.rs
    cargo clippy --all-targets    # no warnings
    cargo build --release

The conformance runner needs a checkout of `excelano/slipcase` with its cases
generated, which you may not have. It is a command and never a test:

    cargo run --bin corpus -- /path/to/slipcase/conformance

If you have it, run it before and after your change. 77 cases must agree.
