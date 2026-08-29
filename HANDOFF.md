# Handoff: what each platform found

Everything here was written and first built on Linux. `DESIGN.md` §7 stage 4 is
file association per platform, and two thirds of it could not be done on the
machine the rest was done on. Windows was then done on Windows and macOS on a
Mac, so no platform is holding up a stage. This file says what each found and
where the detail is, and *What is waiting on a platform* below is what has come
back since — what one machine's review turned up in another machine's arm.
Reading an arm you cannot run has now found a defect that running it had not,
so the section is worth keeping rather than emptying.

**For the release, `RELEASE.md` is the live document** and the two briefs
beside this one are records. It says what is left on each platform, in what
order, and which of it a script should be doing instead of a person.

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


**Provenance was walked on 2026-08-26**, the last thing in this arm that had
compiled without ever executing. It works: a container carrying a
`Zone.Identifier` reports as arrived from elsewhere and one built here does
not, and both extraction paths carry the whole stream onto the payload byte for
byte. The walk also answered the question `DESIGN.md` §5 had left resting on
nothing — a marked file in the temporary directory is gated exactly as one
anywhere else, so the Open button is right not to be disabled — and it closed a
gap nobody had listed: the card's provenance line had no test on this platform,
because the one that covers it is `#[cfg(target_os = "linux")]`. There is a
Windows counterpart now. `CHECKLIST.md` holds the run.

**The window was walked the same day**, which is the last thing in this arm
that only an eye settles. The card draws the provenance line for a container
carrying a zone stream and not for one built here; Open hands the payload
over and the copy carries the whole stream; a zoned `.cmd` in the temporary
directory raises the security warning, so `§5` is now measured from both
sides; and a payload named `CON` extracts as a real file and refuses to open
instead of hanging. It found one defect, in a sentence this application
writes rather than one the platform hands over: `opener::OpenError` keeps its
`Display` to a category, so every refused handover read *the system would not
open it: IO error*. It reads the platform's own words now. `DESIGN.md` §8 is
amended, because it had recorded the platform's wording as what a person sees
before anybody had looked.

**And the hand list is empty.** The stale `UserChoice`, a scaled display, an
upgrade over a running install and a second user account were all run on the
same day, with David at the machine for the parts only a person can do — the
choice a `UserChoice` records cannot be forged, and a display scaling cannot be
changed from here. Three of the four found something, and two of those were
sentences written before anybody had looked: that 64 divides every size Windows
draws a window icon at, and that an upgrade over a running copy explains itself.
The fourth found that a per-user install really is invisible to another account.
`CHECKLIST.md` holds all of it, including one line of one run that came from the
wrong process and is marked void rather than dropped.

**Light mode was the one nobody had thought to try.** Every walkthrough above ran
in dark, and the card colours two lines on purpose. Measured against the card's
own fill, the provenance line was 2.79:1 in light mode where WCAG asks 4.5:1 —
the one line coloured so that it gets read was the least readable thing on the
card — and the failure line was under the bar in *both* themes. `src/main.rs`
picks per theme now and a test holds both to 4.5:1. Nothing about that is
Windows's: it is egui's defaults used unchanged, so Linux and macOS had it too.

Reviewed on Linux the same day, where the dark-mode figures turned out to have
been measured against the wrong grey and every one of them was low. The repair
stands and no conclusion moves — the corrected numbers fail the same bar — but
`CHECKLIST.md` says which were wrong and why the cross-check that was supposed
to catch it did not: it only ever ran in the theme somebody was already looking
at. The figures now come out of the test rather than sitting beside it.

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

**macOS and Linux: the repaired light-mode card has never been looked at.**
Raised by the Windows session on 2026-08-28 and unanswerable from here.

The contrast defect was not Windows's. `CHECKLIST.md`'s *What light mode found*
measured the card's provenance line at 2.79:1 in light mode against a 4.5:1 bar,
and the failure line under the bar in both themes, and the note in this file
already says why that reaches everybody: it is egui's defaults used unchanged,
so Linux and macOS had it too. `warn_colour` and `error_colour` in
`src/main.rs` pick per theme now and a test holds both themes to 4.5:1.

**What is missing on those two platforms is a look, not a measurement**, and the
distinction turned out to matter here. Every Windows walkthrough had run in dark
mode, so the light card had been seen while it was broken and not since it was
fixed — the repair was proven by a test and by nobody's eyes. Looked at on
2026-08-28 when David switched the desktop: it reads. The pixels were measured
off the screenshot too, because that section is a story about figures computed
against a colour the screen never showed, and the antialiased worst case is
4.59:1 where the colour itself is 5.18:1. Both clear the bar, and only the first
can be taken off a screen.

So each of the other two arms wants the same thing: open a container in light
mode and look at the card. It is one glance, it needs the platform, and the
repair it is checking was shared code that nobody has seen work anywhere but
here.

**macOS answered on 2026-08-28: it reads, and the screen agrees with the
arithmetic exactly.** One container carrying both coloured lines at once, opened
in the signed sandboxed bundle with the system switched to Light. Sampled off the
screenshot: the card fill is rgb(248, 248, 248), the error line rgb(180, 0, 0) at
6.72:1, and the warning line rgb(180, 70, 0) at 5.18:1 — the recorded figures to
the digit, so on this display there is no gap between the colour computed and the
colour shown. `CHECKLIST.md`'s *What the light card looked like here* holds it.

One thing for whoever runs this on Linux, and for the entry above. **The
*antialiased worst case* is not a figure that compares across platforms**, because
it is a coverage threshold nobody has stated. On the warning line here it is
4.41:1 at 90% coverage, 4.78:1 at 95% and 5.15:1 at 99%, which brackets the
4.59:1 recorded above rather than disagreeing with it, and below half coverage it
collapses toward the fill by construction the way every antialiased glyph does.
The core colour is the number that travels. Linux is the last arm that has not
looked.

**Linux answered on 2026-08-28, and the answer is that it cannot look.**
Slipcase does not follow the desktop theme on this platform: it drew the dark
card on a light GNOME desktop, and drew it again with the appearance setting
forced to `prefer-light` while the window's own titlebar turned light in the
same screenshot. `winit`'s `system_theme()` is an unconditional `None` on Linux
where the Windows and macOS arms both return a real answer, so egui falls back
to dark and no desktop setting reaches it. `CHECKLIST.md`'s *What looking for
the light card found, and why there was none* has the table and the trace.

**Fixed the same day, and this section has nothing left in it.** David's answer
was that all three platforms should respect the setting. `src/system_theme.rs`
asks the portal and follows it, the three settings now produce dark, light and
light, a change made while the window is open is picked up without a relaunch,
and the light card was then looked at and measured here: 6.72:1 and 5.18:1, the
macOS figures to the digit. `DESIGN.md` §3 carries the reasoning and
`CHECKLIST.md` holds the run.

What follows is what the question looked like before it was answered, kept
because the way it was diagnosed is the useful part.

So the light card is unreachable on this platform rather than unlooked-at, and
the two questions that leaves are David's: whether the application should read
the theme itself — the portal's `org.freedesktop.appearance color-scheme`, which
this same process already reads once — and whether the contrast repair still
earns its place on an arm where the theme it repairs cannot be selected.

That *once* is worth knowing before anybody costs the work. The titlebar's
answer comes from `sctk-adwaita`, which spawns `dbus-send` and greps its output
for `uint32 1`, with a 100ms timeout and a shrug if it misses. So the portal is
already being asked inside the window that ignores it, by a subprocess, and
whatever is decided here should not be a second copy of that.
Neither is a packaging decision and neither blocks a release.

A signature is no longer the answer to this section. `build-app.sh --sign`
signs the bundle with an Apple Development certificate and reads the
entitlements back out of the signature, and that settled the Spotlight
question: the exported type is flagged `trusted` rather than `untrusted`, and
`mdls` reports `com.excelano.slipcase` rather than the synthesised type. Still
unrun is a *distribution*-signed bundle carrying a provisioning profile, which
is a different sandbox context from the development-signed one every
measurement was taken against.

**Windows: should the origin note be written here too, and does an unknown
stream survive anything?** Raised by the macOS session on 2026-08-28 and
unanswerable from either of the other two platforms.

Under the App Sandbox macOS marks whatever the process writes and refuses to
have that mark replaced, so `slpc::provenance::carry` could keep the file gated
and still lose the answer to *where did this come from*. `slpc` 0.3.10 fixes it
by keeping the source's value verbatim in an attribute of its own,
`com.excelano.slipcase.origin`, which `arrived_from_elsewhere` consults and
`carries_a_mark` deliberately does not.

**The Windows arm can reach the same branch and does nothing there.** `carry`'s
fallback fires whenever the zone write fails over a copy that already carries a
gating stream, so it is not dead code the way the Linux arm's is. A second
stream beside `Zone.Identifier` would hold a note perfectly well — a stream is
addressed by appending `:name` and `std::fs` reaches it, which is how the zone
write already works. What is missing is a measurement, and three things need
one:

- What the shell does with a stream it does not recognise. Nothing established.
- Whether it survives the copies and the archivers that strip `Zone.Identifier`.
  If it does not, a note is worth less than it looks; if it survives *more* than
  the zone stream does, that is worth knowing too and in the other direction.
- Whether a packaged install sees it at all. An MSIX-packaged application is the
  case that matters, and it is the one only this platform can try.

Writing the note here on the strength of the macOS result would be exactly the
inference this project keeps refusing to make, so the arm is a documented stub
rather than a guess. If the measurement says a note is useless on Windows, that
is an answer and the stub stays with the reason recorded beside it.

**Two of the three were measured on 2026-08-29, and they say the note should not
be written here.** The shell ignores a stream it does not recognise — a script
carrying only `com.excelano.slipcase.origin` runs under `RemoteSigned` where the
same script carrying `ZoneId=3` is refused — which is the answer that was wanted
and matches `carries_a_mark` disregarding the note on macOS. Survival is
identical to `Zone.Identifier` through `Copy-Item`, `Move-Item`, `robocopy` and
`xcopy`, and both are stripped by a `Compress-Archive` round trip.

The exception is the finding: **`Unblock-File` removes the zone stream and
leaves the note.** So a note would survive the one erasure that is deliberate —
a person saying they have looked at a file and trust it — while being stripped
by the ones that are accidents. `arrived_from_elsewhere` consults the note on
macOS, so the card would go on saying a container arrived from elsewhere after
its owner cleared the mark, with nothing in this application's interface to
clear. That is not the case macOS was solving: there the platform forced its own
mark over ours, and here nothing removes the information except a person choosing
to.

**So the stub stays, with this as the reason.** The third question — whether a
packaged install sees such a stream at all — is unmeasured and is in
`CHECKLIST.md` under Windows as the one item still wanting a hand, because it
needs an MSIX install. It does not change the recommendation; it would only
change how confidently the stub's comment can be written.

**Everyone: the metadata tree showed a payload name unescaped, under a card that
escaped it. Settled.** Found on Windows on 2026-08-29 while running the card's
item 3, and it is here rather than in the Windows section because the code is
`src/tree.rs` and all three platforms had it.

`slpc::display_name` escapes the characters SPEC §3 requires be escaped wherever
a name is shown, and `src/main.rs` puts the card's payload name through it. The
tree rendered every string straight into a `TextEdit`, so `payload.file` —
disabled by `is_protected`, a display rather than a field — read `reportfdp.exe`
for a payload called `report<U+202E>fdp.exe`. egui gives the override zero
advance width, so the tree was drawing the spoof the escaping exists to prevent,
in the one field this application will not let anybody change.

**Item 3 asks about the card, so macOS and Linux ticked it correctly and neither
was pointed two rows down.** That is the argument for running a hand item on
every arm even where the code is shared: the value is in what the item does not
say, and the third pair of eyes is where that shows up.

Fixed the same day in `displayed` — a protected string is escaped, an editable
one is not, because escaping a field somebody can type into writes the escape
back into their document the moment it is touched. `DESIGN.md` §4 is amended,
`CHECKLIST.md` holds the run, and both tests were broken deliberately to check
they bite.

**Everyone: a conformant container can name its payload something that is not
a file, and extraction hung on it. Settled.** Found on Windows on 2026-08-26, the first
time the conformance corpus had been run on that platform, and it is here
rather than in the Windows section because the code is `src/lib.rs` and the
decision is not one arm's to take.

`accept/payload-name-windows-reserved` names its payload `CON`. The container
is conformant, the manifest expects `accept`, and `SPEC.md` §2.3 has a
non-normative note about the difference, so nothing is wrong with the corpus or
the library. Win32 resolves `CON` to the console device wherever the name
appears, so `into.join(payload_name())` is not a path in that directory — it is
the console. `File::create` returns `Ok`, `write_all` returns `Ok`, no file
exists afterwards, `metadata` is error 87, and `std::fs::read` never returns.
The corpus reads the payload back, so it stops there with no output and no CPU;
it was killed at ten minutes twice before the case was identified. `LPT1` fails
cleanly with `NotFound` and `NUL` succeeds and discards, so there is no one
behaviour to code against.

`src/lib.rs:97` reasons that joining a checked name onto a directory cannot
leave that directory. That is true and it is not the question. The exposure is
the Open button, which is the caller that takes the name from the container:
extraction writes nothing, and `opener::open` is handed the console device.
Whether the window hung the way the corpus did was never measured, because the
repair below landed first.

**Repaired the same day, and the repair was none of the three this note first
set out.** Refusing the name in the library, renaming the payload, and refusing
with a sentence are all ways of deciding `CON` is a name this build will not
honour. Windows looks for those device names while it *parses* a path, and a
path in the `\\?\` verbatim form is not parsed that way — so the name stops
being a device without anybody calling it a bad one. `fs::canonicalize` answers
in that form, so `extract` asks it of the directory and joins the container's
name onto the answer. `CON`, `CON.txt`, `con`, `COM1`, `AUX`, `LPT1`, `PRN` and
`NUL` then all wrote, read back byte for byte, carried a `Zone.Identifier`
stream and were removable, exactly as an ordinary name does. **The corpus
passes all 77 on Windows, which it had never done.** Nothing holds a list of
reserved names: the prefix is asked of the directory, so which names are
devices stays Windows's to know.

Three things a reader should not have to rediscover. It took two changes, not
one — with `extract` repaired the run still hung, because `src/bin/corpus.rs`
builds a path out of the payload name too, so the two share one function. The handover is still impossible and now says so:
`opener::open` on such a file returns *the specified device name is invalid*,
an error the application already has a sentence for, where before there was a
hang or a silence. And it cost one thing, paid separately: the path handed back
is the verbatim one because that is what addresses the file, so one function
takes the prefix off for display and nothing else does — an existing test caught
*Extracted to* about to show somebody `\\?\C:\…`, which is how that was noticed
rather than shipped.

Both functions have since moved into `slpc` 0.3.5 as `payload_path` and
`display_path`, with `provenance`, and neither is in this repository now.

`DESIGN.md` §8 carries both amendments, the second recording that the decision
went elsewhere, and `CHECKLIST.md` holds the run. What is still unmeasured is
the window: everything above was measured below it, and a container with a
`CON` payload has not been opened in the running application. That is a hand
item rather than a doubt about the repair.


Two things were found on Linux while reviewing the macOS work, and neither
belongs to Linux. They are here because this file is the only way they reach
the session that owns the arm. Both have since been settled and are left below
with their answers, because what the review cost and what it caught are both
worth knowing before the next one.

**Windows: `carries_a_mark` asked whether the stream exists, not whether it
gates.** Settled on 2026-08-26, and it was a defect rather than a doubt. In
`src/provenance.rs`, which has since moved into `slpc` 0.3.5 and is
`slpc::provenance` now, both questions were answered by
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

**It was run later that same day and this paragraph never said so.** David made
the choice by hand and `CHECKLIST.md`'s *What the stale UserChoice run found*
has the answer in three rows: a stale key naming a ProgID that no longer exists
loses to the package, a stale key naming one that still exists but points at a
deleted executable **kills the association outright** — *Application not found*,
no picker, no way out — and no key at all loses to the package. Reproduced on
2026-08-28 against the real reserved identity for the middle row.

The paragraph is left standing because the way it went stale is the lesson. Three
documents summarised a question `CHECKLIST.md` had already answered, and stayed
wrong for two days, because a summary is easier to write than to revisit.

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

`.github/workflows/windows.yml` is the third, added on 2026-08-26 for the
reason the other two were: eight `#[cfg(target_os = "windows")]` tests — five in
`opens_with`, two in `lib.rs`, one in `main.rs` — ran on no machine that
anything revisited, and `linux.yml`'s cross-check compiles them without running
them. It found one thing on the day it was written, an assertion in
`extraction_tests` comparing a canonicalised path against a `tempfile` one,
which is false on a runner whose `TEMP` is `C:\Users\RUNNER~1\…`. It runs the
suite, clippy, the corpus with a ten-minute timeout, and a check that the
committed `.ico` still matches its generator. `CHECKLIST.md`'s MSIX section says
what else could go in it and what never can.

`.github/workflows/linux.yml` is the second, and it exists for the ordinary
reason rather than a gap: Linux is the machine everything was written on. Past
the suite and the corpus it does two things the macOS file does not. It builds
the `.deb` and runs the mechanical parts of the Linux walkthrough over it —
ownership, modes, and every hash in `md5sums` agreeing — because that package
shipped without `md5sums` once and a release build passed over it. And it runs
lintian, which was a *report* rather than a gate until its tags had been read,
because failing a build on an answer nobody had triaged would be asserting one.

**Both tags are now decided and the step gates on `error,warning`.** The
package ships a hand-written changelog, because an apt repository is the one
place an installer can find out what changed and `git log` — the record of why
the code is the way it is — is a different document from the record of what
changed for whoever installed it; `build-deb.sh` refuses to build a package
whose changelog names a version other than `Cargo.toml`'s, which is the one
thing generating it from the log would have bought. And it ships
`slipcase-desktop(1)`, because `src/main.rs:181` reads one positional argument
and there is no `--help`, so the page is the only place that argument is
written down. `CHECKLIST.md` carries both decisions, the alternatives rejected,
and the two `info` tags left below the gate on purpose.

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

If you have it, run it before and after your change. Every case must agree.
