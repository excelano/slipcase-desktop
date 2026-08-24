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

## What is not done

**A double-clicked container is refused with an error dialog.** This is a defect
and it is named here rather than hidden. What a person sees is:

> The document "a-pdf.slpc" could not be opened. Slipcase cannot open files in
> the "Slipcase container" format.

The association is not what fails. That dialog names **Slipcase container**,
which is this bundle's own `UTTypeDescription`, so Launch Services resolved the
file to this type and to this application before saying it. Asked directly,
`URLForApplicationToOpenURL` on the file returns `Slipcase.app`.

What fails is delivery. macOS hands an opened document to a running application
as an Apple Event rather than as `argv[1]`, and `main` reads
`std::env::args_os().nth(1)`. Measured: `open some.slpc` launches Slipcase with
**no arguments at all**. AppKit then finds no delegate willing to take the
document, refuses the event, and Finder reports the refusal in the words above.
The application is running behind that dialog, showing its empty state.

An earlier version of this file said the double-click merely opened an empty
window. That was measured from the command line, where the process starts and
the exit status is zero, and it missed the dialog. The empty window is real and
it is the smaller half of what happens.

Receiving it needs `application:openURLs:` on an `NSApplicationDelegate`. winit
0.30.13 installs its own `WinitApplicationDelegate`, which implements
`applicationDidFinishLaunching:` and `applicationWillTerminate:` and nothing
else, and calls `setDelegate` itself, so a second delegate would displace
winit's own. eframe adds nothing. Implementing the method here would mean
`unsafe impl NSApplicationDelegate` in this crate's source, and
`#![forbid(unsafe_code)]` is the rule with no exceptions.

So this is not worked around. It belongs upstream in winit, and until it is
there the association still earns its place: the type resolves, the icon is
declared and registered against it, and opening the application and using its
own Open dialog works.

The bundle itself is sound, and the gap is only in that one delivery path.
`argv` still reaches it when something passes one:

    open -a dist/Slipcase.app --args some.slpc

which loads the container. That is the way to exercise a bundled build while
the Apple Event is unhandled, and it is not a fix: nobody double-clicks with
`--args`.

**Code signing and notarization are out of scope**, and here is the measurement
so the decision has evidence behind it. `spctl -a -t exec` reports `rejected`,
`source=no usable signature`. A bundle built here carries no
`com.apple.quarantine` attribute, so `open` launches it with no prompt, which is
what the build-and-test loop does. A bundle that reached a person by download
would carry that attribute and Gatekeeper would refuse it; on a current macOS
the only way past is System Settings → Privacy & Security → Open Anyway. Any
distribution outside this machine therefore needs a Developer ID signature and
notarization.

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
