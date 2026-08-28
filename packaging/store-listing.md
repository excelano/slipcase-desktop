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

**This is generated from `CHANGELOG.md`, not written beside it.** Every claim
below appears there first, checked against the built application rather than
against memory. If the two ever disagree, the changelog is right and this is
stale — that drift is the failure `RELEASE.md`'s readiness review names as the
one this project has caught most often.

Limits, so a later edit does not overrun them:

| Field | Microsoft Store | Mac App Store |
| --- | --- | --- |
| Description | 10,000 | 4,000 |
| Short description | 1,000 | — |
| Subtitle | — | 30 |
| Promotional text | — | 170 |
| Keywords | 7 terms | 100 characters |

---

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

If a container arrived from elsewhere — downloaded, or sent to you — it says so, and the payload you extract carries that marking onward, so whatever opens it next raises the same warning the container would have. Editing and saving keeps the marking too.

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
