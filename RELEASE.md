# Release: getting Slipcase into two stores, repeatably

`HANDOFF-windows.md` and `HANDOFF-macos.md` are records of work that is
finished. This is the live one, and it is the only document that says what is
left before Slipcase is something a person can install without being handed a
file.

**The organising rule is that this is not a checklist.** A checklist gets a
first release out and is useless for the second. Every patch will need the same
things done again, so anything a machine can do belongs in a script under
`packaging/` and this file only holds what a person has to decide, observe, or
type into somebody else's website. Where a step below is prose rather than a
script, that is a claim that it *cannot* be scripted, and a later reader is
invited to prove it wrong.

Each section separates **once** from **every time**. The once-only work is
accounts, certificates, reserved names, and store records; it is done one
morning and never again. The every-time work is what a patch costs, and it is
the number worth minimising.

---

## The order, and why

1. **Here (Linux)** — everything that needs neither a Mac nor a Windows machine.
   Most of the shared work is here, and doing it first means neither platform
   session spends its time on something either could have done.
2. **Windows** — the package that does not exist yet.
3. **macOS** — credentials and hand-runs against a bundle that already builds.
4. **Back here** — a readiness review across all three before anything is
   submitted.

**Nothing is published until step 4 has happened.** Reserving a name in either
store is expected and encouraged — it is cheap, it is reversible, and on Windows
it is what unblocks the package identity. Submitting for review is not. Both
stores treat a submission as an event with a queue behind it, and the point of
step 4 is that the thing in the queue is one we have looked at together.

---

## One number, three spellings

`Cargo.toml` holds the version and nothing else should. Three artefacts want it
in three shapes:

| Where | Shape | Rule |
| --- | --- | --- |
| `Cargo.toml` | `0.1.0` | The source. |
| `AppxManifest.xml` | `0.1.0.0` | Four parts, and the Store requires the fourth to be `0`. |
| `Info.plist` `CFBundleShortVersionString` | `0.1.0` | What a person sees. |
| `Info.plist` `CFBundleVersion` | monotonic | Must increase on **every upload**, including a rejected one resubmitted unchanged. |

`CFBundleVersion` is the awkward one, because it is the only value here that is
not a function of the release version — two uploads of `0.1.0` need different
build numbers. Deriving it from the commit count (`git rev-list --count HEAD`)
makes it monotonic without anybody remembering, and that is the recommendation.

**Decided: the first public release is `0.1.0`.** Both stores display it and
`0.x` says *early* honestly, which is what this is. It affects listing
perception rather than any machinery, and it is settled before a name is
reserved rather than after, which is the only timing that matters.

---

## Here (Linux) — what needs no other machine

### Done, 2026-08-27

- **`slpc` 0.3.6 is published**, carrying `Container::payload_mode` and rather
  more than was planned for it. `excelano/slipcase` gained four requirements in
  SPEC §3 and a Security Considerations section as SPEC §6, and the library
  answers all of them: a bound on what identifying a container costs — measured
  at 620 MB from a 204 KB file before, 11 MB after — and `display_name`, which
  escapes the bidirectional formatting characters SPEC §3 requires be escaped
  wherever a name is shown. The corpus went from 77 cases to 83 and the
  reference implementation agrees on all of them.
- **The card line is in**, with the wording and the gating `DESIGN.md` §5
  decided, and the payload name on the card now goes through `display_name`.
  Neither is reachable by a test in this repository — the card is drawn rather
  than returned — so both are in `CHECKLIST.md` under *the card's two new
  lines*, and **neither has been run by hand on any platform.** That is the
  first thing each platform session should do.

### Every time

- **`packaging/version.sh`** — prints the version in whichever spelling is asked
  for, so that the three above cannot drift and no script re-implements the
  parsing. Consumed by `build-deb.sh`, `build-app.sh`, and whatever builds the
  MSIX.
- **`packaging/preflight.sh`** — refuses to proceed when the tree is dirty, the
  version and the changelog disagree, CI is not green on `HEAD`, or the corpus
  has not been run. This exists because the same seven checks were done by hand
  for `slpc` 0.3.5 and doing them by hand is what will be skipped on the patch
  nobody thinks is risky.
- **`CHANGELOG.md`** — this repository has none. Both stores show release notes
  per version and there is nowhere to write them from; `packaging/debian/changelog`
  is Debian-shaped and not that. The store text should be generated from it
  rather than written twice.

### Once

- **`AppxManifest.xml.in`** — templated on the identity values Partner Center
  assigns, the way `control.in` and `Info.plist.in` are templated. Declarative,
  and writable here even though it can only be *built* on Windows.
- **`SECURITY.md`** — `slpc-rust` has one and this repository does not. It is a
  public repository shipping an application that opens files people were sent,
  so a disclosure path is worth having before a store listing points at it.
- **A privacy policy at `excelano.com/legal/#slipcase`.** Decided, and the
  address follows what is already there: `/legal/` carries one page with a
  per-application anchor, `#blick` and `#zirbe` being the existing two. Both
  stores want the URL. The substance is short — no network calls, no telemetry,
  no analytics, no account, and nothing sent anywhere — and the existing entries
  are the model for how much of that to say and in what voice.

  **It must say what is stored, because something is.** An earlier draft of this
  line said *no configuration directory, nothing stored beyond files a person
  asked for*, and that is not true of the built application:
  `src/main.rs`'s `last_folder` writes the directory of the last container
  opened to `$XDG_STATE_HOME/slipcase-desktop/last-folder`, so that the file
  dialog opens somewhere useful. One line, one path, never sent anywhere, and
  removable by deleting the file — but a privacy policy claiming nothing is
  stored would be false, on a page two stores will link to. This is the class of
  error the readiness review below exists to catch, found here by reading the
  code rather than the sentence.
- **Store listing text**, drafted once and used twice: a description, a short
  description, and keywords. The two stores want different lengths; one draft
  answers both.
- **Screenshots are not here.** They need a real window on each platform, so
  they belong to those sections and are named there.

---

## Windows

Read `packaging/windows/README.md` first, especially *What a Store build would
need*. `CHECKLIST.md`'s MSIX section holds what was measured inside a real
package, and its final paragraphs say which of it a CI step could take over.

### Once

- **Reserve the name in Partner Center.** Do this first: `Package/Identity/Name`,
  `Publisher` and `PublisherDisplayName` are assigned at reservation, and a
  package whose identity disagrees with them is rejected at upload rather than
  at review. Everything below wants those three values.
- **Fill them into `AppxManifest.xml.in`** — or rather into whatever the build
  script substitutes from, so that they live in one place.

### Every time

- **`packaging/windows/build-msix.ps1`**, which does not exist and is the single
  thing standing between this platform and a submission. `packaging/macos/build-app.sh`
  is the model: take a release build, assemble a staging tree, substitute the
  version and the identity, call `makeappx`, and refuse loudly rather than
  produce something subtly wrong. It should also run the **Windows App
  Certification Kit** and fail on it, because certification runs it anyway and
  finding out here is cheaper.
- Note that the self-signed certificate from the MSIX sitting **does not carry
  forward** — the Store signs what it distributes. Local signing stays in the
  script for testing an install, but no certificate from it goes near a
  submission.

### By hand, because no script can

- **The stale `UserChoice` sequence.** `CHECKLIST.md` has it under *Not yet done
  by hand*, and it is unautomatable by design: the key carries a hash Windows
  validates and denies the user write access, precisely so that a default is
  something a person chose. Remove the script install **by hand** rather than
  with `uninstall.ps1`, which removes the `UserChoice` and would destroy the
  subject.
- **Screenshots** of the real window, at the sizes the Store asks for.
- **A walkthrough of the packaged application**, not the executable: install the
  package, double-click a container, press Open, and look at the screen. Every
  window defect this project has found was found this way and none of them by a
  test.

### Do not

Submit. Reserve the name, build the package, run WACK, and stop.

---

## macOS

Read `packaging/macos/README.md`, especially *What a Store build would need*,
and `CHECKLIST.md`'s macOS sections. More is done here than on Windows: the
bundle builds, the sandbox has been measured, the architecture is a build flag,
and `LSApplicationCategoryType` is declared.

### Once

- **Reserve the name in App Store Connect**, and create the app record and the
  App ID for `com.excelano.slipcase-desktop`.
- **Certificates and profile**: an Apple Distribution certificate, a Mac
  Installer Distribution certificate, and a Mac App Store provisioning profile,
  which a Store bundle carries as `embedded.provisionprofile` and which must
  declare the same entitlements the signature does.

### Every time

- **`build-app.sh` gains a Store mode.** It signs with an Apple Development
  identity today; a submission needs distribution signing, the embedded profile,
  and `productbuild --component` to produce the `.pkg` that Transporter uploads.
  Same rule as Windows: refuse loudly rather than produce something subtly
  wrong, and take the version from `packaging/version.sh`.

### By hand, because no script can

Four of these are already in `CHECKLIST.md` under *Not yet done by hand*, and
they are not paperwork — this platform has a history of the sandbox costing more
than anybody expected.

- **A distribution-signed bundle carrying a provisioning profile.** Every sandbox
  measurement to date was against a development-signed one, which is a different
  context. This is the one most likely to find something.
- **A downloaded bundle carrying `com.apple.quarantine`**, which is what a real
  user's first launch is.
- **A second user account, and an upgrade over an existing install.**
- **A container on a second volume, under the sandbox** — whether the grant
  covers the replacement directory.
- **`@2x` on a real high-density display**, which is half done.
- **Screenshots**, at the sizes App Store Connect asks for.

### Do not

Submit. Reserve the name, build and sign the package, validate it through
Transporter if that can be done without submitting, and stop.

---

## The readiness review, back here

The point of stopping before submission is that somebody looks at all three
platforms together, which no platform session can do. What it covers:

- **Every claim in the store listings is true of the built artefacts**, which is
  the class of error this project has caught most often — a sentence written
  before anybody looked.
- **The version is the same number in all three spellings**, and the changelog
  names it.
- **`CHECKLIST.md` has a section for every hand-run both platforms did**, with
  what it found rather than that it passed.
- **The executable-payload line has been seen by somebody.** It is written and
  unit-tested as of 2026-08-27 and has been rendered in front of nobody.
  `CHECKLIST.md`'s *the card's new lines, and what a save keeps* is the run,
  four items, and it wants doing on each platform. The fourth is the one to run
  first on macOS: it is the only place the provenance fix of 0.3.7 is untested. The same applies to the escaped payload name
  beside it.
- **The three CI workflows are green**, the corpus agrees on every case on every platform
  that has run it, and `preflight.sh` passes.

Then, and only then, both submissions go in.

---

## What a patch costs, afterwards

This is the number the whole file exists to keep small. If the above is done
properly, shipping a fix is: bump `Cargo.toml`, write a changelog entry, run
`preflight.sh`, run the three build scripts, upload two packages. Everything
else was once-only.

If shipping a patch turns out to cost more than that, the difference is a defect
in this file rather than in the release, and it belongs here as an amendment.
