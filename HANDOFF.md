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

**Double-clicking opens the container**, which took three attempts and one
agreed exception to the unsafe rule. macOS is the only one of the three that
does not hand the path over as `argv[1]`: it sends an Apple Event, nothing was
listening, and Finder blamed the application for a format its own bundle
declared. `src/opened_document.rs` listens, through the Apple Event manager
rather than the application delegate winit owns. Registering at
`applicationWillFinishLaunching:` is the only moment that works, and the two
that do not are tabulated in `packaging/macos/README.md`.

One thing there is measured and unresolved. **`mdls` reports the synthesised
type** for a `.slpc` after registration while Launch Services reports the
declared one, most likely because an unsigned bundle's exported type is flagged
`untrusted`.

`DESIGN.md` §2, §3, and §8 carry every amendment and the measurements behind
them.

## What is missing

Nothing is waiting on a platform, and all three now open a double-clicked
container. What is left is a signature: an unsigned bundle is refused by
Gatekeeper on any machine that did not build it, which also looks like the
reason Spotlight will not take the exported type.

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

## Continuous integration, one file per platform

`.github/workflows/apple-silicon.yml` is the first, and it is named for what it
is rather than `ci.yml` on purpose: Linux and Windows want their own files, and
one file per platform means a session can add or change its own without touching
anybody else's. It is the same rule as staying inside your own `#[cfg]` arm of
`src/opens_with.rs` and your own directory under `packaging/`.

macOS needed it first for a reason that is not general. Every machine this was
built on is `x86_64`, a Store build has to be universal, and cross-compiling to
`aarch64-apple-darwin` produces something an Intel Mac cannot run — so the arm64
half was code nobody had executed. GitHub's macOS runners are Apple silicon from
`macos-14` onwards, and that workflow checks `uname -m` rather than trusting the
label, because the label is GitHub's to remap.

Two things it does that a platform workflow should copy. It runs the conformance
corpus, which is a command and never a test here because it needs another
checkout with its cases generated — CI is a machine that can always have them,
`excelano/slipcase` is public, and 77 fixtures are worth more than a build that
only compiles. And it treats a clippy warning as an error, because `CLAUDE.md`
says clippy must be silent and CI is where that stops being a habit and starts
being enforced.

What it deliberately does not claim is the window. Nothing headless reaches a
double-clicked document, an icon, or a frame a person looks at. `CHECKLIST.md`
stays the record for those on every platform.

## Conventions

In `CLAUDE.md`, which a Claude Code session started in this directory loads by
itself. Read it before writing anything. The short version is that this
repository measures rather than assumes, records what it got wrong instead of
smoothing it away, and treats `git log` as a document.

## Before you start

    cargo test                    # the count differs per platform: each
                                  # platform's own tests sit inside its own
                                  # arm of opens_with.rs
    cargo clippy --all-targets    # no warnings
    cargo build --release

The conformance runner needs a checkout of `excelano/slipcase` with its cases
generated, which you may not have. It is a command and never a test:

    cargo run --bin corpus -- /path/to/slipcase/conformance

If you have it, run it before and after your change. 77 cases must agree.
