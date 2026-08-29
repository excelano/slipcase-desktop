# Store listing text

One draft, used twice. Both stores want the same things at different lengths, so
everything here is written to the shorter limit and the longer one simply has
room to spare.

**One paragraph differs per store.** The description's sentence beginning *On
macOS it tells you when a payload was stored as an executable file* must be cut
from the Microsoft Store listing. That card line is gated to Unix — a permission
bit is not what makes a file executable on Windows, and `DESIGN.md` §5 says so —
so on Windows the sentence describes something a person will never see. Cutting
it costs 190 characters and both descriptions still fit.

**One sentence is deliberately vaguer than it could be, and this is why.** The
provenance paragraph used to promise that a payload *carries that marking onward,
so whatever opens it next raises the same warning the container would have*. That
is exactly true on Windows, where the zone stream is copied verbatim and the shell
stops for the copy as it would have for the container. It is **not** true of a
Mac App Store build: the sandbox marks whatever the process writes and refuses to
have that mark replaced, so the payload reaches its handler carrying this
application's mark rather than the container's. Measured 2026-08-28, and no API
avoids it — a sandboxed process cannot attribute a file to anyone but itself.

Nothing is lost by it. `CHECKLIST.md`'s *What the provenance sitting found*
established that the sandbox's own mark gates at least as hard as the one it
displaces, and harder for anything that executes. So the wording says *treats it
with the caution it gives anything that came from outside*, which is true of both
stores, rather than *the same warning*, which is true of one. **Resisting the
second per-store variant was the point**: the executable-payload sentence below
already differs, and a listing maintained in two shapes drifts in one of them.

**This is generated from `CHANGELOG.md`, not written beside it.** Every claim
below appears there first, checked against the built application rather than
against memory. If the two ever disagree, the changelog is right and this is
stale — that drift is the failure `RELEASE.md`'s readiness review names as the
one this project has caught most often.

Limits, so a later edit does not overrun them:

| Field | Microsoft Store | Mac App Store |
| --- | --- | --- |
| App name | unmeasured | 30 |
| Description | 10,000 | 4,000 |
| Short description | 1,000 | — |
| Subtitle | — | 30 |
| Promotional text | — | 170 |
| Keywords | 7 terms | 100 characters |

The Mac App Store's 30 is the one that binds; the Microsoft Store's limit is far
above anything here and is left unmeasured rather than written down wrong.

---

## App name — and it differs per store

    Microsoft Store   Slipcase
    Mac App Store     Slipcase Desktop

**The application is called Slipcase and goes on calling itself Slipcase.** This
is the storefront name only. `Package/Properties/DisplayName` in
`AppxManifest.xml` and `CFBundleDisplayName` in `Info.plist` both say `Slipcase`
and neither changes: what a person sees in their Dock, their task bar and the
window is unaffected.

**Why the Mac App Store name is different, so nobody rediscovers it.** `Slipcase`
was refused by App Store Connect on 2026-08-28 — *the app name you entered is
already being used*. It belongs to an insurance and reinsurance news application
by Everlution, on the store since 2016, and *slipcase* is a term of art in that
industry from the *slip* that carries a risk to market. They have the better
claim and there is no trademark route here. Partner Center accepted `Slipcase`
the same day, that namespace being Microsoft's, so the reservation there stands.

**`Slipcase Desktop` rather than the alternatives**, and one was rejected for a
reason worth keeping. *Slipcase Viewer* would have read well and would have been
a lie: `CFBundleTypeRole` is `Editor` precisely because this application writes
edited metadata back, and `packaging/macos/README.md` records that `Viewer`
"would be a claim to the platform that is not true". Making the same claim to a
shopper is worse than making it to Launch Services. Renaming the product
everywhere was the other option and costs more than it buys: the Partner Center
reservation would be surrendered, and the *format* would still be called
slipcase, since that is `excelano/slipcase`'s name and not this application's to
change.

It is written here because it has to be typed correctly into four places that
cannot see each other, and two of them now differ: the App Store Connect
reservation (`Slipcase Desktop`), the Partner Center reservation (`Slipcase`),
`Package/Properties/DisplayName` (`Slipcase`), and `CFBundleDisplayName`
(`Slipcase`). Until this section existed the name appeared in this file only
inside the prose below, which is not somewhere a person copies a value from.

`Slipcase Desktop` is 16 characters, inside the Mac App Store's 30.

## Subtitle (Mac App Store, 30)

Metadata that travels along

## Promotional text (Mac App Store, 170)

Open a container, read the metadata travelling with the file inside it, edit that metadata in place, and hand the file to whatever opens it.

## Short description (Microsoft Store, 1,000)

A slipcase container is one file holding a document of any type together with a TOML description of it, so the description travels with the document instead of living in a filename, a sidecar file, or somebody else's database.

Slipcase opens a container and shows what is in it: the payload's name and size, what your computer says would open it, and the metadata as a tree you can edit. Open hands the payload to the application registered for that kind of file. Extract writes it where you choose. Replace swaps it for another file. Save writes edited metadata back.

It tells you what it found and lets you decide. Where your computer will not say what opens a file, Slipcase says nothing rather than guessing. It ships no list matching filenames to file types and never inspects a payload to guess at one.

## Description (both, written to 4,000)

A slipcase container is one file holding a document of any type together with a TOML description of it. Copy the container, send it, or move it to another machine and the description goes too — instead of living in a filename that gets truncated, a sidecar file that gets separated, or a database on somebody else's computer.

Slipcase opens a container and shows what is inside.

WHAT YOU SEE

The payload's name and size. What your operating system says would open it. The metadata as a tree, with every value editable in place. And the container's verdict against the specification, in the specification's own words — including the two answers that are neither a pass nor a failure: a container whose metadata cannot be read is undetermined, and one written to a newer version of the format is out of scope.

WHAT YOU CAN DO

Open hands the payload to whatever application is registered for that kind of file. Extract writes it somewhere you choose. Replace swaps it for a different file. Save writes an edited metadata document back — keeping your comments, your key order, and any whitespace you did not touch, along with anything else in the container that Slipcase does not recognise.

A rewrite is read back and checked before it replaces anything, so a save that would produce a container the format does not accept changes nothing on disk. A container you did not change is not rewritten at all.

WHAT IT TELLS YOU, AND DOES NOT DECIDE FOR YOU

Slipcase reports. It does not gate.

If a container arrived from elsewhere — downloaded, or sent to you — Slipcase says so, and the payload you extract is marked as well, so your computer treats it with the caution it gives anything that came from outside rather than opening it as though you had made it yourself. Editing the metadata and saving does not quietly erase that: Slipcase still tells you where the container came from afterwards.

On macOS it tells you when a payload was stored as an executable file, and that the copy you extract will not be. That is read out of the container rather than guessed from its name.

Names are shown with the characters that reorder text written out, so a payload cannot present itself as one kind of file while being another.

Where your operating system will not say what opens a payload, Slipcase says nothing rather than guessing. It ships no table matching filenames to types and never inspects a payload's contents to guess at one. What you get is information — where the file came from, what it was, whether the container is well formed — and the decision stays yours.

WHAT IT DOES NOT DO

No network connection of any kind. No account. No telemetry, no analytics, no crash reporting. Nothing about you or your files is sent anywhere, because there is nowhere for it to be sent.

It shows no preview: the payload is handed to another application rather than rendered here. A container holds one payload, which is the format's decision rather than this application's.

OPEN SOURCE

Slipcase is open source, and so is the format it reads and the library that reads it. Every claim above is checkable: github.com/excelano/slipcase-desktop.

## Keywords

**Mac App Store** (100 characters, comma-separated, no spaces after commas):

    metadata,container,slpc,archive,toml,file,document,tagging,zip,viewer

**Microsoft Store** (seven terms):

    metadata, container, slpc, TOML, archive, document, file viewer

## Screenshots (Microsoft Store)

**1366 x 768, PNG**, which is the Store's minimum for a desktop screenshot and
is deliberately not exceeded: the application's window at that size looks like a
window, and at 1920 x 1080 it looks like a window with a great deal of nothing
beside it. The display these were taken on is 2560 x 1302, so a larger size is
available if a listing ever wants one — `screenshot.ps1` takes `-Width` and
`-Height`.

Taken by `packaging/windows/screenshot.ps1`, against the **packaged**
application. ~~2026-08-28.~~ **Retaken 2026-08-29 against 0.1.1 and against the
container `demo-container.sh` builds**, which is what makes the paragraph below
true: the first four were of a container that existed only on one machine, and
the sentence claiming otherwise was written the day the script landed. Both the
package and the pictures are of the same build now. The four are:

| Order | File | What it shows |
| --- | --- | --- |
| 1 | `03-light.png` | A conformant container open: the verdict, the card naming the payload, its size and what would open it, the three buttons, and the metadata tree |
| 2 | `04-light-arrived-from-elsewhere.png` | The same container carrying a `Zone.Identifier`, so the card's provenance line reads *This container arrived from elsewhere, and the payload will carry that* |
| 3 | `01-window.png` | The first again, in dark mode |
| 4 | `02-arrived-from-elsewhere.png` | The second again, in dark mode |

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
