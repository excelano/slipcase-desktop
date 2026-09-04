# macOS

`DESIGN.md` §8. The application bundle, the exported type declaration, and the
icon. Two files here and everything else is generated:

    Info.plist.in    the bundle's property list, with @VERSION@ substituted
    build-app.sh     assembles dist/Slipcase.app

Build it, then register it:

    cargo build --release
    ./packaging/macos/build-app.sh
    lsregister -f dist/Slipcase.app

`lsregister` is not on `PATH`. It lives at
`/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister`,
and `build-app.sh` prints the full line. Moving the bundle into `/Applications`
registers it without the command; `lsregister -f` is how it is done from a build
directory. `lsregister -u` unregisters, which matters because `dist/` is ignored
by git and a deleted bundle otherwise leaves a claim behind it.

## The bundle is the unit

A bare executable draws a window on macOS and this application was first run
here that way, but it is not a thing the platform can associate anything with.
`lsappinfo` reports `bundleID=[ NULL ]` and `bundle path=[ NULL ]` for one, and
Launch Services files it as a nameless foreground process. Nothing below works
without the bundle.

## Two declarations, not one

`UTExportedTypeDeclarations` says what a Slipcase container is.
`CFBundleDocumentTypes` says that this application opens it. A bundle carrying
only the second claims a type it never defined; one carrying only the first
defines a type nothing opens.

The declaration is **exported** rather than imported because this application
defines the type: `SPEC.md` §4 in `excelano/slipcase` is the authority. The
extension `slpc` and the media type `application/x.slipcase+zip` are both taken
from there and neither is invented here. There is no magic-bytes tag, for the
same reason the Linux media type carries no `<magic>`: SPEC §4 reserves none.

Conformance is to `public.zip-archive` and `public.data`. The first is the macOS
half of what `sub-class-of application/zip` does on Linux and is true for the
same reason: a container is a ZIP. The second is implied by the first and is
stated anyway, so that a reader does not have to walk Apple's conformance tree
to learn that a container is a file of bytes.

## Two identifiers

`com.excelano.slipcase` names the **format**. `com.excelano.slipcase-desktop`
names the **application**. They are different things and conflating them would
make the bundle identifier change if the format were ever renamed.

The application's identifier is the reverse-DNS of `APP_ID` in `src/main.rs`, so
the binary, the Linux desktop entry's basename, and this bundle all say
`slipcase-desktop`. `APP_ID` itself needs nothing from macOS: it exists so a
Wayland compositor can find the window's icon, and the equivalent here is
`CFBundleIconFile`, which the bundle carries.

## The role is Editor

`CFBundleTypeRole` is `Editor` rather than `Viewer`. `DESIGN.md` §5 writes
edited metadata back into the container, so this application does modify the
documents it opens and `Viewer` would be a claim to the platform that is not
true. The cost is that macOS offers Slipcase in more places, and it is bounded:
the only type claimed is the one this bundle exports, so nothing but a `.slpc`
is affected.

`LSHandlerRank` is `Owner` for the same reason the declaration is exported.

## Checking that it took

    lsregister -dump | grep -A6 'com.excelano.slipcase'

which should report the type as `exported`, conforming to `public.zip-archive`
and `public.data`, tagged `.slpc` and `application/x.slipcase+zip`.

Asking through the application's own code is the better check, because it is the
path a person actually sees:

    cargo run --example opens-with -- some.slpc report.pdf

Before the bundle is registered this prints nothing for `some.slpc`, because
macOS synthesises a dynamic type for an extension nothing declares and nothing
claims a synthesised type. After it is registered it prints `Slipcase`, by the
same code that prints `Preview` for the PDF: `display_name` reads
`CFBundleDisplayName`, and this bundle's is `Slipcase`.

**Resolved: `mdls` agrees once the bundle is signed.** Measured 2026-08-25
against a bundle signed with an Apple Development certificate: `lsregister
-dump` flags the type `trusted` where it flagged it `untrusted` before, `mdls
-name kMDItemContentType` reports `com.excelano.slipcase`, and `kMDItemKind`
reports `Slipcase container`. The paragraph below was right about the cause and
is kept because it is the measurement that made the suspicion worth having. A
distribution certificate is not needed for this — any signature is.

**`mdls` disagreed while the bundle was unsigned, and was not the authority
then.** After registration,
`mdls -name kMDItemContentType some.slpc` still reports the dynamic
`dyn.ah62d4rv4ge81g5duqq`, on a file created after registration and after
`mdimport` was run against it by hand. Launch Services resolves the extension to
`com.excelano.slipcase` at the same moment, with `isDeclared` true and
`isDynamic` false. The registration record carries an `untrusted` flag, which is
what an unsigned bundle's exported type gets, and that is the likeliest reason
Spotlight will not take it. That was untested when it was written, and the
paragraph above is what checking it found.

## The double-click, and the three attempts it took

**A double-clicked container opens.**
This section used to say it could not be done. It could.

macOS does not deliver an opened document as `argv[1]` the way Linux and
Windows do; it launches the application with no arguments at all and sends an
Apple Event. Nothing was listening, so `AppKit` refused it and Finder reported
*The document could not be opened. Slipcase cannot open files in the "Slipcase
container" format* — an accusation against an application whose association was
correct all along.

`src/opened_document.rs` listens now. It handles the event rather than
replacing the application delegate: `NSApplication` has exactly one delegate and
winit owns it, but `NSAppleEventManager` takes a handler directly and a
notification has any number of observers, so nothing of winit's is displaced.
It is the one module in this application that writes `unsafe`, and `CLAUDE.md`
says what that costs.

**The moment of registration is the whole problem.** Two of the three plausible
moments fail, and both were measured rather than reasoned about:

| Registered | Cold launch | Double-clicked into a running window |
|---|---|---|
| Before `NSApplication` exists | refused | refused |
| At `applicationWillFinishLaunching:` | **opens** | **opens** |
| From `eframe`'s creation closure | refused | opens |

The first fails because `AppKit` installs its own handler for this event while
starting up and overwrites anything earlier — and its handler is the one that
refuses. The third fails because the launch document has already been dispatched
and refused by then. An instrumented run showed the handler firing *before*
`eframe`'s creation closure was reached, which is the ordering that explains
both. The middle row is where Apple's own documentation says to install Apple
Event handlers, and it is reached here through an observer on
`NSApplicationWillFinishLaunchingNotification`.

A container opened this way also records its folder for the Open dialog, the
same as one chosen in the dialog, because to a person it is the same act.

## Signing

`build-app.sh --sign IDENTITY` signs the finished bundle with
`Slipcase.entitlements` beside it. It is the last thing the script does,
because a signature covers what is in the bundle when it is made and anything
added afterwards is how a bundle becomes one macOS calls damaged. The script
then reads the entitlements back out of the signature and refuses a bundle
whose signature does not carry the sandbox — a signature that quietly dropped
the entitlements produces a bundle that launches, behaves exactly like an
unsigned one, and makes every sandbox measurement taken against it meaningless.

**Signing is not optional here, and not only for distribution.** The App
Sandbox is inert until the entitlement is inside a signature, so an unsigned
bundle carrying that file is simply not sandboxed. Every sandbox measurement in
`CHECKLIST.md` was made against a signed bundle for that reason.

**Which certificate does what.** An **Apple Development** identity, which this
machine already holds, signs a bundle that runs and sandboxes here — enough for everything measured so far, and it needs nothing from the
developer portal. An **Apple Distribution** identity is what a Store upload must
be signed with, and a **Mac Installer Distribution** identity signs the package
that carries it. Neither is on this machine yet and both are created in the
account that already ships two iOS applications. A **Developer ID Application**
identity is a third thing again, for distributing outside the Store, and only
that path involves notarization: a Store submission is reviewed rather than
notarized.

**What an unsigned bundle did, kept because it is why the above matters.**
`spctl -a -t exec` reported `rejected`, `source=no usable signature`. A bundle
built here carries no `com.apple.quarantine` attribute, so `open` launched it
with no prompt, which is what made the unsigned build-and-test loop possible at
all. A bundle that reached a person by download would carry that attribute and
Gatekeeper would refuse it, with System Settings → Privacy & Security → Open
Anyway the only way past.

## What a Store build is

It exists and it ships. `build-app.sh --store PROFILE` produces it and
`RELEASE.md` has the process; what belongs here is why it is shaped that way.

**The sandbox is the gate**, and it is not a formality. Every Store binary is
sandboxed, and the sandbox is inert until the entitlement is inside a signature
— so an unsigned bundle carrying the entitlements file is simply not sandboxed,
and every measurement taken against one is meaningless. Signing is therefore not
only a distribution concern here. An Apple Development certificate is enough to
measure with; a distribution certificate is only needed for the upload itself.

**Three paths were measured against a real sandbox rather than reasoned about,
and the prediction was wrong in both directions.** The handover survives:
`opener` forks `/usr/bin/open`, which this file once said the sandbox would deny
outright, and it does not — exec is permitted, the child inherits the sandbox,
and Launch Services is reachable over Mach IPC from inside it. The save did not:
`Destination::in_place` creates a randomly-named sibling of the container, and
the grant a person gives through the open panel covers the file rather than its
directory. That is what `src/staging.rs` exists for, and `DESIGN.md` §5 carries
the reasoning.

**The Store entitlements are not the development ones.**
`packaging/macos/Slipcase.entitlements` holds the sandbox and user-selected
files, which is right for a development build and not enough for an upload: a
Store build also needs `com.apple.application-identifier` and
`com.apple.developer.team-identifier`, which the provisioning profile grants.
The profile grants `keychain-access-groups` as well and it is declined, because
a capability asked for and unused is a question at review with no good answer.

**A Store-signed bundle cannot be launched here.** AMFI refuses a restricted
entitlement without a profile covering the machine, and a Mac App Store profile
covers none, so anything needing a running application uses a Developer ID build
and the real article is reached through TestFlight.

**And Launch Services will launch it anyway, if it is allowed to know about it.**
It does not ask whether a bundle can run before choosing it as a handler, and
among copies of one identifier it prefers the newer version. `--store` writes to
the same `dist/` that a development build does, the `lsregister -f` above leaves
a claim on that path, and the claim survives the rebuild — so once the installed
copy is older than the submission build, or absent, every double-click on a
container launches the submission build and the kernel kills it on the spot:
`SIGKILL (Code Signature Invalid)`, `Taskgated Invalid Signature`, one crash
report per attempt and no window. Measured 2026-09-04, with the Store copy in
the Trash. `build-app.sh --store` now withdraws the claim with `lsregister -u`
as its last step, and that holds: Spotlight indexing the bundle did not register
it over three minutes, only a deliberate hand-off did. A bundle copied aside
(`dist-refused/` is one) keeps whatever claim it had, so unregister it by hand.
If a container opens Archive Utility, that is the *absence* of a claim — the
Store copy is not installed — rather than a defect here.

## No private symbol reaches the binary

Guideline 2.5.1, and it cost a review cycle. The submission was refused for
referencing `_CGSSetWindowBackgroundBlurRadius`, which nothing here calls: it is
`winit`'s, declared in `platform_impl/macos/ffi.rs` and called from
`WindowDelegate::set_blur` with neither a feature nor a `cfg` in front of it.
**Review reads the symbol table and not the call graph**, so unreachable is not
absent, and every macOS binary this project has built carried it.

Three things were eliminated by measurement before anything was forked. Nothing
here reaches blur — `egui-winit` has no reference to it and eframe's only one is
in `src/web/`. A build flag does not remove it: fat LTO with `-Wl,-dead_strip`
still carries the symbol, the call sitting behind a runtime `if attrs.blur` the
optimizer will not fold. And the other CoreGraphics symbol in that file is fine,
`CGShieldingWindowLevel` being declared in the public `CGDirectDisplay.h` while
the two `CGS` ones appear in no header at all — which is the whole definition of
private, and the line the check below draws.

`Cargo.toml`'s `[patch.crates-io]` removes it and says when to delete itself.

**`build-app.sh` refuses to bundle an executable that imports a symbol from a
system framework which that framework's own public headers do not declare.** A
list of names Apple has already caught somebody with would have found nothing
here until after the rejection, so it asks a question instead. Frameworks only:
libSystem and libobjc are the compiler's own runtime, and asking about them
produced a dozen findings that were all noise. The whole `.framework` is
searched rather than its `Headers`, because Carbon and CoreServices are
umbrellas and scoping to `Headers` reported five false positives from those two
alone. It also refuses a binary `nm` read no imports from, an empty list being
`nm` having failed rather than a clean answer.

It was watched to fail on the refused executable before it was believed.
`apple-silicon.yml` already calls `build-app.sh`, so it runs on every push.

## The icon

One drawing, `packaging/linux/icons/slipcase-desktop.svg`, which
`packaging/README.md` names as the source for every platform. Linux carries the
application icon and the document icon as two files with byte-identical content;
macOS needs one `.icns` and both `CFBundleIconFile` and `UTTypeIconFile` point
at it.

`build-app.sh` renders the ten sizes `iconutil` wants using `sips`, which reads
SVG. It rasterizes at whatever width and height the document declares and then
resamples to the size asked for, so rendering the 64-unit source at 1024 gives a
soft upscale of a 64-pixel bitmap rather than a large clean drawing. The script
rewrites the declared size before each rendering, which makes every size a true
rendering at that size and matches how the Linux icon theme draws the same file.
Both ways were compared at 16 pixels and they are near-identical there; the
native rendering was taken because it needs no intermediate.

That rewrite is a substitution on the SVG's `width` and `height` attributes and
would fail silently if they were ever written differently, leaving every icon a
64-pixel upscale. `build-app.sh` checks the size of each rendering and stops if
it is not what was asked for.

## The minimum system version is 12.0 and it is measured

`Info.plist` declares `LSMinimumSystemVersion` 12.0 because
`src/opens_with.rs` calls `-[NSWorkspace URLForApplicationToOpenContentType:]`,
which is macOS 12 and later, and uses `UTType`, which is macOS 11 and later.
Below 12 that selector does not exist and asking what opens a payload would
abort.

The declaration binds the bundle: Finder will not launch it on an older system.
It does not bind the bare executable, which Cargo builds with a deployment
target of 10.12 by default. Building the executable that goes into a released
bundle with the floor set makes the two agree:

    MACOSX_DEPLOYMENT_TARGET=12.0 cargo build --release

This is not needed to build or test on a machine running 12 or later, which is
every machine that can run this at all.
