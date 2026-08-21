# CLAUDE.md

Guidance for Claude Code working in `slipcase-desktop`. Read it before touching
anything; it is short because `DESIGN.md` is where the reasoning lives.

---

## If you are on macOS or Windows, you have a brief

This application was written, built, and tested entirely on Linux. Two platforms
have never drawn a frame of it, and each has a task waiting.

- **macOS** — read `HANDOFF-macos.md`, then `HANDOFF.md` for context.
- **Windows** — read `HANDOFF-windows.md`, then `HANDOFF.md` for context.
- **Linux** — nothing is waiting; `HANDOFF.md` says what the other two owe.

Take your own platform's brief and leave the other alone. They touch different
`#[cfg]` arms of `src/opens_with.rs` and different directories under
`packaging/`, so they do not conflict, but only if each stays inside its own.

---

## What this is

A desktop application that opens a `.slpc` container, shows its metadata as an
editable tree, and hands the payload to whatever the operating system has
registered for it. Presented to a person as **Slipcase**; the crate and the
binary are `slipcase-desktop`.

**It parses no containers.** Every read, every write, and every verdict comes
from `slpc`, the library in `excelano/slpc-rust`. Where it needs behaviour
the library lacks, the behaviour goes into the library — twice so far, filed as
issues and both fixed upstream rather than worked around here.

**Three documents, three authorities.** `SPEC.md` in `excelano/slipcase` is the
authority on the format and this repository neither restates nor amends it.
`DESIGN.md` here is the authority on this application. `git log` is the record
of why everything is the way it is, and it is written to be read.

---

## Commands

    cargo build                   # debug
    cargo build --release
    cargo test                    # 59 tests
    cargo clippy --all-targets    # must be silent
    cargo check --target x86_64-pc-windows-msvc   # cross-check, succeeds on Linux

    cargo run --example opens-with -- report.pdf notes.txt data.bin
    ./packaging/linux/install.sh          # Linux desktop integration
    ./packaging/debian/build-deb.sh       # the .deb, after a release build

**The conformance corpus is a command and never a test.** It needs a checkout of
`excelano/slipcase` with its cases generated, which `cargo test` does not imply,
and a test that has to choose between skipping quietly and failing on a machine
that was never going to have those things is worse than a command run on
purpose. It is the harness that matters — 77 fixtures across verdict, tree,
card, extraction, rewrite, rename, replacement, and pre-flight:

    cargo run --bin corpus -- /path/to/slipcase/conformance

All 77 must agree. Run it before and after any change to `src/lib.rs`.

Two things that have caught people out. `cargo run --bin corpus` rebuilds every
binary in the package, including `slipcase-desktop`, so do not run it while the
application is running from the same target directory. And the target directory
may not be `./target`: `[build] target-dir` in a Cargo configuration file moves
it and no environment variable then says so, which is why the packaging scripts
ask `cargo metadata` rather than guessing.

---

## Rules with no exceptions

**`#![forbid(unsafe_code)]` stays.** It is a property of this crate's own source.
A dependency containing unsafe on our behalf is fine — `rfd` and `opener`
already do.

**Nothing compiles C.** `cc`, `cmake`, `pkg-config`, and `bindgen` stay out of
the build-dependency tree. Run `cargo tree -i cc` after adding any dependency. A
crate that links a system library is fine; one that builds C is not.

**No table mapping filenames to types.** What the card says about a payload's
type is what the platform said. Where the platform will not answer, the card
says nothing rather than guessing. `DESIGN.md` §3.

**The library is not worked around.** If `slpc` cannot do something, file it
against `excelano/slpc-rust` with a runnable reproduction and say so here.

---

## How to work

**Measure, do not assume.** This repository has a history of confident claims
that failed under measurement — that `toml_edit` reproduces a document byte for
byte, that a glyph would render, that a blocking dialog was fine, that the
target directory is `./target`. Every one is recorded rather than quietly fixed.
If you assert something, run the thing that proves it.

**Check that a test bites.** A regression test that passes against the defect it
was written for is worse than no test, and that has happened here. Break the fix
deliberately, watch the test fail, put it back.

**Comments say why.** A comment restating the line below it is noise. One
recording what was measured, what was rejected, or what breaks without the line
is why a file is readable a year later.

**Every test's doc comment says what defect it would catch.**

**Amend `DESIGN.md` when building contradicts it.** In place, marked
**Amended**, stating what was measured. Do not smooth the amendment away: a
design document that quietly rewrites itself to match the code is worth nothing
as a record.

**Commit messages carry the reasoning.** Imperative subject under about 60
characters, prose body with the measurement and the alternatives rejected. Read
`git log` before writing your first one.

**Some things only a hand can test.** `CHECKLIST.md` in the walkthrough folder
is the record. On Linux it found seven defects that the tests and the corpus
could not reach — layout geometry, font coverage, frame timing, and controls
that were drawn but did nothing. Add a section for anything you build that only
a hand can check, and run it.

---

## Layout

    src/lib.rs          state, document operations, the save path, extraction
    src/main.rs         the window: panels, dialogs, threading, the card
    src/tree.rs         the metadata tree, one renderer per TOML type
    src/opens_with.rs   what the platform says would open a payload
    src/bin/corpus.rs   the conformance runner (a command, not a test)
    examples/           the type query without a window
    packaging/          per platform, plus debian
