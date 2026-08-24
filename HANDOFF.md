# Handoff: the platform this repository has still never run on

Everything here was written and first built on Linux. `DESIGN.md` §7 stage 4 is
file association per platform, and two thirds of it could not be done on the
machine the rest was done on. Windows has since been done on Windows; macOS
remains. This file says what is already true, what is missing, and where the
brief is.

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

`DESIGN.md` §3 and §8 carry the amendments and the measurements behind them.

## What is missing

**macOS, both halves.** `src/opens_with.rs` returns `None` there, so the card
says nothing about type — which the design permits where the platform will not
answer, but not as a way of never asking. There is no `packaging/macos` at all.
`HANDOFF-macos.md` is the brief and it is untouched.

The `.icns` converter has still to be written.
`packaging/windows/make-ico` is the one that exists, and a `.icns` one would be
its counterpart: same SVG, same reason, different container format.

## Something this file used to be wrong about

`CHECKLIST.md` was named by `CLAUDE.md` and by both briefs and had never been in
this repository — `git log --diff-filter=A` finds no commit that added it. It is
now at the root, started with the Windows walkthrough. The Linux section is a
placeholder: that walkthrough happened and its seven defects are recorded in
`git log` rather than in the file.

## The brief

`HANDOFF-macos.md`, written to be worked through by a Claude Code session on
that platform: clone, read, build, test, commit, push.

`HANDOFF-windows.md` is kept beside it as the record of what was asked for
there, and it is worth reading before starting the macOS one: the two were
written together and pose the same problems, and the Windows answers to several
of them — a safe API rather than the raw call the brief expected, an icon
converter as its own package, a walkthrough that found what the tests could not
— are the nearest thing to a precedent macOS has.

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
