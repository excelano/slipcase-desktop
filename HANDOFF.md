# Handoff: the platforms this repository was not written on

Everything here was written, built, and tested on Linux. `DESIGN.md` §7 stage 4
is file association per platform, and two thirds of it could not be done on the
machine the rest was done on. macOS has since been done on a Mac; Windows has
not. This file says what is already true, what is missing, and where the
remaining brief is.

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

## What macOS has since done

`opens_with` answers there, through Launch Services rather than a table:
`UTType` turns the payload's extension into a type, `NSWorkspace` names the
application, and the bundle's `CFBundleDisplayName` gives the name a person
recognises. It needs no file on disk, so it asks nothing of the placeholder
trick the Linux arm needs. `packaging/macos` holds the bundle, the exported type
declaration, and the icon, and `packaging/macos/README.md` is the detail.

Two things there are measured and unresolved rather than fixed, and both are in
`DESIGN.md` §8. **A double-clicked container is refused with an error dialog**,
because macOS delivers an opened document as an Apple Event rather than as
`argv[1]` and winit 0.30.13 exposes no hook for one; implementing it here would
need unsafe code in this crate. The type and the association are correct and it
is delivery that fails. It belongs upstream in winit. And **`mdls` reports the wrong
type** for a `.slpc` after registration while Launch Services reports the right
one, most likely because an unsigned bundle's exported type is flagged
`untrusted`.

## What is missing

`src/opens_with.rs` returns `None` on Windows. The card then says nothing about
type, which the design permits where the platform will not answer but not as a
way of never asking.

Windows has no packaging at all.

## The briefs

`HANDOFF-windows.md` is the one still outstanding, written to be worked through
by a Claude Code session on that platform: clone, read, build, test, commit,
push. `HANDOFF-macos.md` is kept as the record of what that platform was asked
for and what it found.

The Windows arm is now `#[cfg(not(any(target_os = "linux", target_os = "macos")))]`
rather than the combined stub the macOS work split, and `packaging/windows` is
still empty. `cargo check --target x86_64-pc-windows-msvc` succeeds from macOS
as well as from Linux, so the cross-check is available before that platform is
in front of anyone.

## Conventions

In `CLAUDE.md`, which a Claude Code session started in this directory loads by
itself. Read it before writing anything. The short version is that this
repository measures rather than assumes, records what it got wrong instead of
smoothing it away, and treats `git log` as a document.

## Before you start

    cargo test                    # 63 on macOS, 59 on Linux; the difference
                                  # is the platform's own tests in opens_with.rs
    cargo clippy --all-targets    # no warnings
    cargo build --release

The conformance runner needs a checkout of `excelano/slipcase` with its cases
generated, which you may not have. It is a command and never a test:

    cargo run --bin corpus -- /path/to/slipcase/conformance

If you have it, run it before and after your change. 77 cases must agree.
