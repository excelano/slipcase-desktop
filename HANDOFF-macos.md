# Brief: macOS

Read `HANDOFF.md` first. Two tasks, in this order.

**This brief is a record.** Both tasks below are done. If you are here for the
release, `RELEASE.md` is the live document and this one is history.

---

## Task 1 — `opens_with` on macOS

`src/opens_with.rs` has one function per platform returning `Option<String>`:
the display name of the application the platform would open a payload with,
asked **from the payload's filename alone**, before anything has been
extracted. The Linux arm is the model. Read it before writing anything: it
solves the same problem and the shape of its answer is the shape of yours.

### The contract

Given `"report.pdf"`, return something a person recognises — `Preview`, not
`com.apple.Preview` and not `/System/Applications/Preview.app`. Given a name the
system has no association for, return `None`. Never guess, never fall back to
the extension in capital letters, and never carry a table: `DESIGN.md` §3 says
the application ships no mapping of filenames to types, and that is the rule
with no exceptions here.

The question is asked from a name, not from bytes. Nothing has been extracted
when the card is drawn, so there is no file to inspect. On Linux this needed a
trick — `xdg-mime` insists on a file that exists, so two placeholder files
carrying the payload's name are written, one sniffing as text and one as
binary, and the answer is taken only where both agree. Read that reasoning in
`mime_of`; macOS may need nothing like it, or may need something similar.

### The constraint that shapes this

`#![forbid(unsafe_code)]` is at the top of `src/lib.rs` and it stays. Launch
Services is a C API, so you cannot call it directly from this crate. Three
routes are open, in order of preference:

1. **A dependency whose unsafe lives inside it.** `forbid` is a property of this
   crate's own source, and `rfd` and `opener` already contain unsafe on our
   behalf. A maintained crate wrapping `LSCopyDefaultApplicationURLForContentType`
   or `NSWorkspace.urlForApplication(toOpen:)` is the clean answer. Check what
   exists and is maintained; do not vendor a dead one.
2. **Shelling out**, the way Linux shells to `xdg-mime`. Whatever you reach for
   has to be present on a stock macOS — `duti` is not, so it is not an option
   for a shipped application however convenient it is at a prompt.
3. **`None`, with the reason written down.** If neither of the above holds, that
   is a legitimate result and not a failure. Replace the current placeholder
   comment with what you tried, what the obstacle was, and what would change
   the answer. A dead end that is documented is worth more than one that is
   rediscovered.

Adding a dependency means adding it target-gated in `Cargo.toml`:

    [target.'cfg(target_os = "macos")'.dependencies]

with a comment saying what it is for, in the style the other entries use. Run
`cargo tree -i cc` afterwards and confirm nothing compiles C.

### How to exercise it

There is an example that asks the question without a window:

    cargo run --example opens-with -- report.pdf notes.txt data.bin archive.zip

On this Linux machine that prints:

    report.pdf: Document Viewer
    notes.txt: Text Editor
    data.bin: (the platform did not answer)
    archive.zip: Files

Yours should print the macOS equivalents. `data.bin` is the interesting one: on
Linux nothing claims it and the honest answer is silence. Find out what macOS
does with a name it has no association for, and make sure the answer is `None`
rather than something invented.

### Tests

Unit tests go in `src/opens_with.rs` beside the Linux ones. Test the parsing and
the decisions, not the system's answer: a test asserting that `.pdf` opens with
Preview fails on a machine where somebody changed the default, and a test that
fails for being true is worse than no test. The Linux tests are the pattern —
they test `name_in`, which is parsing, and never test what `xdg-mime` said.

Every test's doc comment says what defect it would catch.

---

## Task 2 — Packaging and association

`DESIGN.md` §8: *An application bundle with `CFBundleDocumentTypes` and an
exported type declaration conforming to `public.zip-archive`, which a container
is.*

Build it under `packaging/macos/`, matching how `packaging/linux/` is laid out:
the assets, a script that assembles them, and a `README.md` saying where each
file goes and why.

### What has to be true

The uniform type identifier is exported rather than imported, because this
application defines the type. Reverse-DNS under the organisation:
`com.excelano.slipcase` unless you have reason to prefer another, and say why in
the commit if you do. It conforms to `public.zip-archive` and to `public.data`.
Its extension is `slpc` and its MIME type is `application/x.slipcase+zip`, both
from `SPEC.md` §4, which is the authority — do not invent either.

`CFBundleDocumentTypes` claims that UTI with the role of `Viewer`, or `Editor`
given that the application writes metadata back. Decide which and justify it:
`Editor` is arguably right and makes macOS offer this application in more
places, which may or may not be wanted.

The bundle identifier and `APP_ID` in `src/main.rs` are the same kind of fact on
two platforms. `APP_ID` is `slipcase-desktop` and matches the Linux desktop
entry's basename, which is how a Wayland compositor finds the window's icon.
Check whether macOS needs anything of it and leave a note either way.

The icon exists as SVG at `packaging/linux/icons/slipcase-desktop.svg`. Convert
it to `.icns` at the sizes macOS wants. Look at the result at 16 points before
you commit it — the drawing was checked at 16, 24, 32, and 48 pixels on Linux
and the first version failed at 48.

Code signing and notarization are out of scope unless David says otherwise.
Record what an unsigned bundle does on a current macOS when double-clicked, so
the decision has evidence behind it when it is taken.

### How to verify

The association is not done until the platform agrees. Register the bundle, then
check that a `.slpc` file reports the exported UTI, that Finder shows the icon,
and that double-clicking one opens the application with that container loaded
rather than opening it empty. That last one is the argument path — `main` takes
one positional path — and on macOS a document is delivered as an Apple Event
rather than as `argv[1]`. **Expect this not to work and check whether it does.**
If `winit`/`eframe` does not deliver the opened document, say so plainly, record
what you found, and do not paper over it: an application that launches empty
when you double-click a file is a defect worth naming, not hiding.

---

## Finishing

    cargo test
    cargo clippy --all-targets
    cargo run --example opens-with -- report.pdf notes.txt data.bin archive.zip

Amend `DESIGN.md` where macOS contradicted it, marked **Amended** with what you
measured, the way §4, §5, §6, and §7 already are.

Add a section to `CHECKLIST.md` in the walkthrough style for anything only a
hand can test, and run it. The Linux walkthrough found seven defects that 59
tests and 77 corpus fixtures could not reach, all of them in layout, font
coverage, frame timing, and controls that existed but did nothing. A platform
nothing has ever run on will have its own.

Commit in the style of `git log`, and push. If something is unresolved, leave it
in the commit message and in `HANDOFF.md` rather than in your own head.
