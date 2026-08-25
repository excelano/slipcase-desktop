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

`UTExportedTypeDeclarations` says what a slipcase container is.
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

**`mdls` disagrees and is not the authority here.** After registration,
`mdls -name kMDItemContentType some.slpc` still reports the dynamic
`dyn.ah62d4rv4ge81g5duqq`, on a file created after registration and after
`mdimport` was run against it by hand. Launch Services resolves the extension to
`com.excelano.slipcase` at the same moment, with `isDeclared` true and
`isDynamic` false. The registration record carries an `untrusted` flag, which is
what an unsigned bundle's exported type gets, and that is the likeliest reason
Spotlight will not take it. Untested, because testing it needs a signature. If
you sign the bundle, check `mdls` again and record what changed.

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

## What is not done

**Code signing and notarization are out of scope**, and here is the measurement
so the decision has evidence behind it. `spctl -a -t exec` reports `rejected`,
`source=no usable signature`. A bundle built here carries no
`com.apple.quarantine` attribute, so `open` launches it with no prompt, which is
what the build-and-test loop does. A bundle that reached a person by download
would carry that attribute and Gatekeeper would refuse it; on a current macOS
the only way past is System Settings → Privacy & Security → Open Anyway. Any
distribution outside this machine therefore needs a Developer ID signature and
notarization.

## What a Store build would need

Signing is out of scope above as a *decision about this machine*, not as a
verdict on the channel. The Mac App Store is under consideration because it is
how a person who has been sent a container finds something to open it: Finder
offers *Search App Store* by document type, and outside the Store that search
returns nothing. What follows is what such a build would need, written down
before it is attempted so the cost is known rather than discovered.

**The sandbox is the gate and it is measured elsewhere.** Every Store binary is
sandboxed, and `CHECKLIST.md` holds the three paths in this application that
run at that wall — the in-place save, the handover through `opener`, which is
`Command::new("open")` and so is denied outright, and the `com.apple.quarantine`
write that extraction fails hard on. Nothing below is worth doing until those
three have been run. None of them needs a distribution certificate: an Apple
Development certificate signs a bundle with entitlements perfectly well for
local testing, and the sandbox is inert until the entitlement is inside a
signature.

**The account exists; nothing macOS does.** Team `9K6W5PMFYP` already ships two
iOS applications, so App Store Connect, the agreements, and the tax and banking
side are done. Absent on this machine are an Apple Distribution certificate, a
Mac Installer Distribution certificate, a macOS App ID for
`com.excelano.slipcase-desktop`, and a Mac App Store provisioning profile,
which a Store bundle carries as `embedded.provisionprofile` and which must
declare the same entitlements the signature does.

**No Xcode project is needed and none should be added.** `build-app.sh`
assembles the bundle; a Store submission is that bundle signed with the
entitlements and the profile, wrapped by `productbuild --component` into a
package signed with the installer certificate, and uploaded with Transporter.
The build stays a shell script, which is the point of it.

**One property list key is missing.** App Store Connect requires
`LSApplicationCategoryType` and `Info.plist.in` does not carry one;
`public.app-category.utilities` is the honest fit. It is not added here,
because a bundle should not claim a channel that has not been chosen.

**The architecture is a real problem.** This machine is `x86_64` and
`aarch64-apple-darwin` is not installed. An Intel-only binary on the Store means
every Apple silicon buyer runs under a translation layer Apple is winding down,
so a Store build wants a universal binary: the second target, and a `lipo` step
in `build-app.sh`. Nothing here compiles C, so the cross-build should be
uneventful — but the arm64 slice cannot be *run* on this machine, and the one
platform-specific module in the crate is the Objective-C one, which is exactly
the code least safe to ship untested.

**Two things already argue well at review.** `DESIGN.md` §3 refuses to open a
payload automatically on a double-click, which is the behaviour an autorun
archive would have and the behaviour review exists to catch, and `§5` carries
the container's provenance onto the payload rather than laundering it.

A Developer ID `.dmg` and a Store build ship from one codebase, so this is not
a fork in the road.

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
