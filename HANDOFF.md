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

`src/opens_with.rs` returns `None` on macOS and on Windows. The card then says
nothing about type, which the design permits where the platform will not answer
but not as a way of never asking.

Neither platform has any packaging at all.

## The briefs

`HANDOFF-macos.md` and `HANDOFF-windows.md`. Each is written to be worked
through by a Claude Code session on that platform: clone, read, build, test,
commit, push. Take one and leave the other alone — they touch different
`#[cfg]` arms of the same file and different directories under `packaging/`, so
they do not conflict, but only if each stays inside its own.

## Conventions this repository holds to

**Comments say why, not what.** A comment that restates the line below it is
noise; one that records what was measured, what was rejected, or what breaks
without the line is why the file is readable a year later.

**Every test's doc comment says what defect it would catch.** A test named for
its assertion and explained by its own body has not earned its place.

**Verify by measuring.** This repository has a history of claims that turned out
false under measurement, and every one of them is recorded rather than quietly
fixed. If you assert something about a platform, run the thing that proves it
and put the output in the commit message.

**Commit messages carry the reasoning.** Imperative subject, prose body, the
measurement, and the alternatives rejected. `git log` is the durable record of
this project and is meant to be read.

**`#![forbid(unsafe_code)]` stays.** See each brief for what that leaves open.

**Nothing compiles C.** `cc`, `cmake`, `pkg-config`, and `bindgen` must stay out
of the build-dependency tree. Check with `cargo tree -i cc` before adding any
dependency; a crate that links a system framework is fine, one that builds C is
not.

## Before you start

    cargo test                    # 59 tests, all should pass
    cargo clippy --all-targets    # no warnings
    cargo build --release

The conformance runner needs a checkout of `excelano/slipcase` with its cases
generated, which you may not have. It is a command and never a test:

    cargo run --bin corpus -- /path/to/slipcase/conformance

If you have it, run it before and after your change. 77 cases must agree.
