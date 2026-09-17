# Submission notes

What a store submission needs from a person and no file supplies: the notes an
App Review or certification reader is handed, the answers a form asks that no
build can give, and the reasoning behind the screenshots.

The listing text itself is not here. It is `store-listing.toml` beside this,
which `ship` checks before the tag and pushes to both stores on every release,
and what a release tells them changed is `release-notes.toml`. A field edited
in this file would reach nobody.

## App Review notes

Slipcase Desktop reads and writes Slipcase containers. No account, no sign-in, no test credentials, and no network connection of any kind are needed to test it.

A container to open is at https://excelano.com/slipcase/quarterly-report.pdf.slpc — a one-page PDF inside a container with metadata rich enough to show every renderer the editor has. Download it and open it, or launch the application and use Open a container. The subject is invented: no real person, organisation, matter or date appears in it. It is built by `packaging/demo-container.sh` in the repository, which pins its archive timestamps so that any machine rebuilds the same bytes.

To exercise the rest: the metadata pane on the right edits every value by its kind, and a save keeps the comments, key order, whitespace and quoting of everything you did not touch. New container... asks which file to put in and where the container should go, writes it, and opens it, so any file you have to hand is enough to make a second container from nothing.

The App Sandbox is on with exactly two entitlements: the sandbox itself and read-write access to user-selected files. A save replaces the file the person chose, staged in the replacement directory macOS provides on the file's own volume and swapped in with one call, so it stays inside that grant. There is no network entitlement and the application makes no network request.

The application declares the Slipcase container type and claims it at rank Owner. This is the format's own application and the declaration below it in the bundle is the exported one, so Owner is the true rank; an application that merely opened somebody else's format would rank Alternate.

Slipcase implements no cryptography and makes no network request. It reads containers whose members may be encrypted and refuses those, which is a different claim from encrypting anything itself. The full privacy statement is at https://excelano.com/legal/#slipcase and the complete source is at https://github.com/excelano/slipcase-desktop.

## Screenshots (Microsoft Store)

**1366 x 768, PNG**, which is the Store's minimum for a desktop screenshot and
is deliberately not exceeded: the application's window at that size looks like a
window, and at 1920 x 1080 it looks like a window with a great deal of nothing
beside it. The display these were taken on is 2560 x 1302, so a larger size is
available if a listing ever wants one — the size is a constant at the top of
`packaging/windows/shots.ps1`.

Taken by `packaging/windows/shots.ps1`, against the **packaged**
application. ~~2026-08-28.~~ **Retaken 2026-08-29 against 0.1.1 and against the
container `demo-container.sh` builds**, which is what makes the paragraph below
true: the first four were of a container that existed only on one machine, and
the sentence claiming otherwise was written the day the script landed. Both the
package and the pictures are of the same build now. The four are:

| Order | File | What it shows |
| --- | --- | --- |
| 1 | `01-light.png` | A conformant container open: the verdict, the card naming the payload, its size and what would open it, the three buttons, and the metadata tree |
| 2 | `02-light-arrived-from-elsewhere.png` | The same container carrying a `Zone.Identifier`, so the card's provenance line reads *This container arrived from elsewhere, and the payload will carry that* |
| 3 | `03-dark.png` | The first again, in dark mode |
| 4 | `04-dark-arrived-from-elsewhere.png` | The second again, in dark mode |

`shots.ps1` names the frames in listing order and takes all four in one run. It
sets the desktop's theme before each launch and reads it back, because the theme
is the desktop's and no click reaches it, and a shot taken after a theme that
did not take is a duplicate of its pair. It marks the second container as
downloaded itself, on a copy, so a rerun does not find the first shot's
container already carrying the stream the second shot is about.

**Light leads, and that is a decision rather than a preference.** The application
follows the system theme, and a fresh Windows 10 or 11 installation runs apps
light — `AppsUseLightTheme` is 1 — so the majority of people looking at this
listing are looking at a light desktop, and a listing whose first picture is dark
shows them something their machine will not give them. The dark pair is kept
because following the theme is worth showing and costs two slots out of ten.

**Both pairs are the same two containers and the same script**, so the only
difference between 1 and 3 is the desktop's theme. That is deliberate: a shopper
comparing them sees the application, not two different demonstrations.

~~**The container in them is a demonstration and is not in this repository.**~~
**It is now, as of 2026-08-29: `packaging/demo-container.sh` builds it.** The
paragraph this replaces described it in prose and said rebuilding it was a few
lines, which was true and was not enough — the four Windows screenshots could
not be reproduced anywhere, macOS had its own still to take, and the website
needed images too. Three people building three containers from one prose
description is three demonstrations that do not look alike, discovered after two
listings are live.

It holds a one-page PDF and a metadata document written to exercise the tree
rather than to be minimal — a string, three dates in two shapes, an array,
integers, a float, a boolean, a nested table and an array of tables — because
the tree is the thing worth photographing and the walkthrough fixtures have
three keys between them. Its subject is invented and names no real person or
organisation.

**The PDF is generated correctly rather than approximately, and that took two
tries.** The first version declared a stream `Length` of 92 over 87 bytes and
carried no cross-reference table at all, and poppler rendered it regardless,
because mainstream viewers repair a broken xref rather than refusing. A payload
that only opens in viewers that repair is not what goes in two store listings.
The script now measures the stream and builds the xref from where the objects
actually landed, and both were checked: 88 declared against 88 written, and
every offset resolving to the object it names.

**What the script will not do is decide whether a screenshot is any good**, and
it says so when it finishes. It guarantees the size, which is the part that gets
an upload refused, and nothing about the composition.

## Screenshots (Mac App Store)

**1440 x 900, PNG**, taken 2026-08-29 by `packaging/macos/screenshot.sh` against
`dist-devid/Slipcase.app` built from `7d38b4f`, using the same two containers
`demo-container.sh` builds. App Store Connect accepts four sizes for macOS —
1280x800, 1440x900, 2560x1600 and 2880x1800 — and 1440x900 is the largest
reachable here: the other two need a backing scale of 2 and no Retina display
has been available on any machine this project has run on.

| Order | File | What it shows |
| --- | --- | --- |
| 1 | `03-light.png` | A conformant container open: the verdict, the card naming the payload, its size and what would open it, the three buttons, and the metadata tree |
| 2 | `04-light-arrived.png` | The same container carrying `com.apple.quarantine`, so the card's provenance line reads *This container arrived from elsewhere, and the payload will carry that* |
| 3 | `01-dark.png` | The first again, in dark mode |
| 4 | `02-dark-arrived.png` | The second again, in dark mode |

The ordering follows Windows' and for the same reason turned the other way up: a
Mac ships light by default, so light leads. Both pairs are the same two
containers and the same script, so the only difference between 1 and 3 is the
desktop's theme.

**These cannot be of the artefact that gets uploaded, and no macOS screenshot
ever will be.** A Mac App Store package cannot be launched anywhere but the Store
or TestFlight — *what a Store-signed build did when it was
launched* has the kernel refusing it — so the closest available is a bundle
signed with a different certificate and built from the same commit. Windows can
photograph its packaged application and this platform cannot, and a reader
comparing the two sections should know the difference is the platform's rather
than an inconsistency in how the two were done.

**The pointer is parked before the shutter**, which Windows established the
expensive way: a shot came back 2292 pixels different from its predecessor with
none of the difference being the change it was taken for, because the pointer was
resting on a field and egui drew it hovered and focus-ringed with the scroll bar
showing.

**The window is photographed by its id rather than by its rectangle**, so
whatever happens to be in front of it stays out of the picture. The first
attempt here used a region and came back as a screenful of terminal.
