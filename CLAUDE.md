# CLAUDE.md

A desktop application that opens a `.slpc` container, shows its flyleaf as an editable
tree, hands the content file to whatever the operating system has registered for it, and
makes a container out of a file a person chooses. Presented as **Slipcase**; the crate and
the binary are `slipcase-desktop`. It parses no containers: every read, write and verdict
comes from `slpc` in `excelano/slpc-rust`, and behaviour that library lacks goes into that
library. The flyleaf tree is `flyleaf::render` from `excelano/flyleaf`, called once from
`src/main.rs` with `RequiredKeys`; behaviour the widget lacks goes there. `SPEC.md` in
`excelano/slipcase` is the authority on the format and this repository neither restates nor
amends it. `DESIGN.md` here is the authority on the application.

## Commands

    cargo build --release
    cargo test                        # each platform's arm carries its own tests
    cargo clippy --all-targets        # must be silent
    cargo check --target x86_64-pc-windows-msvc        # cross-check from Linux or macOS
    cargo run --example corpus -- /path/to/slipcase/conformance   # a command, never a test
    ./packaging/linux/check-libraries.sh               # needs a display
    ./packaging/debian/build-deb.sh
    ./packaging/macos/build-app.sh
    powershell -ExecutionPolicy Bypass -File packaging\windows\check-imports.ps1
    powershell -ExecutionPolicy Bypass -File packaging\windows\check-install.ps1
    ./po/update-po.sh                 # after changing any sentence a person reads
    ./po/pseudo.sh                    # then a debug build with POTEXT_LANG=en-x-pseudo

The target directory may not be `./target`: `[build] target-dir` moves it and no
environment variable says so, which is why the packaging scripts ask `cargo metadata`.
Releases: run `ship slipcase/slipcase-desktop`. There is no release document.

## Rules

Stay inside your own platform's arm: its `#[cfg]` arm of `src/opens_with.rs`, its directory
under `packaging/`, its file under `.github/workflows/`. Reviewing another platform's arm
is worth doing and is how the worst defects here were found; what you cannot settle from
here goes to David rather than into a guess. `src/lib.rs` is `forbid(unsafe_code)` and does
not move; `src/main.rs` is `deny`, lifted for `src/opened_document.rs` alone, where macOS
delivers a double-clicked container as an Apple Event. A second such module is David's
decision. Nothing in this crate compiles C, and the check is always the artefact rather
than the manifest — `cargo tree -i cc` is non-empty and always will be
(`~/notes/pure_rust_preference.md`). Each platform has its own artefact check because each
had the same defect: `check-imports.ps1` walks the PE import table, `check-libraries.sh`
reads `/proc/PID/maps` under both display backends, and `build-app.sh` refuses a symbol no
public framework header declares. A `cargo update` or an `eframe` bump off winit 0.30.13
stops `[patch.crates-io]` applying, which is the moment to check whether the release it
lands on carries the `private-apple-apis` gate. No table maps filenames to types: what the
card says about a content file is what the platform said, and where it will not answer the card
says nothing (`DESIGN.md` §3). This tree is not rustfmt-clean and has no fmt check: never run `cargo fmt` here, or a
six-file change arrives swamped by eight hundred lines nobody asked for. Every test's doc
comment says what defect it would catch,
and a new test is broken deliberately once to watch it fail. The commit trailer is one
line, a `Co-Authored-By` naming the model: this repository is public, so no session URL.
