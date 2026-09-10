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

Nothing is lost by it. The sandbox's own mark was measured gating at least as
hard as the one it displaces, and harder for anything that executes. So the wording says *treats it
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
| Short description | 500 | — |
| Subtitle | — | 30 |
| Promotional text | — | 170 |
| Keywords | 7 terms | 100 characters |

The Mac App Store's 30 binds the subtitle. **The short description's 500 is the
submission API's, measured on Duckling 2026-09-09** - Partner Center's form takes
1,000 and this listing carried 811 for a year, which is why 0.1.6 rewrites it: the
API copies the published listing into every new submission and refuses it over 500,
so the old text would block an upload before it started. fenster's `LIMITS.md`
carries the measurement and `check-listing.ps1` is what catches it.

---

## App name — and it differs per store

    Microsoft Store   Slipcase
    Mac App Store     Slipcase Desktop

**The application ships as Slipcase and is marketed as Slipcase Desktop.**
`Package/Properties/DisplayName` in `AppxManifest.xml` and `CFBundleDisplayName`
in `Info.plist` both say `Slipcase` and neither changes: what a person sees in
their Dock, their task bar and the window is unaffected. What did change, on
2026-09-02, is everything a reader meets before installing — `excelano.com/slipcase/`
now says Slipcase Desktop throughout, because the *format* took the capital that
same day and a page that calls both of them Slipcase cannot be read. The
Microsoft Store listing stays `Slipcase`.

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
Slipcase, since that is `excelano/slipcase`'s name and not this application's to
change.

It is written here because it has to be typed correctly into five places that
cannot see each other, and they do not all agree: the App Store Connect
reservation (`Slipcase Desktop`), the Partner Center reservation (`Slipcase`),
`Package/Properties/DisplayName` (`Slipcase`), `CFBundleDisplayName`
(`Slipcase`), and the product page at `excelano.com/slipcase/` (`Slipcase
Desktop`). Until this section existed the name appeared in this file only
inside the prose below, which is not somewhere a person copies a value from.

`Slipcase Desktop` is 16 characters, inside the Mac App Store's 30.

## Subtitle (Mac App Store, 30)

Metadata that travels along

## Promotional text (Mac App Store, 170)

Open a container, read the metadata travelling with the file inside it, edit that metadata in place, and hand the file to whatever opens it.

## Short description (Microsoft Store, 500)

A Slipcase container is one file holding a document together with the metadata that describes it, so the metadata travels with the document instead of living in a filename, a sidecar, or somebody else's database.

Slipcase makes containers and opens them: the payload, what would open it on your computer, and the metadata as a tree you can edit, with Open, Extract, Replace and Save. Where your computer will not say what opens a file, Slipcase says nothing rather than guessing.
## App features (Microsoft Store, up to 20 bullets of 200 characters)

    Metadata travels with the document: one file holds the payload and the metadata describing it.
    Makes a container out of a file you choose, then opens it so the metadata is filled in where the rest gets edited. A large payload packs with a progress bar and a Stop.
    Edit the metadata in place and save. Comments, key order and whitespace you did not touch survive the rewrite.
    Hands the payload to whatever your computer has registered for that kind of file. No preview, no guessing at types.
    Says when a container arrived from elsewhere, and marks the payload you extract so your computer treats it with the same caution.
    Reports the container's verdict against the specification in its own words, including undetermined and out of scope.
    A rewrite is read back and checked before it replaces anything, and a container you did not change is not rewritten at all.
    No network connection of any kind. No account, no telemetry, no analytics, nothing sent anywhere.
    Open source, and so are the format it reads and the library that reads it.

**Every bullet restates something the description already says**, which is the
point rather than an oversight: the Store shows these as a summary beside the
description, and a feature list making a claim the description does not is a
second listing to keep true.

**Written during the first submission, and it should not have been.** This
section did not exist on 2026-08-29 and the bullets were typed into the form,
which is exactly the drift the note at the top of this file forbids. They are
here now so the next submission copies them. There is no Mac App Store
equivalent of this field.

## Description (both, written to 4,000)

A Slipcase container is one file holding a document of any type together with metadata describing it. Copy the container, send it, or move it to another machine and the metadata goes too — instead of living in a filename that gets truncated, a sidecar file that gets separated, or a database on somebody else's computer.

Slipcase opens a container and shows what is inside, and makes one out of a file you choose.

WHAT YOU SEE

The payload's name and size. What your operating system says would open it. The metadata as a tree, with every value editable in place. And the container's verdict against the specification, in the specification's own words — including the two answers that are neither a pass nor a failure: a container whose metadata cannot be read is undetermined, and one written to a newer version of the format is out of scope.

WHAT YOU CAN DO

Open hands the payload to whatever application is registered for that kind of file. Extract writes it somewhere you choose. Replace swaps it for a different file. Save writes an edited metadata document back — keeping your comments, your key order, and any whitespace you did not touch, along with anything else in the container that Slipcase does not recognise.

A rewrite is read back and checked before it replaces anything, so a save that would produce a container the format does not accept changes nothing on disk. A container you did not change is not rewritten at all.

New container asks which file to put in and where the container should go, writes it, and then opens it, so the metadata is filled in where the rest of it gets edited. A large payload packs with a progress bar and a Stop, and stopping leaves nothing behind. A file the format will not accept as a payload is refused when you choose it, rather than after you have said where the container goes.

WHAT IT TELLS YOU, AND DOES NOT DECIDE FOR YOU

Slipcase reports. It does not gate.

If a container arrived from elsewhere — downloaded, or sent to you — Slipcase says so, and the payload you extract is marked as well, so your computer treats it with the caution it gives anything that came from outside rather than opening it as though you had made it yourself. Editing the metadata and saving does not quietly erase that: Slipcase still tells you where the container came from afterwards. It travels the other way too: a file that arrived from elsewhere still says so on the container you pack it into.

On macOS it tells you when a payload was stored as an executable file, and that the copy you extract will not be. That is read out of the container rather than guessed from its name.

Names are shown with the characters that reorder text written out, so a payload cannot present itself as one kind of file while being another.

Where your operating system will not say what opens a payload, Slipcase says nothing rather than guessing. It ships no table matching filenames to types and never inspects a payload's contents to guess at one. What you get is information — where the file came from, what it was, whether the container is well formed — and the decision stays yours.

WHAT IT DOES NOT DO

No network connection of any kind. No account. No telemetry, no analytics, no crash reporting. Nothing about you or your files is sent anywhere, because there is nowhere for it to be sent.

It shows no preview: the payload is handed to another application rather than rendered here. A container holds one payload, which is the format's decision rather than this application's.

OPEN SOURCE

Slipcase is open source, and so is the format it reads and the library that reads it. Every claim above is checkable: github.com/excelano/slipcase-desktop.

## Release notes

*What's new in this version* on the Microsoft Store and *What's New* on the Mac
App Store, one version's text each, written from `CHANGELOG.md` and kept latest
first. Read it for the person it reaches: **the Store serves 0.1.2**, so what a
customer upgrading from it meets is everything since, and 0.1.3 and 0.1.4 are
not in it because one was a Mac submission fix and the other was Linux
packaging - neither changed anything a person on this store can see.

### 0.1.7

Slipcase speaks German. On a machine set to German the window comes up in German: the bar, the card, the messages after a save or an extraction, the file dialogs and the metadata tree. There is nothing to choose — it reads the language the desktop is already set to, and falls back to English for every other one. A container's verdict stays in English, because that sentence is the format library's own.

Slipcase makes containers now. *New container...* asks which file to put in and where the container should go, writes it, and opens it, so the metadata editor is where you fill it in. A large payload packs with a progress bar and a Stop, and stopping leaves nothing behind. A file the format will not allow as a payload is refused when you choose it, not after you have said where the container goes.

The metadata editor is the general one shared with Tommy Flyleaf. Every value has a kind menu offering the conversions it allows, arrays are editable, comments can be edited, added and removed, and undo and redo run through all of it. A save still keeps the comments, key order, whitespace and quoting of everything you did not touch.

The buttons and the file dialog say "Open a container", now that the format is spelled Slipcase.

### 0.1.6

Slipcase makes containers now. *New container...* asks which file to put in and where the container should go, writes it, and opens it, so the metadata editor is where you fill it in. A large payload packs with a progress bar and a Stop, and stopping leaves nothing behind. A file the format will not allow as a payload is refused when you choose it, not after you have said where the container goes.

The metadata editor is the general one shared with Tommy Flyleaf. Every value has a kind menu offering the conversions it allows, arrays are editable, comments can be edited, added and removed, and undo and redo run through all of it. A save still keeps the comments, key order, whitespace and quoting of everything you did not touch.

The buttons and the file dialog say "Open a container", now that the format is spelled Slipcase.

## URLs

Both forms ask for the same three, and both lanes take them from here:

| Field | URL |
| --- | --- |
| Privacy policy | https://excelano.com/legal/#slipcase |
| Support | https://excelano.com/slipcase/#support |
| Marketing / website | https://excelano.com/slipcase/ |

The page at `excelano.com/slipcase/` is the support and marketing URL both. Its
*Support* heading is the anchor, and the anchor was read back off the served
page on 2026-09-09 rather than taken from this file — Duckling's support URL
was a 404 on the day its form was filled, which is the failure this line
exists to prevent. The legal page's Slipcase section is
`packaging/privacy-entry.html` as pasted, and it is the answer to both stores'
privacy question.

## App Review notes

Slipcase Desktop reads and writes Slipcase containers. No account, no sign-in, no test credentials, and no network connection of any kind are needed to test it.

A container to open is at https://excelano.com/slipcase/quarterly-report.pdf.slpc — a one-page PDF inside a container with metadata rich enough to show every renderer the editor has. Download it and open it, or launch the application and use Open a container. The subject is invented: no real person, organisation, matter or date appears in it. It is built by `packaging/demo-container.sh` in the repository, which pins its archive timestamps so that any machine rebuilds the same bytes.

To exercise the rest: the metadata pane on the right edits every value by its kind, and a save keeps the comments, key order, whitespace and quoting of everything you did not touch. New container... asks which file to put in and where the container should go, writes it, and opens it, so any file you have to hand is enough to make a second container from nothing.

The App Sandbox is on with exactly two entitlements: the sandbox itself and read-write access to user-selected files. A save replaces the file the person chose, staged in the replacement directory macOS provides on the file's own volume and swapped in with one call, so it stays inside that grant. There is no network entitlement and the application makes no network request.

The application declares the Slipcase container type and claims it at rank Owner. This is the format's own application and the declaration below it in the bundle is the exported one, so Owner is the true rank; an application that merely opened somebody else's format would rank Alternate.

Slipcase implements no cryptography and makes no network request. It reads containers whose members may be encrypted and refuses those, which is a different claim from encrypting anything itself. The full privacy statement is at https://excelano.com/legal/#slipcase and the complete source is at https://github.com/excelano/slipcase-desktop.

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
or TestFlight — `CHECKLIST.md`'s *What a Store-signed build did when it was
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
