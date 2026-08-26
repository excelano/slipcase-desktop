# Handoff: what each platform found

Everything here was written and first built on Linux. `DESIGN.md` §7 stage 4 is
file association per platform, and two thirds of it could not be done on the
machine the rest was done on. Windows was then done on Windows and macOS on a
Mac, so no platform is holding up a stage. This file says what each found and
where the detail is, and *What is waiting on a platform* below is what has come
back since — what one machine's review turned up in another machine's arm.
Reading an arm you cannot run has now found a defect that running it had not,
so the section is worth keeping rather than emptying.

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

## What is waiting on a platform

A signature is no longer the answer to this section. `build-app.sh --sign`
signs the bundle with an Apple Development certificate and reads the
entitlements back out of the signature, and that settled the Spotlight
question: the exported type is flagged `trusted` rather than `untrusted`, and
`mdls` reports `com.excelano.slipcase` rather than the synthesised type. Still
unrun is a *distribution*-signed bundle carrying a provisioning profile, which
is a different sandbox context from the development-signed one every
measurement was taken against.

Two things were found on Linux while reviewing the macOS work, and neither
belongs to Linux. They are here because this file is the only way they reach
the session that owns the arm. Both have since been settled and are left below
with their answers, because what the review cost and what it caught are both
worth knowing before the next one.

**Windows: `carries_a_mark` asked whether the stream exists, not whether it
gates.** Settled on 2026-08-26, and it was a defect rather than a doubt. In
`src/provenance.rs` both questions were answered by
`std::fs::metadata(path:Zone.Identifier).is_ok()`. That was harmless until
`carry` gained its `AlreadyMarked` fallback, which uses the predicate to decide
whether a payload whose zone write failed is safe to hand to the system.
`std::fs::write` creates the stream before it writes into it, so a write that
fails partway — a full disk being the realistic one — leaves a stream that
exists and carries no `ZoneId` line, and a stream with no `ZoneId` is not a
file the shell gates. `carry` called that `AlreadyMarked` and the payload
opened ungated.

Reproduced before it was fixed, which is what the note asking for it was for.
The read only attribute denies the stream write the way the macOS tests use a
mode of `0o444` to stand in for a sandbox, and against the old predicate the
test failed exactly as the paragraph above said it would. The repair is the one
that was asked for — the predicate says what it means — and what the shell
means was measured rather than reasoned about: a script run under
`-ExecutionPolicy RemoteSigned` resolves its zone through the same stream, and
it stops for a `ZoneId` of 3 or above inside a `[ZoneTransfer]` section and for
nothing else. Zones 0, 1 and 2, an empty value, and a `ZoneId` under any other
section or under none all ran. `DESIGN.md` §8 carries the table and the one
place the predicate is deliberately stricter than the platform. Six tests own
the Windows arm now, where it had none, and three deliberate breaks were run
against them.

One thing it did not settle. The premise that a full disk is what leaves the
stream half written is still read out of `std::fs::write` rather than measured:
this machine has no second volume to fill and no administrator to make one, so
the reproduction constructs the state the failure leaves rather than causing
the failure. The macOS arm did own a second volume for its own question, so a
session that has one may be able to do better.

**Windows: the channel is chosen, and the container has now been measured.**
The Microsoft Store, in MSIX, decided for the reason macOS chose the Mac App
Store and recorded in `packaging/windows/README.md` with the paragraph it
amends left standing. The three questions were named after the macOS sandbox
question deliberately, because that was taken for a formality and cost a new
module, a rewritten save path, and a reopened section of `DESIGN.md`.

They were run on 2026-08-26 against a signed package built from the release
binary, and MSIX turned out to be the cheaper container. `opens_with` gets the
same answers inside as outside — twenty payload types, no row different —
because a package's registry virtualisation covers its writes and not its
reads, which was established by watching a write from inside the container fail
to appear outside it before anything else measured inside it was believed.
`opener::open` reaches the shell from inside and the handler starts. And the
association declared in `AppxManifest.xml` needs no registry key from anybody:
a double-clicked container launches the packaged binary. Nothing in
`src/` has to change, which is not what the macOS answer looked like.

What it did find is at the edges. With the package and the two scripts both
registered, Windows chooses neither and shows its *how do you want to open this
file* picker, so a package installed over a live script installation turns a
working association into a prompt — the package has to clean up after the
scripts, or say that they must be uninstalled first. `AssocQueryString` reports
`ERROR_NO_APPLICATION_ASSOCIATED` for a packaged handler's executable and
command line while still returning its friendly names, which is the same trap
as `assoc` and `ftype` and will make any executable-path check call a correct
package no association at all. And `opener::open` returns `Ok` for a payload
nothing is registered for, on both builds, so that return value is not evidence
anything opened.

One part is left and is left deliberately. Whether a declared association beats
a *stale* `UserChoice` cannot be measured by a script: the key is
hash-validated and denies the user write access, so a default is something a
person chooses and cannot be forged into place. It is in `CHECKLIST.md` with
the sequence, including the one step that matters — remove the script install
by hand rather than with `uninstall.ps1`, which removes the `UserChoice` and
would destroy what is being tested.

Getting any package installed costs exactly one administrator action, which is
worth knowing before the next session plans around it. `makeappx`,
`New-SelfSignedCertificate`, `signtool` and `Add-AppxPackage` all run as an
ordinary user; the signing certificate has to reach `LocalMachine\TrustedPeople`
and the per-user store is not read for it, and Developer Mode is the same
elevation by another door.

**macOS: replacing across volumes was never run, and it did not work.**
Answered the day it was raised, and it was a defect rather than a doubt. The
rewrite waited wherever `TMPDIR` pointed, which is the boot volume, and
`replaceItemAtURL:` wants both of its ends on one volume: APFS, HFS+, FAT32 and
exFAT all refuse with `EXDEV` under a Cocoa 512, so Save did not work for any
container on an external drive, a mounted image, or a share. The original was
untouched and the error reached the person every time, so nothing was lost —
it simply never worked, and it went unnoticed because everything opened here
had been on the boot volume. `NSItemReplacementDirectory` asked with
`appropriateForURL:` is the directory Apple provides for this and lands on the
right volume; the boot-volume case is unchanged because it returns one in the
same per-user temporary area as before. `CHECKLIST.md` holds the run and
`DESIGN.md` is amended a second time. A test owns a second volume now, and it
took two goes: the first read the mount point out of the wrong field and
replaced boot-volume files with boot-volume files, passing green against the
break that was supposed to prove it bites.

What is left of it is the sandbox, which no test can enter: the replacement
directory on a second volume is not the file a person chose through the open
panel, and Apple's word that the grant reaches it is documentation rather than
measurement. That is an item in `CHECKLIST.md` under macOS.

One thing from the same review is settled rather than waiting, and it is worth
knowing about because it cuts both ways. `Destination::in_place` carries the
original's permissions onto the replacement and `Destination::new` deliberately
does not, so when the macOS arm changed constructors the container's mode
stopped coming from the library and started coming from Apple. That claim was
documentation rather than measurement until
`the_container_keeps_the_permissions_it_had` was written; it passes on the
Apple silicon runner, so the replacement does keep them. The general lesson is
the one worth carrying into the other arms: a platform-specific arm inherits
none of the library's promises that its constructor did not ask for.

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

`.github/workflows/linux.yml` is the second, and it exists for the ordinary
reason rather than a gap: Linux is the machine everything was written on. Past
the suite and the corpus it does two things the macOS file does not. It builds
the `.deb` and runs the mechanical parts of the Linux walkthrough over it —
ownership, modes, and every hash in `md5sums` agreeing — because that package
shipped without `md5sums` once and a release build passed over it. And it runs
lintian as a *report* rather than a gate, which is deliberate: nobody has read
its output yet, so nothing knows which tags are findings and which are
conventions this package has declined, and failing a build on an answer nobody
has triaged would be asserting one. Triaging them is an open item.

Two things it is honest about that a copy of it should stay honest about. The
build installs no development packages, because it needs none — `ldd` names
libc, libgcc and libm and nothing else. And the tests do not reach `xdg-mime`:
run with the tool off `PATH` they all still pass, since the Linux arm returns
`None` where it is absent and the tests accept that. So that file does not
repeat the macOS one's claim that its tests exercise the platform.

Its first run measured the last of those. `xdg-mime` is no use on a machine
with no desktop session: with no session to defer to it falls back to `file`,
which reads magic bytes, and `SPEC.md` §4 reserves none — so a correctly
registered container came back `application/zip`, which is what a ZIP looks
like to anything reading its contents rather than its name. The step asks GLib
instead, which is the path a GTK file manager takes and the one whose answer
items 1 and 2 are actually about. Worth knowing before a Windows or minimal-
machine session reaches for `xdg-mime` to prove something.

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
