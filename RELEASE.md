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

**Amended 2026-08-29: apt is an exception, taken deliberately and once.**
`slipcase-desktop` 0.1.0 went to the Excelano apt repository before step 4. The
rule above is written about the two stores and its reasoning is theirs — a queue
behind a submission, and a thing in it nobody can pull back. apt is our own
repository: publishing is `apt-ship` and unpublishing is a prune, both one
command, and nothing sits in anybody's review queue meanwhile.

What made it worth the exception rather than merely allowable: it was found
missing while the marketing pages were being written, so the alternative was a
page that either claimed an install that did not work or had no Linux story at
all. Linux is also the platform this project has tested hardest — every card
item run, lintian clean, the corpus agreeing, CI green — and shipping through
one channel first exercises the release path end to end before either store
submission depends on it.

**It is amd64 only, and that is stated wherever the install is written.**
Nothing here cross-compiles, no runner builds Linux arm64, and there is no
arm64 machine to run one on — so an arm64 `.deb` would be a binary nobody had
ever executed, which is not a thing this project ships. The command-line tool
ships both architectures through cargo-dist; this ships one, and a page
implying otherwise sends an arm64 reader to a command that answers *no
installation candidate*.

The question was opened and closed on 2026-08-29 on a machine that turned out
not to be arm64. It is recorded rather than tidied away because the answer
turns entirely on whether such a machine exists, so anybody reaching the same
conclusion later should know the position is *no hardware*, not *no interest*.

**What came out of it stays, because it was never about arm64.**
`build-deb.sh` now reads `e_machine` out of the ELF header and refuses where it
disagrees with the architecture the package would declare. `arch` comes from
`dpkg-architecture`, which answers about the machine and says nothing about a
binary handed over with `--binary`; a `.deb` declaring one architecture and
carrying another installs perfectly, does nothing when run, and shows no sign
of it anywhere on its face. That is the check `build-msix.ps1` already makes
against the PE header, and it guards the day a second architecture does
arrive.

**This does not move steps 2, 3 and 4.** Neither store submission happens before
the readiness review, and the review now has one more thing to check: that what
apt is serving is the same version the stores are being given.

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

**Amended 2026-08-29: the number the stores get is `0.1.1`, and apt has to be
re-shipped.** `0.1.0` was tagged and went to apt that morning. That afternoon
Windows found the tree showing a payload name unescaped where the card escapes
it, which is shared code and therefore in the `.deb` already installed. Building
the store packages from `HEAD` would have handed two stores a different binary
under a number apt was already serving — exactly what the readiness review below
says to check for.

So the bump is not bookkeeping: **what apt is serving is now the older, defective
build, and re-shipping it is Linux's work before step 4.** The store packages are
`0.1.1` and the tag for it belongs on the commit they were built from.

This is also the first exercise of the every-time path this file exists to keep
cheap, and it cost what it claims: a version bump, two changelog entries, a
rebuild, a reinstall, and the screenshots retaken. Nothing once-only was touched.

---

## Here (Linux) — what needs no other machine

### Done, 2026-08-27

- **`slpc` 0.3.6 is published**, carrying `Container::payload_mode` and rather
  more than was planned for it. `excelano/slipcase` gained four requirements in
  SPEC §3 and a Security Considerations section as SPEC §6, and the library
  answers all of them: a bound on what identifying a container costs — measured
  at 620 MB from a 204 KB file before, 11 MB after — and `display_name`, which
  escapes the bidirectional formatting characters SPEC §3 requires be escaped
  wherever a name is shown. The corpus went from 77 cases to 87 over the day and
  the reference implementation agrees on all of them.
- **The card line is in**, with the wording and the gating `DESIGN.md` §5
  decided, and the payload name on the card now goes through `display_name`.
  Neither is reachable by a test in this repository — the card is drawn rather
  than returned — so both are in `CHECKLIST.md` under *the card's two new
  lines*, and **neither has been run by hand on any platform.** That is the
  first thing each platform session should do.

### Every time

- ~~**`packaging/version.sh`**~~ — done 2026-08-28. Prints the version in
  whichever spelling is asked for, and `build-deb.sh` and `build-app.sh` both
  ask rather than parsing `Cargo.toml` themselves. Whatever builds the MSIX
  should ask it for `--appx`.

  It found something on the way in. `Info.plist.in` used one `@VERSION@` for
  both `CFBundleShortVersionString` and `CFBundleVersion`, and those must not be
  equal: App Store Connect deduplicates uploads by the second, so a bundle
  resubmitted after a rejection — unchanged, as a rejection often warrants —
  would have been refused for carrying a build number it had already seen. The
  template now takes `@BUILD@` as well, and `--build` is the commit count.
- ~~**`packaging/preflight.sh`**~~ — done 2026-08-28. Nine checks: a clean tree,
  nothing unpushed, both changelogs naming the version, a version the Appx
  spelling can represent, silent clippy, a passing suite, the corpus agreeing,
  and CI green on `HEAD` rather than on some commit.

      ./packaging/preflight.sh --corpus /path/to/slipcase/conformance --ci

  It refuses and never repairs, and each check was verified by breaking the
  thing it guards. The CI check in particular: its first version reported two
  failures on a green commit, because `gh` writes an empty string rather than
  null for a run still in progress and the grep matched it. It asks `jq` now,
  and distinguishes three states — a run still going is neither a pass nor a
  failure, and a release cut mid-flight is one nobody has checked.
- ~~**`CHANGELOG.md`**~~ — done 2026-08-28, with a 0.1.0 entry marked
  *unreleased* until a tag exists. It is written so the store listing text can
  be generated from it rather than written a second time and left to drift, and
  every factual claim in it was checked against the built artefact rather than
  against memory: the three buttons, the verdict vocabulary, the escaping, the
  unchanged-save, the read-back-before-replace, and `ldd` naming only libc,
  libgcc and libm.

  `packaging/debian/changelog` still says the same things in Debian's shape,
  because `dpkg` will not read the other one, and `build-deb.sh` already refuses
  to build a package whose version that file does not name — which is what keeps
  the two from parting.

### Once

- ~~**`AppxManifest.xml.in`**~~ — done 2026-08-28. Four placeholders: the three
  identity values Partner Center assigns when the name is reserved, and the
  version in its appx spelling. The file-type association mirrors what
  `install.ps1` writes for a side-loaded install, and the only capability
  declared is `runFullTrust`, because a capability asked for and unused is a
  question at certification with no good answer and a line a person reads before
  installing.

  It names image assets that do not exist: five PNGs under `Assets\`. The
  sources are all in the tree — `packaging/windows/slipcase.ico` and
  `packaging/linux/icons/` — so nothing has to be drawn, but producing them at
  the Store's sizes belongs in `build-msix.ps1`. `packaging/windows/README.md`
  says so where the Windows session will read it.
- ~~**`SECURITY.md`**~~ — done in `46dad19`, and this entry went on saying it was
  not for as long as it took somebody to check. It is a public repository
  shipping an application that opens files people were sent, so a disclosure
  path was worth having before a store listing points at it, and there is one:
  GitHub Security Advisories with an email fallback and a stated response time.

  **Left as a struck entry rather than deleted, because the drift is the
  lesson.** This file is the live document and it listed a finished thing as
  outstanding. That is the same class of error as a listing sentence written
  before anybody looked, arriving from the opposite direction, and it is what
  the readiness review below is for.
- **A privacy policy at `excelano.com/legal/#slipcase`.** Decided, and the
  address follows what is already there: `/legal/` carries one page with a
  per-application anchor, `#blick` and `#zirbe` being the existing two. Both
  stores want the URL. The substance is short — no network calls, no telemetry,
  no analytics, no account, and nothing sent anywhere — and the existing entries
  are the model for how much of that to say and in what voice.

  ~~Done 2026-08-28~~, in `packaging/privacy-entry.html`, following the Zirbe
  section's structure and voice. Maintained here rather than on the website, so
  its history is the history of the code it describes; it is pasted into
  `excelano.com/legal/index.html` after that section.

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

  **And there is a second write nobody had counted.** A first draft of the entry
  said two things reach disk: the remembered folder and an extracted payload.
  That was wrong. On Linux, `opens_with` asks `xdg-mime` what would open a
  payload, and `xdg-mime` answers about a file rather than a name — so it
  briefly creates two placeholders *carrying the payload's filename* in a
  private temporary directory. They hold a space and four zero bytes, never the
  payload, and the directory is removed before the question is answered. It is
  disclosed, because a privacy statement that omits a write is the same failure
  as one that overstates.
- **The five answers neither store will let you past, decided 2026-08-28.** None
  of these had been written down anywhere, and all five are asked by a form
  rather than by a build, which is why a repository full of measurements had
  nothing to say about them. Both stores ask for all five, so they live here
  rather than under a platform.

  | | |
  | --- | --- |
  | Price | **Free.** |
  | Support URL | `https://excelano.com/slipcase/` — **live**, checked 2026-08-29. |
  | Privacy policy URL | `https://excelano.com/legal/#slipcase` — **live**, checked 2026-08-29. |
  | Age rating | Every answer None. |
  | Export compliance | **No encryption.** Slipcase makes no network request and implements no cryptography; it *reads* containers whose members may be encrypted and refuses those, which is not the same claim. |

- ~~**Two pages on `excelano.com` are now submission blockers.**~~ **Both are
  live, checked 2026-08-29, and neither blocks anything.** `/slipcase/` answers
  200 and is the product page the pattern below asked for, carrying the install
  instructions for all three platforms. `/legal/` carries a Slipcase section
  whose anchor really is `id="slipcase"` — read out of the served HTML rather
  than out of a rendering of it, because the first check said the id was
  `#slipcase-macos-windows-linux-privacy` and it is not. The section is
  `packaging/privacy-entry.html` as written, down to the remembered folder and
  the two Linux placeholders.

  The struck text is kept because the shape of the error is the useful part:
  this file was measured against the live site, was right on the day, and went
  stale as soon as somebody did the work. **Both URLs are typed into both store
  forms, so a reader who trusts this paragraph fills in a field it says is
  broken.** Check the site, not this file.

  What follows is what the entry said before the pages existed.

  The privacy entry is written, in `packaging/privacy-entry.html`, and has never
  been pasted in: `/legal/` today carries xinglet, Blick and Zirbe and no
  Slipcase, so `#slipcase` resolves to the top of the page. A store listing
  linking to an anchor that is not there is worse than no anchor.

  The support page does not exist at all — `/slipcase/` is a 404. The pattern is
  a page per application, `/blick/`, `/blick-cli/`, `/zirbe/`, and that pattern
  raises a question this file cannot answer: **Blick has separate pages for the
  application and its command-line tool, and Slipcase has a desktop application,
  a command-line tool, and a format.** Whether that is one page or several is a
  decision, and the support URL above assumes one.

  Either can be typed into a store form before it resolves. Neither may still be
  unresolved at submission.

- ~~**Store listing text**~~ — done 2026-08-28, in `packaging/store-listing.md`.
  Every field measured against both stores' real limits, and one paragraph
  marked as differing per store: the executable-payload sentence is cut for the
  Microsoft Store, because that card line is gated to Unix and would describe
  something a Windows shopper never sees.
- **Screenshots are not here.** They need a real window on each platform, so
  they belong to those sections and are named there.

---

## Windows

Read `packaging/windows/README.md` first, especially *What a Store build would
need*. `CHECKLIST.md`'s MSIX section holds what was measured inside a real
package, and its final paragraphs say which of it a CI step could take over.

### Once

- ~~**Reserve the name in Partner Center**~~ — done 2026-08-28. `Slipcase` was
  available and is reserved. The section on the name below says why it is that
  and not `slipcase-desktop`. It was done first for the reason given here:
  `Package/Identity/Name`, `Publisher` and `PublisherDisplayName` are assigned
  at reservation, and a package whose identity disagrees with them is rejected
  at upload rather than at review. Everything below wanted those three values.
- ~~**Fill them into `AppxManifest.xml.in`**~~ — done 2026-08-28, and into
  `packaging/windows/identity.psd1` rather than into the template, which is what
  *or rather into whatever the build script substitutes from* asked for. The
  manifest keeps its four placeholders and nothing is edited per build.

  It holds two values beyond the three, and they earn their place: the package
  family name, which is what `Get-AppxPackage` is asked for to check an install,
  and the store id, which the listing URL is built from. Both are otherwise only
  findable by logging in. The MSA application id on the same page is needed by
  nothing and is recorded nowhere.

  **The file is not committed; `identity.psd1.example` beside it is.** None of
  it is a credential — `Publisher` is in the manifest of every package the Store
  distributes and the store id is in a public URL — and it was committed on that
  reasoning first. The reasoning was true and the call was wrong: an account's
  identifiers on a public record is the judgement
  `packaging/macos/SUBMITTING.local.md` already got. Caught and rewritten before
  the commit was pushed, so nothing was published and nothing had to be taken
  back out. `build-msix.ps1` refuses by naming the template, so a fresh checkout
  learns what it needs instead of failing at `makeappx`.

### Every time

- ~~**`packaging/windows/build-msix.ps1`**~~ — done 2026-08-28, and it built
  `Slipcase-0.1.0.0-x64.msix` from the release binary on the first run that
  parsed. `build-app.sh` was the model as intended: it takes a release build,
  stages the executable and the assets, substitutes the identity and the version
  — asking `version.sh --appx` rather than parsing `Cargo.toml` a second time —
  calls `makeappx`, and refuses rather than producing something subtly wrong.

  **Four refusals, each verified by breaking what it guards.** Two are read out
  of the PE header, because neither is visible in a finished package: the
  architecture, and the subsystem. The subsystem one is the interesting one.
  `src/main.rs` carries `windows_subsystem = "windows"` only when
  `debug_assertions` is off, so a debug binary packaged by mistake is a console
  subsystem one — and *a console window behind the application* is a defect the
  Windows walkthrough already found by hand. Handing it `target\debug\` is
  refused. The other two are the manifest's: a placeholder that survived
  substitution, and a `Publisher` that is not an X.500 string, which is the
  display name typed into the field that gets a package rejected at upload.

  The five images the manifest names exist now too, built by `make-ico` from the
  same SVG every platform's icon comes from and committed beside the `.ico`,
  with `windows.yml` comparing the whole directory against a rebuild. The `.ico`
  is byte-identical across that change, which is what says the shared renderer
  did not quietly alter it. `packaging/windows/README.md` records the one part
  of them that is a reading of Microsoft's guidance rather than a measurement.
- Note that the self-signed certificate from the MSIX sitting **does not carry
  forward** — the Store signs what it distributes. Local signing stays in the
  script for testing an install, but no certificate from it goes near a
  submission.

  **And it did not carry forward in a second sense, which was not foreseen.**
  The certificate from 2026-08-26 cannot sign this package at all: `signtool`
  refuses one whose certificate subject and manifest `Publisher` differ, and
  that certificate's subject was invented before a reservation existed.
  `-SelfSign` builds the subject out of `identity.psd1` instead of having it
  typed a second time, and makes a fresh throwaway.

### By hand, because no script can

- **One administrator action, and it gates the three below it.** The throwaway
  signing certificate has to reach `LocalMachine\TrustedPeople`; the per-user
  store is not read for this and importing there leaves deployment failing
  `0x800B0109` just the same, which `CHECKLIST.md` measured on 2026-08-26.
  `build-msix.ps1 -SelfSign` prints the two commands and does not attempt them.
- ~~**The Windows App Certification Kit**~~ — run 2026-08-28, three times, the
  middle one void, and **again on 2026-08-29 against 0.1.1, which is the run that
  counts**: the three earlier ones tested a binary that `src/tree.rs` changed the
  day after. `OVERALL_RESULT="PASS"`, `Blocked executables` failing as recorded,
  the gate recognising it and passing the run — the first time the rebuilt gate
  has been exercised by a run it should not refuse. It also traced the one
  message nobody had explained: `CSi` is three bytes of an address in a table in
  `.rdata`, not a string, which is why it appears in some builds and not others.
  Twenty-four tests, twenty-two passing.
  `CHECKLIST.md` holds all of it. What it left was two findings, both decisions
  rather than repairs, and **both have since been taken** — one by a repair that
  the kit then stopped reporting, one by a decision to submit with it failing.
  They are below with the reasoning for each.
- ~~**The tiles, looked at**~~ — done 2026-08-28. The tile and the application
  list entry were right, which settles the two-thirds split that was a reading
  of Microsoft's guidance. **The taskbar was not**, and nobody had thought to
  look there: it drew the icon on a plate filled with the user's accent colour.
  Fixed with unplated variants, scale variants, and the resource index that
  makes any of them resolve, then looked at again.
- ~~**A walkthrough of the packaged application**~~ — done 2026-08-28, against
  the installed package rather than the executable. Open hands the payload over,
  a container opened from Explorer launches the packaged binary, and the window
  reads correctly in both themes. Every window defect this project has found was
  found this way and none of them by a test, which is why it stayed on this list
  until somebody had looked.
- ~~**Screenshots** of the real window, at the sizes the Store asks for.~~
  **Done 2026-08-28, and this entry was wrong about needing a person.** It sat
  under *by hand, because no script can*, which was an assumption:
  `packaging/windows/screenshot.ps1` sizes the window so the visible frame is
  exactly what the Store asks for, brings it to the front, captures it, crops,
  and refuses if what came back is the wrong size. Two shots at 1366 x 768,
  against the packaged application, listed in `packaging/store-listing.md`.

  What a script genuinely cannot do is smaller than the entry claimed, and the
  script says it out loud when it finishes: choosing which container to open and
  judging whether the result is a good advertisement. The first is an editorial
  decision recorded in the listing; the second is a look.

  It cost two measurements, both pixels, and both are in the script's header.
  `SetWindowPos` sizes the window rect, which carries an invisible resize border
  — asking for 1366 x 768 gave a visible frame of 1352 x 761, under the minimum.
  And the frame's top edge is a pixel above what is drawn, so the first two
  attempts came back with a strip of console text across the top.

### Two decisions the certification kit left, and neither is a repair

Both are about the executable rather than the package, both survived three runs
unchanged, and both are **David's**. `CHECKLIST.md` has what each was traced to.

**Both have now been taken, and neither is waiting on anybody**: the DPI one by
a repair, and this one by a decision to submit with it failing. The heading says
*decisions* because that is what the kit left, not because two are open.

- **`Blocked executables`, FAIL.** The kit objects to `cmd.exe` and `\cmd.exe`
  in the binary, and to `CreateProcessW` and `ShellExecuteW`. The two `cmd.exe`
  strings are the Rust standard library's batch-file spawn — they sit beside a
  `library\std\src\sys\…` path in the binary, and a hello-world Rust binary
  contains neither. Every `std::process::Command` in this repository is inside a
  Linux or macOS `#[cfg]` arm, so none of it compiles here. `ShellExecuteW` is
  `opener`, whose Windows arm calls that and nothing else: it is the Open
  button, and removing it is removing the application.

  So the decision is not *what to change* but **whether to submit with it**.

  **And it is a much smaller decision than it looked, because the kit says so
  itself.** `configuration.xml` in the App Certification Kit defines that task
  with `OPTIONAL_FOR_APP_TYPES="Centennial"`, and the report's own root says
  `APP_TYPE="Centennial"` — a packaged desktop application. The test is optional
  for this kind of application, which is also why three runs reported `WARNING`
  overall over a test reading `FAIL`. That was read out of the files the kit
  ships rather than out of documentation; `CHECKLIST.md` has the task ids.

  **And the kit has now demonstrated what that attribute means, rather than
  merely declaring it.** With the DPI warning answered, the same package with
  the same failing test comes out `OVERALL_RESULT="PASS"` — twenty-three tests
  passing, `Blocked executables` still failing, and the kit passing the package
  anyway. An optional test failing does not stop the kit. That is no longer an
  inference from an XML attribute; it is what the tool did.

  It is evidence and not a guarantee. Certification runs this suite, so it is a
  good deal better than a guess, but a person applies policy on top of a suite
  and nothing local can measure that. The honest position is that the one
  failing test is one Microsoft's own tool marks optional for this kind of
  application and passes the package in spite of, and that the application
  cannot do its job without the API it names. **That is about as settled as it
  gets short of submitting.**

  **Decided 2026-08-28: submit with it failing.** David's, on the reasoning
  above, and written here because two other places defer to this file for it —
  `CHECKLIST.md`'s certification section and `KNOWN_FINDINGS` in
  `build-msix.ps1` each say the decision is his and that `RELEASE.md` carries
  it. It carried the argument and not the answer until 2026-08-29, so a reader
  arriving at either of those pointers found an open question that had been
  taken. That is the drift the struck `SECURITY.md` entry above is kept as a
  lesson about, and this is the second instance in the same file.

  **What it commits to is a submission and not a change.** Nothing in the binary
  moves, `-Certify` goes on passing the package with the finding recorded, and
  if review objects the position is the one stated above rather than removing
  the API: `ShellExecuteW` is how the Open button hands a payload to the system,
  and an application that cannot do that is not this application.

  **It does not move the *Do not* below.** What was decided is that this finding
  is not a reason to hold the submission back, not that the submission happens
  now. Step 4 is still the gate, and this is one of the things it will look at.

- **`DPIAwarenessValidation`, WARNING.** The kit says the application is not DPI
  aware. It is: `GetWindowDpiAwarenessContext` on the running packaged window
  reports `PER_MONITOR_AWARE`, which winit sets at startup, and the 125% and
  200% hand run agrees. What is missing is the *declaration* — the kit reads the
  PE application manifest and there is none.

  Declaring it hits the wall the window icon hit: `rc.exe` and `windres` are
  build steps `DESIGN.md` §2 keeps out, which is why the icon travels through
  `include_bytes!`. There is a route that compiles nothing — the MSVC linker
  takes `/MANIFESTINPUT` with `/MANIFEST:EMBED` through `-C link-arg` — and
  taking it is a `DESIGN.md` §2 amendment rather than a packaging change.

  **Decided, done, and confirmed, 2026-08-28.** The linker route was taken,
  `DESIGN.md` §2 is amended with why a linker argument is not the build step it
  keeps out, and the manifest is embedded and read back out of the binary. It
  changed no behaviour — the build from before it was already
  `PER_MONITOR_AWARE_V2` — so what it buys is the declaration itself and
  awareness being set before any of this program's code runs.

  **The kit is satisfied: it no longer reports the finding, and the package now
  passes overall.** This item is closed.

### Do not

Submit. Reserve the name, build the package, run WACK, and stop.

---

## macOS

Read `packaging/macos/README.md`, especially *What a Store build would need*,
and `CHECKLIST.md`'s macOS sections. More is done here than on Windows: the
bundle builds, the sandbox has been measured, the architecture is a build flag,
and `LSApplicationCategoryType` is declared.

### Once

- ~~**Reserve the name in App Store Connect**~~ — done 2026-08-28, and the name
  is **`Slipcase Desktop`** rather than `Slipcase`, which was already taken. The
  section on the name below says by whom and why no claim was available. The App
  ID `com.excelano.slipcase-desktop` and the three certificates and the Mac App
  Store profile are all done and verified; those are reservations in a different
  namespace from the storefront name and the distinction is why only one of them
  had to move.
- ~~**Certificates and profile**~~ — done 2026-08-28 and verified from this
  machine rather than taken on trust. Three identities are installed with their
  private keys, and the distribution one was used to sign a real bundle: full
  chain to the Apple Root CA, `TeamIdentifier=9K6W5PMFYP`, entitlements intact
  through the signature. The certificate names differ from the portal's labels
  and a script has to use the real ones:

      Apple Distribution: Excelano LLC (9K6W5PMFYP)
      3rd Party Mac Developer Installer: Excelano LLC (9K6W5PMFYP)
      Developer ID Application: Excelano LLC (9K6W5PMFYP)

  The middle one is what Apple's portal calls *Mac Installer Distribution*, and
  it does not appear under `security find-identity -p codesigning` because it
  signs packages rather than code — its absence there is correct and has caught
  people out. The third is not needed for the Store and was taken as the hedge
  if review goes badly.

  The profile is `Slipcase Mac App Store`, platform OSX, bound to exactly one
  certificate — the Apple Distribution one actually installed here — naming
  `9K6W5PMFYP.com.excelano.slipcase-desktop` with no wildcard, expiring
  2027-08-28.

  **It also settled what a Store build must be signed with, which is not what
  the local builds carry.** The profile grants
  `com.apple.application-identifier` and `com.apple.developer.team-identifier`,
  and `packaging/macos/Slipcase.entitlements` has neither — it holds the sandbox
  and user-selected files, which is right for a development build and is not
  enough for an upload. The Store mode needs its own entitlements. It should
  **not** take the third thing the profile offers: `keychain-access-groups` is
  granted and this application touches no keychain, and a capability asked for
  and unused is a question at review with no good answer.

### Every time

- ~~**`build-app.sh` gains a Store mode.**~~ — done 2026-08-28.
  `--store PROFILE` produces what a submission is: a universal bundle carrying
  the profile as `embedded.provisionprofile`, signed for distribution with the
  entitlements a Store build needs, wrapped by `productbuild --component` into a
  signed `.pkg`. Built and verified against the real certificates and the real
  profile rather than against a description of them.

  **Nothing account-specific is written down.** The team and the application
  identifier are read out of the profile, so the profile — which is what App
  Store Connect validates against — is the only copy and cannot drift from a
  second one. The identities are found by prefix and team, and it refuses on
  anything but exactly one match: two certificates of a kind in one keychain is
  an ordinary state, an expiring one beside its replacement, and picking
  whichever came first is how a package gets signed with the wrong one.

  It refuses before it builds: no profile, a file that is not one, an expired
  one, a profile whose application identifier does not match `CFBundleIdentifier`,
  or `--sign` given alongside. Afterwards it reads back what the signature
  actually carries — the sandbox and the application identifier — because
  neither is visible by looking at the bundle and each has its own cost, a day
  and an upload. The identifier check was broken deliberately by pointing a good
  profile at a renamed bundle, and it refused with both names.

  `keychain-access-groups` is declined although the profile grants it, for the
  reason `AppxManifest.xml` declares only `runFullTrust`: a capability asked for
  and unused is a question at review with no good answer.

### By hand, because no script can

Most of these are already in `CHECKLIST.md` under *Not yet done by hand*, and
they are not paperwork — this platform has a history of the sandbox costing more
than anybody expected.

**These come before any of the packaging work.** Both ask whether the
App Sandbox breaks something this application did to itself in the days before,
and both have answers that are decisions about `DESIGN.md` §5 rather than
repairs. Taking a decision like that after a bundle is signed and a listing is
written around it is the expensive order. The first is now answered; the second
is not.

- ~~**Can a launched application read the payload at all, under the sandbox?**~~
  **Answered 2026-08-28: yes, under the sandbox.** ~~and Open works on the Store
  build~~ — **struck 2026-08-29**: the bundle was development-signed, and a
  Store build will not launch on a developer's own machine at all. See *What a
  Store-signed build did when it was launched* in `CHECKLIST.md`. Preview came
  forward showing the PDF, the card's *Opens with* line read Preview, the
  payload landed at 0700 inside this application's container and byte for byte,
  and nothing outside it was written. `CHECKLIST.md`'s *What the sandboxed
  handover found* holds the run, including the two things it settled on the way:
  that `ps eww` cannot tell you whether a process is sandboxed and will lie in
  the reassuring direction, and that the platform marks what a sandboxed process
  writes while `slpc::provenance` correctly disregards its own agent. **No
  `DESIGN.md` §5 decision is owed.** The question and its reasoning are kept
  below because the answer is only worth as much as the doubt it settled.

  It was `CHECKLIST.md` item 6, and **the very first thing** — one open, one
  button, and one look at the screen, and the cheapest way to find out whether
  the Store build works at all.

  On 2026-08-27 the handover directory became mode 0700, to stop every payload
  somebody pressed Open on being readable by every account on the machine. On
  Linux that costs the handler nothing, because it runs as the same user, and
  `tests/handover.rs` proves it by reading the payload back from a separate
  process. Under the App Sandbox the handler is a *different application with a
  container of its own*, and the payload now sits in a private directory inside
  this one's. Launch Services normally grants the opened application a scope for
  the URL it was launched with, which ought to be enough — but *ought* is not a
  measurement, and nothing on Linux can take it.

  **If it had failed, Open would have failed for every container on the Store
  build**, and the fix would have been a decision rather than a repair: the mode
  that keeps a payload private from other accounts on the machine is the mode
  that would have been hiding it from the program meant to open it. It did not
  fail.
- ~~**Saving an edit to a downloaded container, under the sandbox.**~~
  **Answered 2026-08-28, and it is the failure this entry was afraid of.** Save
  succeeds and the edit lands. The container stays gated — the flags are
  unchanged at `0083` — but the agent becomes `slipcase-desktop`, so
  `arrived_from_elsewhere` answers false and the card's *arrived from elsewhere*
  line is there before the save and gone after it. `CHECKLIST.md`'s *What saving
  a downloaded container under the sandbox found* holds the measurement and the
  mechanism.

  **Fixed the same day, in the library, and this repository needs no code
  change.** A sandboxed process cannot attribute a file to anyone but itself,
  which the probe settles — but it can write an attribute of its own beside the
  platform's, and that one survives the replacement that destroys the
  attribution. `slpc` gained `Mark::Recorded` and
  `com.excelano.slipcase.origin`, and `arrived_from_elsewhere` consults it. The
  card is right after a save and after a reopen, proven end to end through a
  signed sandboxed bundle. **Done**: `slpc` 0.3.10 shipped on
  2026-08-28 and this repository takes it, which is all that was left here.
  The original entry is kept below because its reasoning was right.

  `CHECKLIST.md`
  item 4, and it was **the first thing left**: if the sandbox breaks
  the provenance carry the way it once broke Save, that is a decision about
  `DESIGN.md` §5 rather than a repair, and it is cheaper to take before a bundle
  has been signed and a listing written around it.

  `slpc` 0.3.7 stopped an in-place rewrite stripping the mark, and CI proves that
  on all three platforms **unsandboxed** — the Windows and macOS runners each
  execute `tests/provenance.rs` in full, writing and reading back a real
  `Zone.Identifier` stream and a real `com.apple.quarantine`. What no runner
  reaches is this application's own macOS arm: `src/staging.rs` replaces the
  container through `-[NSFileManager replaceItemAtURL:…]` rather than the rename
  the library uses, and carries the mark itself. Sandboxed, the platform also
  marks whatever this process writes and names this application as the agent —
  and `DESIGN.md` §5 has the card disregarding those, so the file can stay gated
  while the card stops saying where it came from. That is the answer to look for.
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

**The Apple silicon question is mostly answered, and by a machine.**
`packaging/macos/README.md` said somebody had to open a container on an Apple
silicon Mac before submission. Since 2026-08-28 the arm64 runner does it on every
push: it opens one through Launch Services, asserts that a window appeared by
asking the window server rather than by looking at a screenshot, and asserts that
the container's folder was remembered — which is what distinguishes a delivered
document from a bare launch, the refusal it guards against having been an empty
window. `CHECKLIST.md`'s *What Apple silicon answered, and what it still cannot*
holds the run and the photograph.

**It is not answered enough to submit on, and the paragraph above should not be
read that way.** What the runner exercises is an unsigned, unsandboxed debug
bundle. What a submission is, is a universal *sandboxed* bundle signed for
distribution and wrapped in a `.pkg`, and none of that has run on Apple silicon
at all. Every sandbox measurement this project holds — the handover, the save,
the provenance carry, the origin note — was taken on x86_64. Sandbox policy
ought not to depend on the architecture, and *ought* is the word that has cost
this platform a module, a rewritten save path and a reopened `DESIGN.md` section
already.

So a real Apple silicon Mac is still required before submitting. What changed is
the shape of the risk rather than its presence: the Objective-C event module is
no longer unexecuted there, so what remains to be found is in the sandbox and the
signing context, which is a narrower place to look and a shorter list to get
through. **The distribution-signed bundle carrying a provisioning profile is
still the run most likely to find something**, and it has happened on neither
architecture.

### Do not

Submit. Reserve the name, build and sign the package, validate it through
Transporter if that can be done without submitting, and stop.

---

## What was decided about the name, and about the command-line tool

Asked on 2026-08-28, whether two names should be reserved — `slipcase` for the
command-line tool and `slipcase-desktop` for this application. Decided that one
name is reserved in each store and that it is **Slipcase**, and the reasoning is
here because both halves of the question will come back.

**Amended the same day: the two stores hold different names, because one of them
was taken.** Partner Center accepted `Slipcase`. App Store Connect refused it —
it belongs to an insurance and reinsurance news application by Everlution, on the
store since 2016, and *slipcase* is a term of art in that industry. There is no
trademark route and they have the better claim. **`Slipcase Desktop` is reserved
on the Mac App Store**, and `packaging/store-listing.md` carries both names with
the alternatives that were rejected.

What did **not** change is everything below: the application still calls itself
`Slipcase` in `CFBundleDisplayName` and `Package/Properties/DisplayName`, the
format is still `slipcase` and is not this repository's to rename, and
`slipcase-desktop` is still an identifier rather than a name. A storefront name
was the only thing that moved.

**`slipcase-desktop` is not a name.** `CLAUDE.md` has said so from the start:
the application is presented to a person as Slipcase, and `slipcase-desktop` is
the crate, the binary, and the stem of `com.excelano.slipcase-desktop`. A store
reservation claims the display name a shopper sees and searches, and no store
displays an identifier. The bundle identifier is claimed separately in App Store
Connect and that reservation is not this one. `packaging/store-listing.md` now
carries the name as a field rather than only inside its prose, with the four
places it has to be typed identically named there.

**The command-line tool is not a store product, and on macOS it cannot be.**
`slpc-rust` already distributes it more thoroughly than this application is
distributed: `dist-workspace.toml` builds five targets with cargo-dist,
including `aarch64-apple-darwin`, and produces a shell installer, a PowerShell
installer, and a Homebrew tap at `excelano/homebrew-tap`, alongside crates.io
for `cargo install` and `[package.metadata.deb]` for Debian. Those are the
channels somebody looking for a CLI reaches for, and a store is not among them.

Apple's side settles itself: the Mac App Store takes GUI application bundles,
and a sandboxed bundle cannot put anything on `PATH`, so a command-line tool
could only ship buried inside a container where being on `PATH` is the entire
point. The Microsoft Store could carry a console application inside an MSIX, but
winget is the idiomatic Windows channel for one and the PowerShell installer
already covers the same ground. If a store-shaped Windows presence is ever
wanted for the CLI, a winget manifest is the thing to want, and it is
independent of everything in this file.

**So the stores are for the GUI.** Reserving a second name buys nothing and both
stores reclaim reservations that never receive a build, so a defensive
reservation would most likely lapse before it was ever used.

---

## What was decided about reviewing from other machines

Asked on 2026-08-27, whether the security review should also run on a Mac and a
Windows box. Decided not, and the reasoning belongs here because the question
will come back.

Almost none of it needs those machines. Every defect found that day lived in code
all three platforms share, and the platform-specific surface is genuinely small:
three `#[cfg]` arms in `src/opens_with.rs`, one module in `src/opened_document.rs`,
and the packaging scripts. CI already runs the two arms it can, on real runners.

What is left needs a person rather than a reviewer: the App Sandbox, the Apple
Event a double-click delivers, the MSIX and the `.app`, and everything in
`CHECKLIST.md` that wants eyes. Those belong to the platform sections above,
which will already be sitting in front of the right machine — as extra items on a
session that is building a package anyway, rather than a review pass of its own.

---

## The readiness review, back here

The point of stopping before submission is that somebody looks at all three
platforms together, which no platform session can do. What it covers:

- **Every claim in the store listings is true of the built artefacts**, which is
  the class of error this project has caught most often — a sentence written
  before anybody looked.

  ~~**One is already known to need rewording, found 2026-08-28.**~~ **Rewritten
  the same day, and it did not become a second per-store variant.** The
  description promised that a payload *carries that marking onward, so whatever
  opens it next raises the same warning the container would have*. That is
  exactly true on Windows, where the zone stream is copied verbatim and the shell
  stops for the copy as it would have for the container. It is not true of a Mac
  App Store build: the sandbox marks what the process writes and refuses to have
  that mark replaced, so the payload carries this application's mark rather than
  the container's, and no API avoids it.

  **A wording problem and not a safety one**, established before it was assumed:
  `CHECKLIST.md`'s *What the provenance sitting found* measured the sandbox's own
  mark gating at least as hard as the one it displaces and harder for anything
  that executes, and stripping it being the difference between an unsigned
  application from the internet running and being stopped.

  So the sentence now says the payload is marked and the computer *treats it with
  the caution it gives anything that came from outside* — true of both stores —
  rather than naming a warning that is only one store's. **Not splitting it was
  the point.** The executable-payload sentence already differs per store, and a
  listing kept in two shapes drifts in one of them; the note at the top of
  `packaging/store-listing.md` records the reasoning where an editor will see it
  before improving the vaguer wording back.
- **The version is the same number in all three spellings**, and the changelog
  names it.
- **`CHECKLIST.md` has a section for every hand-run both platforms did**, with
  what it found rather than that it passed.
- **The executable-payload line has been seen by somebody** — on macOS as of
  2026-08-28 and on Linux as of 2026-08-29, along with the escaped payload name
  and the silence for an ordinary container. ~~**All six are done on macOS and
  all five that apply are done on Linux; Windows has had none of them and is the
  only platform this item is still waiting on.**~~ **Done everywhere as of
  2026-08-29**, Windows having run all five that apply that day, so this item is
  closed and `CHECKLIST.md` holds three runs.

  **The last arm to run it found something the first two had ticked past**, and
  the review should know what that says about hand items. Item 3 says to look at
  the card, and macOS and Linux looked at the card and were right: it escapes the
  payload name. Two rows below it the metadata tree was rendering the same name
  unescaped, so a payload called `report<U+202E>fdp.exe` read `reportfdp.exe`
  there — the spoof the escaping exists to prevent, in the one field this
  application will not let anybody edit. Shared code, so all three platforms had
  it, and no test in either repository reached it. Fixed the same day in
  `src/tree.rs`, `DESIGN.md` §4 amended, two tests each broken deliberately.

  **It makes the first bullet concrete a second time.** `CHANGELOG.md` says
  *names are shown with the Unicode characters that reorder text escaped*, and
  the store descriptions are generated from it. The sentence was true of the card
  and false of the tree — written against the built application, and checked in
  the one place the claim already held. It is true of both now.

  `CHECKLIST.md` holds all three runs, under *What the card's three lines looked
  like* — *on macOS*, *on Windows*, and *here*, which is Linux's, the two
  sections having briefly shared a title until the third one arrived.

  ~~**One of the six cannot be run as written and this review should not accept
  a tick for it.**~~ **Fixed at the source on 2026-08-29.** Item 2 asks for the
  silence on a container recording *no* mode, and the fixture it named recorded
  0644 like every other `accept` case, so followed literally it tested
  `minimal.slpc` twice. Rather than a third platform building one by hand, the
  case went into the corpus — `accept/payload-no-mode-recorded`,
  `excelano/slipcase` `996dcca`, `2.0 fat` with external attributes `0x20`. The
  corpus is 88 cases and the three workflows are pinned to it. The item now runs
  from the corpus everywhere, and this review should expect a tick with a
  fixture name beside it.
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
