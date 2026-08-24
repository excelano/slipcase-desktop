# Handoff: the two platforms this repository has never run on

Everything here was written, built, and tested on Linux. `DESIGN.md` §7 stage 4
is file association per platform, and two thirds of it cannot be done on the
machine the rest was done on. This file says what is already true, what is
missing, and where the two briefs are.

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

## What is missing

`src/opens_with.rs` returns `None` on macOS. The card then says nothing about
type, which the design permits where the platform will not answer but not as a
way of never asking.

Windows answers, as of `HANDOFF-windows.md` task 1. It reads the registry along
the path the shell takes rather than calling `AssocQueryString`, which is a raw
call this crate cannot make; DESIGN.md §3 carries the amendment and the
measurement behind it. Task 2 there — packaging and file association — is not
started.

Neither platform has any packaging at all.

`CHECKLIST.md` is named by this file's conventions, by `CLAUDE.md`, and by both
briefs, and has never been in this repository. Whoever writes the first one is
starting it rather than adding to it.

## The briefs

`HANDOFF-macos.md` and `HANDOFF-windows.md`. Each is written to be worked
through by a Claude Code session on that platform: clone, read, build, test,
commit, push. Take one and leave the other alone — they touch different
`#[cfg]` arms of the same file and different directories under `packaging/`, so
they do not conflict, but only if each stays inside its own.

## Conventions

In `CLAUDE.md`, which a Claude Code session started in this directory loads by
itself. Read it before writing anything. The short version is that this
repository measures rather than assumes, records what it got wrong instead of
smoothing it away, and treats `git log` as a document.

## Before you start

    cargo test                    # 59 tests, all should pass
    cargo clippy --all-targets    # no warnings
    cargo build --release

The conformance runner needs a checkout of `excelano/slipcase` with its cases
generated, which you may not have. It is a command and never a test:

    cargo run --bin corpus -- /path/to/slipcase/conformance

If you have it, run it before and after your change. 77 cases must agree.
