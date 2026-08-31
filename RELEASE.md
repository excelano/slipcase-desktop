# Release: getting Slipcase into two stores, repeatably

**This is the process, not the history.** What a given release did is in
`git log` and in `CHANGELOG.md`; what is here is what the *next* one costs and
in what order. Anything a machine can do belongs in a script under `packaging/`,
and where a step below is prose rather than a script, that is a claim it cannot
be scripted — a later reader is invited to prove it wrong.

Two documents are named here and not committed, because each carries an
account's own identifiers: `packaging/windows/SUBMITTING.local.md` and
`packaging/macos/SUBMITTING.local.md`. They walk their store's form page by
page. `packaging/windows/identity.psd1` is the same and has an
`identity.psd1.example` beside it.

---

## The order

1. **Linux**, which needs no other machine and where most shared work lands.
2. **Windows.**
3. **macOS.**
4. **Back on Linux**, for the readiness review across all three.

**Nothing is submitted until step 4.** Both stores treat a submission as an
event with a queue behind it, and the point of the review is that the thing in
the queue is one somebody looked at across every platform.

**apt is the exception, taken deliberately.** It is our own repository:
publishing is one command and unpublishing is a prune, and nothing sits in
anybody's review queue meanwhile. What that costs is a rule of its own — see
*What apt costs* below.

---

## One number, three spellings

`Cargo.toml` holds the version and nothing else should. `packaging/version.sh`
is the only thing that reads it, and every build script asks it rather than
parsing `Cargo.toml` a second time.

| Where | Shape | Rule |
| --- | --- | --- |
| `Cargo.toml` | `X.Y.Z` | The source. |
| `AppxManifest.xml` | `X.Y.Z.0` | Four parts, and the Store requires the fourth to be `0`. |
| `Info.plist` `CFBundleShortVersionString` | `X.Y.Z` | What a person sees. |
| `Info.plist` `CFBundleVersion` | the commit count | Must increase on **every upload**, including a rejected one resubmitted unchanged. |

`CFBundleVersion` is the awkward one: two uploads of the same version need
different build numbers, so it comes from `git rev-list --count --first-parent
HEAD`. `--first-parent` is not decoration: this history carries a merge, and
counting without it lands three commits away.

**It buys a second thing.** The build number App Store Connect shows is a
pointer back to a commit, so the store record identifies its own source with
nobody having written the mapping down. Read it back with the same count against
a candidate commit, and take `version.sh` as the authority over this paragraph —
which used to carry a worked example naming a build and a commit. The example
counted without `--first-parent`, so it agreed by coincidence while pointing
three commits away from the real one, and it sat there wrong because nobody
recomputes a number a document has already written down. Deleted rather than
corrected, for the reason `CLAUDE.md` gives about the conformance count.

**The Appx spelling has no such property**: it is a function of the release
version alone, so on Windows the only record of which commit was uploaded is the
one `SUBMITTING.local.md` keeps.

**Bump only for a number that has been *tagged*.** A code change costs a
certification re-run either way; only a published number costs a version as
well. Ask `git tag --list` before deciding. The corollary caught us once: a
packaging fix found after `v0.1.2` was tagged stayed *inside* 0.1.2, because no
0.1.2 package had been published and there was nothing for it to disagree with.
The rule exists to stop one number covering two artefacts, not to count changes.

---

## Linux

    cargo build --release
    ./packaging/linux/check-libraries.sh          # both display backends
    ./packaging/debian/build-deb.sh
    ./packaging/preflight.sh --corpus /path/to/slipcase/conformance --ci

`preflight.sh` is the gate: a clean tree, nothing unpushed, both changelogs
naming the version, a version the Appx spelling can represent, silent clippy, a
passing suite, the corpus agreeing, and CI green on `HEAD` rather than on some
earlier commit. It refuses and never repairs.

`check-libraries.sh` runs the window under Wayland and X11 and refuses any
library whose package `Depends` does not transitively reach. It exists because
two releases shipped without `libxkbcommon-x11-0`, which every build machine had
and a clean X11 machine did not.

Then tag, release, and ship:

    git tag -a vX.Y.Z            # the commit the store packages were built from
    gh release create vX.Y.Z dist/slipcase-desktop_X.Y.Z_amd64.deb --notes-file …
    apt-ship slipcase-desktop vX.Y.Z -y

**amd64 only, and say so wherever the install is written.** Nothing here
cross-compiles and there is no arm64 machine to run a build on, so an arm64
`.deb` would be a binary nobody had executed. The position is *no hardware*, not
*no interest*.

### What apt costs

Publishing to apt before the stores is allowed and the rest of the rule is not
suspended: the readiness review still gates both submissions, and it has one
more thing to check — that what apt is serving is a version the stores also
have, or a later one whose difference is understood.

`apt-ship -n` followed by `-y` used to abort — the dry run's prune had already
taken the old version, so the second run found nothing to prune while the remote
deletion was still pending and the guard refused a removal nobody asked for.
Fixed in `~/bin` `da3e961`: the record of pruned-but-undeployed files outlives
the run and a successful deploy empties it. Both flags are safe in either order
now, and a carried-over deletion says so in the preview.

---

## Windows

    cargo build --release
    powershell -File packaging\windows\check-imports.ps1
    powershell -File packaging\windows\build-msix.ps1 -SelfSign -Certify

`check-imports.ps1` walks the PE import table and refuses any DLL not known to
ship with Windows. `.cargo/config.toml` links the CRT in for that target alone,
because 0.1.1 linked `VCRUNTIME140.dll` — not part of Windows — and failed
certification on a tester's clean machine under policy 10.2.4.1.

`build-msix.ps1` refuses four things, each verified by breaking it: a wrong
architecture and a console subsystem, both read out of the PE header; a
placeholder that survived substitution; and a `Publisher` that is not an X.500
string. It runs `check-imports.ps1` and will not package a binary that fails.

**One administrator action gates the local install**: the throwaway signing
certificate must reach `LocalMachine\TrustedPeople`. The per-user store is not
read for this and leaves deployment failing `0x800B0109`. `-SelfSign` prints the
two commands and does not attempt them. The certification kit also needs an
elevated prompt.

**Do not rebuild before uploading.** A rebuild of identical source produces a
different file — 24 bytes, being the COFF timestamp, the three debug directory
timestamps and the CodeView PDB GUID. The artefact uploaded has to be the one
the certification kit passed, not a fresh build of the same commit. Two packages
come out of one staging tree: the unsigned one is the upload, because the Store
signs what it distributes, and the throwaway-signed one is for installing here.

### What the Partner Center form actually does

Four things no documentation said. The first three are walked through in
`SUBMITTING.local.md`; the fourth was found afterwards, by looking at what the
first submission had been filed under:

- **The Store logo field takes 1080x1080 or 2160x2160**, not the 300x300 the
  older documentation describes. `make-ico` draws both into
  `packaging/windows/listing/` rather than into the package assets, because a
  file added to the assets lands in the MSIX and a package that gains a file has
  to be certified again.
- **The restricted-capability justification caps at 500 characters**, counting
  newlines, and truncates silently at the paste.
- **The reviewer has nothing to open.** Slipcase without a container is an empty
  window and the notes field takes no attachment, which is why the sample
  container is served from the website and its URL is one of the answers below.
- **The store listing has a language, and it came from the package.** The first
  submission was filed under *English (United Kingdom)*, which is what
  `AppxManifest.xml.in` declared and the only place in this repository that
  names a language. It declares `en` now, so the next package's submission
  belongs under *English*: add that listing, copy `packaging/store-listing.md`
  into it, and delete the English (United Kingdom) one, the form having no
  rename. The copy is spelled British either way, which is a house style rather
  than a market. Read the language back in the form before committing the
  submission — neither channel below serves it.

**The submission API cannot make a first submission.** MSIX apps use the API at
`manage.devcenter.microsoft.com` — `api.store.microsoft.com` is for MSI and EXE
installers — and it requires one submission to exist, made in Partner Center
with the age-ratings questionnaire answered. From the second onward it can, and
the setup is once: an Entra directory on the account, an Azure AD application in
*Users* with the **Manager** role, and its tenant id, client id and key. **A
submission created through the API must be edited only through the API**;
touching it in Partner Center can leave it uncommittable, and the recovery is to
delete it and start again.

### Reading the listing back, which needs no login

Once a submission is live the Store publishes it, and two public channels answer
without a Partner Center session — which is the point, because a dashboard
reporting its own success is the same evidence as a build script reporting its
own output:

    $id = (Import-PowerShellDataFile packaging\windows\identity.psd1).StoreId
    winget show --id $id --source msstore
    Invoke-RestMethod ("https://displaycatalog.mp.microsoft.com/v7.0/products/" +
        $id + "?market=US&languages=en-US&fieldsTemplate=Details")

`winget` gives the description, price, category, publisher and both URLs — so
the description can be **diffed** against `packaging/store-listing.md` rather
than read, which is the only way to confirm the per-store cut that file's header
requires was actually made in the form. The display catalogue gives the package:
`PackageFullName` carries the version, and `Version` is the same four parts
packed into a 64-bit integer, so `0.1.2.0` reads `4295098368`. That is what says
the Store is serving the build that was uploaded. `winget`'s own `Version`
column reads `Unknown` from the msstore source and that is not a fault.

**What neither serves back** is the screenshots, the search terms, the
`runFullTrust` justification, the notes to certification, and *What's new in
this version*. Those stay write-only, so a check here must say so rather than
imply a listing was verified.

---

## macOS

    cargo build --release
    ./packaging/macos/build-app.sh --store ~/Downloads/Slipcase_Mac_App_Store.provisionprofile
    ./packaging/macos/check-install.sh
    ./packaging/macos/screenshot.sh

`--store` produces what a submission is: a universal bundle carrying the profile
as `embedded.provisionprofile`, signed for distribution, wrapped by
`productbuild --component` into a signed `.pkg`. Nothing account-specific is
written down — the team and application identifier are read out of the profile,
so the profile is the only copy and cannot drift from a second one. It refuses
before it builds on a missing, invalid or expired profile, on an application
identifier that does not match `CFBundleIdentifier`, and on anything but exactly
one matching signing identity: two certificates of a kind in one keychain is an
ordinary state, and picking whichever came first is how a package gets signed
with the wrong one.

`keychain-access-groups` is declined although the profile grants it, for the
reason `AppxManifest.xml` declares only `runFullTrust`: a capability asked for
and unused is a question at review with no good answer.

`build-app.sh` also refuses a binary importing a symbol from a system framework
that the framework's public headers do not declare — the macOS counterpart of
`check-imports.ps1` and `check-libraries.sh`, and there for the same reason all
three exist. A submission was refused under Guideline 2.5.1 for a private
CoreGraphics symbol `winit` links unconditionally and nothing here calls;
`packaging/macos/README.md` has it.

**The signing identities, whose names differ from the portal's labels:**

    Apple Distribution: Excelano LLC (9K6W5PMFYP)
    3rd Party Mac Developer Installer: Excelano LLC (9K6W5PMFYP)
    Developer ID Application: Excelano LLC (9K6W5PMFYP)

The middle one is what the portal calls *Mac Installer Distribution*; it signs
packages rather than code, so it does not appear under `security find-identity
-p codesigning` and its absence there is correct. The profile is `Slipcase Mac
App Store`, platform OSX, naming `9K6W5PMFYP.com.excelano.slipcase-desktop` with
no wildcard, **expiring 2027-08-28**.

**A Store-signed build cannot be launched off the Store**, which
`CHECKLIST.md` explains where a person is about to try it. What follows here:
screenshots can never be of the exact artefact uploaded, so build a Developer ID
bundle from the same commit and say so in `packaging/store-listing.md`, and the
walkthrough against the real article goes through TestFlight.

**A rejection can arrive only by email.** An upload can answer *UPLOAD SUCCEEDED
with no errors* and be refused afterwards with nothing in the web interface
saying so. Check mail after every upload.

---

## The answers both stores ask, which no build supplies

| | |
| --- | --- |
| Price | **Free.** |
| Support URL | `https://excelano.com/slipcase/` |
| Privacy policy URL | `https://excelano.com/legal/#slipcase` |
| Age rating | Every answer None. |
| Export compliance | **No encryption.** Slipcase makes no network request and implements no cryptography; it *reads* containers whose members may be encrypted and refuses those, which is not the same claim. |
| A container for the reviewer | `https://excelano.com/slipcase/quarterly-report.pdf.slpc` |

The last goes in *Notes for certification* telling the tester to open it. It is
`packaging/demo-container.sh`'s output, which pins its archive timestamps so
that any machine rebuilds the same bytes.

**The age rating is answered once and then reused.** IARC generates the ratings
from that questionnaire and they cover the product *and any later change that
would not alter the answers* — so an ordinary patch costs nothing, and anything
that gives Slipcase something to rate means answering it again. The Global
Rating ID it issues is portable to every other storefront that has licensed
IARC, which makes it an identifier rather than a fact: it is in
`packaging/windows/SUBMITTING.local.md` with the rest of them.

**Both pages are on `excelano.com` and are submission blockers.** Check the
served HTML rather than this file — the anchor is `id="slipcase"`, which was
once recorded here wrongly, and the privacy section is
`packaging/privacy-entry.html` pasted in, which has twice gone stale against the
repository's copy.

**Going live makes a third page stale, and nothing prompts you.** `/slipcase/`
says a store listing is *coming* until somebody edits it, so publication is not
finished when the Store says published: the product page has to gain the link
and lose the promise. It is the last step of a release rather than an
afterthought, because it is the page both store forms give as the support URL
and the one a person actually arrives at.

---

## Decisions that still bind

**The storefront name differs per store.** Microsoft Store `Slipcase`; Mac App
Store `Slipcase Desktop`, because `Slipcase` belongs to an insurance news
application on that store since 2016 and there is no trademark route. The
application goes on calling itself Slipcase everywhere a person sees it:
`Package/Properties/DisplayName` and `CFBundleDisplayName` both say so and
neither changes. *Slipcase Viewer* was rejected as an alternative because
`CFBundleTypeRole` is `Editor` precisely because this application writes edited
metadata back, and making that claim to a shopper is worse than making it to
Launch Services.

**Submit with `Blocked executables` failing.** The kit objects to `cmd.exe`
strings from the Rust standard library's batch-file spawn, and to
`ShellExecuteW`, which is `opener` — the Open button, and removing it removes
the application. The kit's own `configuration.xml` marks that task
`OPTIONAL_FOR_APP_TYPES="Centennial"` and the report's root says
`APP_TYPE="Centennial"`, and the kit passes the package with the test failing.
That is evidence and not a guarantee; if review objects, the position is the one
stated here rather than removing the API.

**The command-line tool is not a store product, and on macOS it cannot be.**
`slpc-rust` already distributes it further than this application: cargo-dist
builds five targets including `aarch64-apple-darwin`, with a shell installer, a
PowerShell installer, a Homebrew tap at `excelano/homebrew-tap`, crates.io for
`cargo install`, and `[package.metadata.deb]` for Debian. A sandboxed bundle
cannot put anything on `PATH`, so on the Mac App Store a CLI could only ship
buried inside a container where being on `PATH` is the point. If a store-shaped
Windows presence is ever wanted for it, a winget manifest is the thing to want,
and it is independent of everything here. **So the stores are for the GUI**, and
reserving a second name buys nothing — both stores reclaim reservations that
never receive a build.

**The security review does not need the other two machines.** Nearly every
defect found in one lived in code all three platforms share, and the
platform-specific surface is three `#[cfg]` arms, one module, and the packaging
scripts. What is left needs a person rather than a reviewer, and belongs to a
session already sitting at that machine. **Reading another platform's arm is
still worth doing** — it found the tree's unescaped payload name, and going
looking for the Windows dependency defect's counterpart found the Linux one.

---

## The readiness review, back here

Somebody looks at all three platforms together, which no platform session can
do. What it covers:

- **Every claim in the store listings is true of the built artefacts.** This is
  the error this project has caught most often — a sentence written before
  anybody looked — and it has been caught in the listing, in the changelog and
  twice on the privacy page.
- **The version is the same number in all three spellings**, both changelogs
  name it, and what apt serves is understood against what the stores have.
- **`CHECKLIST.md`'s hand items have been run on every platform they apply to**,
  and anything found is in a commit.
- **The three CI workflows are green**, the corpus agrees on every platform that
  has run it, and `preflight.sh` passes.

Then, and only then, both submissions go in.

---

## Still to write

`packaging/windows/store-metadata.ps1` and `packaging/macos/store-metadata.sh`:
read a submission back from each store's API, so that what the listing says can
be checked against what was actually filed. They are what turns *the listing is
right* from a reading into a check.

**The Windows one should be written smaller than this entry assumed.** It was
here because no submission existed to read; one does now, and most of what it
wanted comes from the two public commands above rather than from the submission
API — no Entra directory, no token, and it runs on any machine. What is left for
the API is the write-only half listed there, and a script that checks the public
half must say plainly that it did not check the other.

Each platform's own decisions are in its directory rather than repeated here:
`packaging/windows/README.md`, `packaging/macos/README.md`, and
`packaging/linux` alongside `packaging/debian`.

---

## What a patch costs, afterwards

Bump `Cargo.toml`, write a changelog entry in both shapes, run `preflight.sh`,
run the three build scripts, upload two packages, `apt-ship` the third. If it
costs more than that, the difference is a defect in this file rather than in the
release, and it belongs here as an amendment.
