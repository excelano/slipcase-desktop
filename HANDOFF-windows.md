# Brief: Windows

Read `HANDOFF.md` first. Two tasks, in this order.

**This brief is a record.** Both tasks below are done. If you are here for the
release, `RELEASE.md` is the live document and this one is history.

---

## Task 1 — `opens_with` on Windows

`src/opens_with.rs` has one function per platform returning `Option<String>`:
the display name of the application the platform would open a payload with,
asked **from the payload's filename alone**, before anything has been
extracted. The Linux arm is the model. Read it before writing anything: it
solves the same problem and the shape of its answer is the shape of yours.

### The contract

Given `"report.pdf"`, return something a person recognises — `Acrobat Reader`
or `Microsoft Edge`, not `AcroExch.Document.DC` and not a path to an `.exe`.
Given a name the system has no association for, return `None`. Never guess,
never fall back to the extension in capital letters, and never carry a table:
`DESIGN.md` §3 says the application ships no mapping of filenames to types, and
that is the rule with no exceptions here.

### The constraint that shapes this

`#![forbid(unsafe_code)]` is at the top of `src/lib.rs` and it stays.
`AssocQueryString` is the textbook answer and it is a raw FFI call, so it cannot
be made from this crate directly. Three routes are open, in order of preference:

1. **A dependency whose unsafe lives inside it.** `forbid` is a property of this
   crate's own source, and `rfd` and `opener` already contain unsafe on our
   behalf. Either a crate wrapping `AssocQueryString` safely, or a registry
   crate — the registry is where the answer lives and a safe registry API is
   easy to find.
2. **Shelling out.** Consistent with Linux, which shells to `xdg-mime`. Whatever
   you reach for has to be present on a stock Windows.
3. **`None`, with the reason written down.** A documented dead end beats one
   that gets rediscovered.

If you go the registry route, the order that decides the default is the part to
get right, and it is not obvious. A per-user choice under

    HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.pdf\UserChoice

beats the machine-wide association under `HKCR\.pdf`, and the ProgID it names is
then looked up for a friendly name. **Verify this against a real machine rather
than trusting the sketch above** — it is written from memory by someone who has
never run this code, and the friendly name is not always where you expect. Test
against an extension whose default you changed by hand, which is the case that
catches reading only the machine-wide key.

Adding a dependency means adding it target-gated in `Cargo.toml`:

    [target.'cfg(target_os = "windows")'.dependencies]

with a comment saying what it is for, in the style the other entries use. Run
`cargo tree -i cc` afterwards and confirm nothing compiles C. The whole
dependency tree is pure Rust today and `DESIGN.md` §2 states it as a property of
the project.

### How to exercise it

There is an example that asks the question without a window:

    cargo run --example opens-with -- report.pdf notes.txt data.bin archive.zip

On this Linux machine that prints:

    report.pdf: Document Viewer
    notes.txt: Text Editor
    data.bin: (the platform did not answer)
    archive.zip: Files

`data.bin` is the interesting one: on Linux nothing claims it and the honest
answer is silence. Windows will very likely hand back something for an unknown
extension rather than nothing — find out what, and make sure that case comes
back `None` instead of showing a person the words *Unknown application*.

### Tests

Unit tests go in `src/opens_with.rs` beside the Linux ones. Test the parsing and
the decisions, not the system's answer: a test asserting that `.pdf` opens with
Edge fails on a machine where somebody changed the default, and a test that
fails for being true is worse than no test. The Linux tests are the pattern —
they test `name_in`, which is parsing, and never test what `xdg-mime` said.

Every test's doc comment says what defect it would catch.

---

## Task 2 — Packaging and association

`DESIGN.md` §8: *The extension and the media type registered by the installer.*
That is one sentence for a whole platform, so most of the design here is yours
to make and to write down.

Build it under `packaging/windows/`, matching how `packaging/linux/` is laid
out: the assets, whatever assembles them, and a `README.md` saying where each
file goes and why.

### What has to be true

The extension is `.slpc` and the media type is `application/x.slipcase+zip`,
both from `SPEC.md` §4, which is the authority — do not invent either. SPEC §4
reserves no magic bytes, so the extension is the only identification available
and there is no content type to sniff.

Pick the installer technology and justify the pick in the commit. The
application is a single executable with no runtime files of its own beyond the
icon, so the lightest thing that registers an extension properly and uninstalls
cleanly is likely right. Whatever you choose, uninstalling has to remove the
association rather than leaving a dead ProgID pointing at a deleted binary.

The icon exists as SVG at `packaging/linux/icons/slipcase-desktop.svg`. Convert
it to `.ico` with the sizes Windows wants, 16 and 32 included. Look at it at 16
pixels before you commit it — the drawing was checked at 16, 24, 32, and 48 on
Linux and the first version failed at 48.

`APP_ID` in `src/main.rs` is `slipcase-desktop` and exists so a Wayland
compositor can match the window to its desktop entry. Windows has its own
notion, the Application User Model ID, which decides how the taskbar groups
windows and whether a pinned shortcut works. Check whether `eframe` sets one and
leave a note either way.

### How to verify

The association is not done until the platform agrees. Install, then check that
Explorer shows the icon for a `.slpc`, that the type description reads as
something a person would want to see, and that double-clicking one opens the
application **with that container loaded** rather than opening it empty. That
last one is the argument path: `main` reads one positional path, which is what a
file manager hands an application it was asked to open a document with. Confirm
it, because it has never been confirmed anywhere but Linux.

Then uninstall and check that nothing is left behind.

---

## Finishing

    cargo test
    cargo clippy --all-targets
    cargo run --example opens-with -- report.pdf notes.txt data.bin archive.zip

The repository already cross-compiles: `cargo check --target
x86_64-pc-windows-msvc` succeeds on Linux carrying no MSVC toolchain, and
`DESIGN.md` §2 states it. If your change breaks that, it is a real cost and
worth naming — a dependency that only resolves on Windows takes away a check the
Linux side has been relying on. Say so in the commit either way.

Amend `DESIGN.md` where Windows contradicted it, marked **Amended** with what
you measured, the way §4, §5, §6, and §7 already are.

Add a section to `CHECKLIST.md` in the walkthrough style for anything only a
hand can test, and run it. The Linux walkthrough found seven defects that 59
tests and 77 corpus fixtures could not reach, all of them in layout, font
coverage, frame timing, and controls that existed but did nothing. A platform
nothing has ever run on will have its own — and this one has never drawn a
single frame.

Commit in the style of `git log`, and push. If something is unresolved, leave it
in the commit message and in `HANDOFF.md` rather than in your own head.
