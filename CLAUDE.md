# CLAUDE.md

Guidance for Claude Code working in `slipcase-desktop`. Read it before touching
anything; it is short because `DESIGN.md` is where the reasoning lives.

---

## Every platform has drawn a frame now

This application was written and first built on Linux. Windows was then done on
Windows and macOS on a Mac, so both briefs beside `HANDOFF.md` are records
rather than tasks and nothing is waiting on a platform.

- **Linux** — `packaging/linux` and `packaging/debian`.
- **Windows** — `packaging/windows/README.md` says what was decided.
- **macOS** — `packaging/macos/README.md` says what was decided, including two
  things measured and left unresolved.

`HANDOFF.md` summarises what each platform found. Stay inside your own
platform's `#[cfg]` arm of `src/opens_with.rs` and its own
directory under `packaging/`.

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
    cargo test                    # the count differs per platform: each
                                  # platform's arm carries its own tests
    cargo clippy --all-targets    # must be silent
    cargo check --target x86_64-pc-windows-msvc   # cross-check, from Linux or macOS

    cargo run --example opens-with -- report.pdf notes.txt data.bin
    ./packaging/linux/install.sh          # Linux desktop integration
    ./packaging/debian/build-deb.sh       # the .deb, after a release build
    ./packaging/macos/build-app.sh        # Slipcase.app, after a release build

    powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1
    powershell -ExecutionPolicy Bypass -File packaging\windows\uninstall.ps1
    cd packaging/windows/make-ico && cargo run --release   # rebuild the .ico

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

**Unsafe code has exactly one home, and it is named.** A dependency containing
unsafe on our behalf is fine and always was — `rfd`, `opener`, and the `objc2`
crates all do. What this rule is about is unsafe in this crate's own source.

`src/lib.rs`, where containers are read and written, is `#![forbid(unsafe_code)]`
and that does not move. `src/main.rs` is `#![deny(unsafe_code)]`, and the
difference between the two words is the whole exception: `forbid` cannot be
lifted beneath it and `deny` can. Exactly one module lifts it,
`src/opened_document.rs`, carrying `#[allow(unsafe_code)]` on its declaration.

It exists because macOS is the only platform of the three that does not deliver
a double-clicked container as `argv[1]`, and receiving one needs an Objective-C
method. This was documented as impossible before it was done, and both the
attempt and the correction are in `git log`. Adding a second such module is a
decision to take with David, not one to take because there is precedent.

The rest of this section still has no exceptions.

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

**The trailer block is one line.** A `Co-Authored-By` naming the model, and
nothing under it. Some harnesses also append a `Claude-Session:` line carrying a
URL, and a co-author naming a context window rather than a model. This
repository is public, so a session URL in a commit message is a private
identifier written into a permanent public record for no reader's benefit. Both
have now been stripped from this history twice, the second time from commits
that had already been pushed, which cost a rewrite and a force-push. Read what
you are about to commit rather than trusting what the harness composed.

**Some things only a hand can test.** `CHECKLIST.md` at the root is the record.
On Linux it found eight defects the tests and the corpus could not reach, then
three more when the association was walked; on Windows two, a console window
behind the application and a window with no icon; on macOS the refusal a
double-click produced. They are enumerated there rather than counted here,
because the count in this paragraph was wrong for three days and nobody could
tell, the list having never been written down. Add a section for anything you
build that only a hand can check, run it, and write down what it found.

---

## Layout

    src/lib.rs          state, document operations, the save path, extraction
    src/main.rs         the window: panels, dialogs, threading, the card
    src/tree.rs         the metadata tree, one renderer per TOML type
    src/opens_with.rs   what the platform says would open a payload
    src/bin/corpus.rs   the conformance runner (a command, not a test)
    examples/           the type query without a window
    packaging/          per platform, plus debian
