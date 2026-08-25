# slipcase-desktop — Design Document

**Status:** built through `§7` stage 4, on all three platforms.
**Document version:** 2026-08-21
**Amendments:** this document was written before anything was built, and building it contradicted parts of it. Every change since 2026-08-20 is marked **Amended** and states what was measured. A design that quietly rewrote itself to match the code would be worth nothing as a record.
**Implements:** slipcase specification 1.0, through the `slpc` library rather than directly.
**Section references:** `SPEC §2.3` is the specification in `excelano/slipcase`. `slpc-rust §4.4` is the design document in that repository. A bare `§4` is this document. All three number their sections independently, so none is safe to read from context.

---

## 1. What this repository is

A desktop application that opens a `.slpc` file, shows its metadata, and hands the payload to whatever application the operating system has registered for it. It is presented to a person as **Slipcase**; the crate and the binary are `slipcase-desktop`, so nothing on `PATH` collides with the command-line tool.

The application parses no containers. Every read, every write, and every verdict comes from `slpc`, the library in `excelano/slpc-rust`. Where it needs behavior the library lacks, the behavior goes into the library.

The specification in `excelano/slipcase` is the authority on the format, and this document neither restates nor amends it. Read `SPEC.md` before implementing anything. Do not infer format rules from this document.

---

## 2. Dependencies

- **The framework** — egui, through `eframe`. Chosen on `§3`'s decision to carry no preview surface; with a document renderer in scope the answer would be the platform's own toolkit.
- **The library** — `slpc`, from crates.io at a published version. Never a path dependency in a committed manifest.
- **File dialogs** — `rfd`, whose default features are `xdg-portal` and `wayland`, both pure Rust. Its `gtk3` feature links C through `gtk-sys` and must stay off.
- **Launching the payload** — `opener`, which hands a path to the platform's own mechanism.
- **TOML** — none taken directly. `slpc` re-exports the `toml_edit` it is built on.

**Amended: the Open button takes the keyboard when a container is shown.** `§3` gives the card three buttons and says nothing about which one a person reaches first. The common thing to do with a container just opened is to open what is in it, so Enter now does that: showing a container asks for focus on Open, and pressing Enter is the whole of the interaction from a double-click onwards. This is also what was chosen instead of two other proposals. Opening the payload automatically on a double-click was rejected, because it would make a container an autorun archive and would be the thing store review exists to catch; disabling Open for a container that arrived from elsewhere was rejected in `§5`. One keystroke is the middle: the deliberate act survives, and it costs a press rather than a reach for the mouse. The focus is asked for once and only where the button is enabled — every frame would pin it there and leave the tree and the Save unreachable from the keyboard, and a focus ring on a disabled button says press me about a button that cannot be pressed. Both are tests, and both were broken to watch them fail.

**Amended: answering on macOS costs four dependencies.** `§3` describes the platform question as one function returning an optional string, which on Linux it is, because `xdg-mime` is a command and costs nothing to ask. Launch Services is a C API and `#![forbid(unsafe_code)]` is a property of this crate's own source, so the question goes through `objc2`, `objc2-foundation`, `objc2-app-kit`, and `objc2-uniform-type-identifiers`, which carry that unsafe on our behalf the way `rfd` and `opener` already do. Every call `src/opens_with.rs` makes into them is a safe function, and the module compiles under `forbid` with no unsafe block in it. Three of the four are already in the tree on that target under `winit`, `rfd`, and `arboard`, so one crate is new. Their features are named rather than taken, and not for weight alone: `cargo add` enables 176 features of `objc2-foundation`, and rustc 1.98.0 on `x86_64-apple-darwin` segfaults inside LLVM's debug information emission compiling that, at the default stack size and again at the larger one it suggests. Nothing added here compiles C.

**Nothing compiles C.** `cc`, `cmake`, `pkg-config`, and `bindgen` are all absent from the build-dependency tree. `wayland-sys` and `linux-raw-sys` are declaration crates that resolve their symbols at run time. A build needs a Rust toolchain and nothing else.

**It cross-compiles.** `cargo check --target x86_64-pc-windows-msvc` succeeds on a Linux machine carrying no MSVC toolchain.

**The minimum supported Rust version is 1.95, and it comes from `eframe`** rather than from anything written here. Measured, and expected to rise whenever egui raises its own.

---

## 3. Shape

A single window.

**The metadata is the window.** It gets the space rather than a panel down one side.

**The payload is a card**: its filename, its size, what the operating system says would open it, and the three things that can be done with it. **Open** extracts it and launches whatever the platform registered. **Extract** puts it where the user says and launches nothing. **Replace** puts another file in its place. It is not previewed.

**The card asks before it offers.** Where this build cannot decode the payload, it says so in the card and does not offer the two actions that would have to. Replace stays, because writing over a member does not read it. The alternative is to offer everything and let a person find out by pressing, which costs them a dialog and a wait before the same sentence appears.

**There is no preview surface.** It keeps this a small, pure-Rust, single-binary application. Text and image preview may be added later if they are wanted; PDF, office, and CAD are out permanently.

**What the card says about type is what the platform says.** The application ships no table mapping filenames to types. On Linux the question goes to `xdg-mime`, on macOS to Launch Services, on Windows to `AssocQueryString`, each behind one function returning an optional string. Where the platform will not answer, the card says nothing rather than guessing, and the Open button still works.

**Amended: `forbid` became `deny` in one crate root, for one module.** `§2` treated unsafe in this crate's own source as impossible rather than merely refused, and the difference matters once something needs it. `src/lib.rs`, where containers are read and written, is still `#![forbid(unsafe_code)]`. `src/main.rs` is `#![deny(unsafe_code)]`, which differs only in that it can be lifted beneath, and exactly one module lifts it: `src/opened_document.rs`, which receives the document macOS will not pass as an argument. Checked rather than assumed — an `unsafe` block put anywhere else in `src/main.rs` is still a hard error naming that `deny`. Three more `objc2-foundation` features and `block2` come with it, and `cargo tree -i cc` is still empty.

**Amended: the two platforms that answer do not answer alike, and one of them invents.** `data.bin` is the case. Linux declines it, because nothing claims that name and the two placeholders `mime_of` writes then disagree, while macOS names Archive Utility, because `com.apple.macbinary-archive` is declared for that extension whether or not the payload is one. Both are the platform's own answer rather than this application's, which is what this section asks for, so the same container legitimately reads differently on two machines. Where macOS knows nothing of an extension it does not decline either: it synthesises a dynamic type, `dyn.ah62d4rv4ge81g5duqq` for `slpc` until `§8` registers one. Nothing claims a synthesised type, so Launch Services names no application and the card is silent for the right reason without this code having to inspect the type. A declared type reaching no application is that same silence, measured on `xlsx` on a machine carrying nothing that opens it.
**Amended: on Windows the question does not go to `AssocQueryString`.** It goes to the registry, along the path the shell itself takes: a per-user `UserChoice` beats the machine-wide association, the `ProgID` either of them names is looked up for a plain application name, and where the `ProgID` only names an executable the name comes from `Applications\<exe>\FriendlyAppName` or from the shell's own `MuiCache`. Two things forced it and one thing settled it. `AssocQueryString` is a raw FFI call and `#![forbid(unsafe_code)]` stays, so the choice was a safe registry crate or nothing. And the friendly name is a resource reference — `@C:\Windows\system32\notepad.exe,-469`, or `@{Package?ms-resource://...}` for a packaged application — as often as it is a string, and following one is `SHLoadIndirectString`, another raw call; both shapes are refused rather than shown to a person. What settled it is that the registry answers more often than the API does: measured across the 260 extensions this machine has an entry for, the two never disagreed, the registry named an application for 18 that `AssocQueryString` declined — `.zip`, `.msi`, and `.ps1` among them — and `AssocQueryString` named one for a single extension the registry route missed, `.m4a`, whose handler is a packaged application whose name is only in its manifest.

**Amended: on Windows, saying nothing is the common answer and usually the right one.** Of those 260 extensions, 232 got no answer from either route. Most are claimed by a packaged application that is not installed: this machine's `.txt`, `.png`, and `.jpg` all name a `ProgID` with no key behind it, and `AssocQueryString` answers nothing for them too rather than falling back to the machine-wide `txtfile`, so neither does this. The card saying nothing about a `.png` is not the question going unasked. It is the platform having nothing to say, which §3 already permits and which Windows reaches far more often than Linux does.

**Accessibility.** `eframe` enables AccessKit by default and it stays on.

---

## 4. Rendering the metadata

Arbitrary TOML, with no schema knowledge. Past the two keys SPEC §2.2 requires, the specification defines no vocabulary, so there is nothing to special-case and no allowlist to write.

The document renders as a collapsible tree: tables are sections, scalars and arrays are leaves.

**One renderer per TOML type, not per schema.** The types `toml_edit` distinguishes are table, array of tables, inline table, array, string, integer, float, boolean, and four datetime shapes: offset date-time, local date-time, local date, and local time. A datetime formats itself, a boolean is a checkbox, a string is a string.

**Amended: an integer is not right-aligned.** Every value starts in the same column, integers included. Right-aligning a number put it against the window's edge, far from its own key and from every value above and below it, and the arithmetic legibility that buys is worth less in a metadata document than knowing which key a number belongs to. A tree with one such row measured 916 pixels wide in a 900-pixel window, and the delete button beside it fell off the edge.

**Document order is preserved and never sorted.** Authoring order carries intent.

**Comments are shown beside the key they attach to.**

---

## 5. Editing

**Metadata: yes.** Through `toml_edit`, so comments, key order, and untouched whitespace survive. Written back through `slpc::Repack` into a `slpc::Destination`, which is the pair the command-line tool's `repack` verb is built on. Not by shelling out to `slipcase`, and not by rebuilding the archive by hand — SPEC §3 requires that members an implementation does not recognize survive a rewrite.

**Amended: on macOS the rewrite does not wait beside the container.** `Destination::in_place` asks `NamedTempFile` for a randomly-named sibling of the container and renames it over the original when it is committed, which is the whole of what makes the replacement atomic. Under the App Sandbox that sibling cannot be created: the grant a person gives through the open panel covers the file they chose and not the directory holding it, so Save stopped with *Operation not permitted*, measured 2026-08-25 and recorded in `CHECKLIST.md`. `src/staging.rs` is the one place the three platforms now differ. Linux and Windows keep `in_place`. macOS reserves the rewrite in a scratch directory of its own through `Destination::new`, validates it there exactly as before, and lands it with `-[NSFileManager replaceItemAtURL:withItemAtURL:…]`, which is the call Apple sanctions for replacing a file a person chose and which preserves the original's metadata the way `in_place` promised to. It is not a way around the library: both arms use `slpc` for reserving, writing, reading back, and committing, and differ only in which public constructor they ask for. No unsafe: the binding is a safe function in `objc2-foundation`, which was already here for `NSBundle` and the Apple Event, so `NSFileManager` and `NSError` are two features of a crate in the tree rather than anything new in it. Measured the same day, by hand, under the sandbox that refused the old path: the edit is written, the container reads back conformant, and the comment attached to an untouched key survives the round trip.

**And it costs a false statement, which the sandbox and not this change is the author of.** The staged file is created by a sandboxed process, so the platform marks it `com.apple.quarantine` the way it marks everything such a process writes, and `replaceItemAtURL:` carries the new file's mark onto the original along with its bytes — `com.apple.macl` and the last-used date belonging to the original survive, the quarantine attribute does not. A container that was local before it was edited is therefore reported as having arrived from elsewhere the moment it is saved, and the card says so in as many words. It is cosmetic beside the extraction failure above and it is the same platform behaviour reaching the same code from the other side, so `§5`'s provenance policy has one more thing to answer for when it is reopened: under a sandbox, a mark on a file this application wrote says nothing about where the contents came from.

**The container is read back before it replaces anything.** `Destination::written` hands back what was just written, and the application validates it there.

**Nothing the user did not edit is re-serialized.** Dropping a new `Item` over an old one discards its decor, which is the whitespace and the comments attached to it, so a value is changed by assigning into it and restoring the decor it had.

**Amended: `toml_edit` does not quite reproduce a parsed document byte for byte.** Two of the conformance corpus's 37 conformant containers come back changed by a parse and a re-serialization alone: a leading byte order mark is dropped, and CRLF line endings return as LF. SPEC §2.2 permits both. Two things follow. Whether a document has been edited is decided by comparing it against itself as parsed rather than against the bytes in the container, or those two would be called edited the moment they were opened. And the document is handed to `Repack` only where somebody edited it, so a container whose payload alone was replaced keeps its metadata member byte for byte.

**Amended: one key did not keep its comment, and now does.** `Repack` moves `payload.file` when a payload is replaced under a new name, and it moved it by assignment, so the comment and the whitespace attached to that value were discarded — the mistake the paragraph above exists to avoid, made inside the library. Filed as `slpc-rust#2` and fixed in `slpc` 0.3.4, which also leaves the key alone where it already holds the name being written. Measured with this application's own guard removed, the disagreement it caused across the corpus fell from three containers to the two the paragraph above accounts for.

**A container nothing has changed in is not written.**

**Payload: extract and replace, as explicit actions.** No temporary-file watching and no save interception. The user says when they are done and the application does not guess.

**A replacement waits for a Save.** Choosing the file is not writing it. It waits beside the metadata edits so that one press writes one container. Writing on the press would rewrite the same archive twice with a window between the two where a failure leaves half of what was asked for, and it would put an arbitrarily large write inside a file dialog.

**Opening extracts to a temporary directory owned by the process** and removed when it exits. **Amended: extracting writes where the user named**, which is the difference between the two actions — the application chooses a location only when nobody asked it to. It never writes beside the container it opened.

---

## 6. States to design, not to crash on

SPEC §2 and §3 define these conditions. The list is not exhaustive, and none of them is a dialog box or a panic; each is a state the window renders.

| State | The library's verdict | What the window can show |
|---|---|---|
| Not an archive at all | non-conformant | the verdict |
| No metadata member, or more than one | non-conformant | the verdict |
| Metadata present, not UTF-8 or not TOML | non-conformant | the verdict |
| Metadata parses, a required key is absent | non-conformant | the verdict, and the tree |
| `payload.file` names no member, or more than one | non-conformant | the verdict, and the tree |
| `payload.file` is not a name a payload may have | non-conformant | the verdict, and the tree |
| The metadata member cannot be read at all | undetermined | the verdict, and nothing further |
| A `slipcase_version` this build does not implement | out of scope | the verdict, and the tree |
| Conformant, payload of zero length | conformant | everything, size stated plainly |
| Conformant, payload very large | conformant | everything, extraction that can be waited on |
| **Amended:** conformant, payload encrypted or compressed by a method this build lacks | conformant | everything, and a refusal where the payload would have been |

**Every tree above comes from `slpc::metadata_of`**, which parses the metadata member alone; `Container::read` fails the payload check before it yields a document. The verdict comes from `slpc::validate`.

**The library reaches every one of these and the application renders it.** A container whose metadata cannot be read may not be reported conformant or non-conformant, and one declaring a version this build does not implement may not be reported conformant to a version it does.

**A payload of zero length is conformant** under SPEC §2.3 and the card says nothing about it beyond its size.

**A very large payload** makes extraction work with a duration, so the Open button reports progress and can be cancelled.

**Amended: a conformant container's payload may still be out of reach.** SPEC §2.5 puts encryption and compression method outside conformance, so a container carrying an encrypted payload is sound and its bytes cannot be had. The card describes it in full, because the name and the size come from the central directory and need no decoder, and it states the refusal before anything is pressed: `Container::check_payload_readable` answers from that same entry, so asking costs nothing. Open and Extract are not offered; Replace is, because nothing has to read a member to write over it. One of the corpus's 37 conformant containers is this, and the corpus holds the two answers against each other — every payload the card refused failed to extract in the same words, and every payload it offered extracted.

**Amended: a path that cannot be read at all is not in the table above.** Every row there is something a container can be, and this is something a path can be. `slpc::validate` returns every verdict as `Ok` and reserves `Err` for not reaching the bytes: a path that is not there, a directory, a file the process may not read. The window states it in a line of its own and shows nothing further. Nothing in the conformance corpus reaches it, because every case there is a container.

---

## 7. Build order

1. **Open and look.** Open a container from an argument and from a file dialog; render the metadata tree; render the payload card with a working Open button; render every state in `§6`. **Shipped.**
2. **Editing the metadata**, and writing it back with `Repack` into a `Destination::in_place`. **Shipped.**
3. **Extracting and replacing the payload**, as the two explicit actions `§5` describes. **Shipped.**
4. **File association**, per platform, per `§8`. **Shipped on all three.** Each was built and looked at on the platform itself rather than cross-compiled and assumed; `packaging/` holds what each decided and the amendments below say where the platforms disagreed. The one place this stage cost more than association was macOS, which does not deliver a double-clicked document as an argument at all — recorded twice below, once wrongly.

Stage 1 is a whole program rather than a preview of one: it opens a container and shows what is in it.

**Amended: stage 1 named a drop as a third way in, and there is none.** winit's Wayland backend carries no data-device plumbing, so a dropped file arrives only under X11 and nothing written here delivers it. The feature was removed rather than shipped working on one display server and silently dead on the other.

---

## 8. Packaging and association

**Naming.** The crate and the binary are `slipcase-desktop`; the application is Slipcase to a person.

**Linux.** A `shared-mime-info` XML declaring `application/x.slipcase+zip` with a glob on `*.slpc`, a desktop entry naming that type, and an icon. The glob is the only identification available: SPEC §4 reserves no magic bytes. Distribution is the Excelano apt repository with the runtime libraries declared as package dependencies.

**Amended: the media type is a subclass of `application/zip`.** A container is a ZIP, so an archive tool can open one and a desktop that knows nothing of slipcases still has something to offer. It costs the glob nothing: where a name matches `*.slpc` and the content sniffs as `application/zip`, shared-mime-info takes the glob, because the magic-matched type is the parent rather than the child. Before the type is installed a `.slpc` reports as `application/zip`, which is true and useless.

**Amended: extraction carries the container's provenance, because it was laundering it.** `§5` describes extraction as a copy and says nothing about what the platform records on the file being copied. It records a great deal. A container downloaded from the internet carries `com.apple.quarantine` on macOS and a `Zone.Identifier` stream on Windows, both consulted before a file is opened and both properties of the file rather than of its contents — so the payload written by this application carried neither, and reached its handler as something this machine had made. The warning the platform would have shown never appeared. That is the shape of defect that made disk images and archives a delivery vehicle, and it was live in the explicit Open button long before anything was proposed about double-clicking. `src/provenance.rs` carries the mark, and the policy lives there rather than in the caller: carrying fails only where the platform gates opening on a mark the container carried and it could not be written to the copy, so an error means the payload is not opened and is not left on disk. Windows needs no API for it — a stream is addressed by appending `:Zone.Identifier` to the path, so `std::fs` reaches it and nothing is FFI. macOS needs the `xattr` crate, which is pure Rust with its unsafe inside it as `rfd` and `opener` already have theirs, and which adds exactly one crate because `rustix`, `bitflags`, and `linux-raw-sys` were already in the lockfile. Linux has no counterpart that gates anything, so it carries `user.xdg.origin.url` and `user.xdg.referrer.url` as a note and says so with a separate answer, rather than reporting a control it does not have. `SPEC.md` §3 declines this question deliberately — *nothing else here is a security requirement* — so it is this document's to decide.

**Amended again: under the App Sandbox the platform carries it, and refuses to let us.** Measured 2026-08-25 against a bundle signed with `com.apple.security.app-sandbox`; `CHECKLIST.md` holds the run. Two things the paragraph above could not have known. A sandboxed process has its writes marked for it — a payload extracted from a container carrying no `com.apple.quarantine` at all came out carrying one, `0086` in the application's own temporary directory and `0082` at a location chosen through the save panel, both naming `slipcase-desktop`, with `provenance::carry` having returned `Silent` and written nothing. So the laundering this section exists to prevent cannot happen under a sandbox whatever this code does. And `xattr::set` of that attribute is then refused, most likely because the file already carries the platform's own mark and replacing one quarantine value with another is how forgery would work. That refusal is not survivable as written: `copy_out` fails the whole extraction when carrying fails, deliberately, so a container that arrived from elsewhere can be neither extracted nor opened under a sandbox — the exact containers a Store build would exist to serve. The policy has to learn that a platform which has already marked the file has done the job, which is a decision about §5 rather than a repair, and it is not taken here.

**Amended: the rule became a test of the copy rather than of this module's own success.** The paragraph above leaves Extract and Open failing outright under a sandbox for every container that arrived from elsewhere, which is the whole set this section exists for, so the rule was reopened rather than lived with. `carry` now fails only where the platform gates opening on a mark, the source carries one, and **the copy ends up carrying none** — where the copy is already marked, by whatever put it there, it succeeds and says `Mark::AlreadyMarked`. What this section calls laundering is a payload reaching its handler looking like something this machine made, and the warning that then never appears. It is not the absence of one particular value. A copy the platform marked is gated, so the harm does not arise; what is lost is the detail of which agent downloaded what, which is a fact this application no longer knows rather than a control it has given up. The check asks the file and not the process, so it is one branch on all three platforms and nothing anywhere asks whether it is sandboxed — the environment test was rejected for the reason this document keeps rejecting them. Two tests hold it in both directions and both were broken to watch them fail: a marked copy is not a failure, and an unmarked one still is.

**Amended: and a mark this application wrote is not provenance.** The change above left one thing standing. Under a sandbox the platform marks whatever this process writes, so saving an edit marks the container, and a predicate that asked only whether a mark existed then told a person that a container they made here had arrived from elsewhere. Two questions had been one function because nothing until now wrote a mark of its own: `carry` needs to know whether the copy is **gated**, and the card needs to know whether the container **came from somewhere**. They are separate now. The card's answer disregards a mark whose agent is this application, reading the agent field out of `flags;timestamp;agent;event-uuid` and comparing it against the running executable's own filename, which is what the platform was measured to write there. That is a change of stance about a value this module otherwise treats as opaque, and it is confined to one field read for one comparison: nothing is rewritten, and copying the value verbatim is still the rule. Every uncertainty answers that the file arrived from elsewhere, because over-reporting provenance costs a line of caution and under-reporting it is what `§5` exists to prevent.

The card reports it and nothing is disabled, which was decided rather than assumed. Disabling the Open button for a marked container was proposed, so that a person had to extract the payload and open it themselves. It buys nothing now that the mark is carried: both paths produce the same file with the same mark, the platform gates the handler rather than the launcher, and the only difference is the number of clicks — while the cost falls entirely on the common case, a container that arrived by download being exactly the one somebody wants to look inside. It would also be this application substituting its own judgement about what is dangerous for the platform's, which `§3` refuses to do about type and should not start doing about provenance. So the card carries one line in the warning colour and the person decides. **This rests on one thing that has not been measured**: that the platform treats a marked file in a temporary directory the same as one anywhere else. `CHECKLIST.md` asks for it on both platforms that have a mark, and if the answer is that a temporary copy is trusted, this paragraph is wrong and the button should be disabled after all.

**Amended: the runtime libraries cannot be derived from the executable.** It links libc, libm, and libgcc, and nothing else. Everything that draws a window — the Wayland client library, the keyboard map library, the EGL and Vulkan loaders, the X11 libraries — is opened by name at run time, which is the other face of §2's claim that `wayland-sys` and `linux-raw-sys` resolve their symbols then. `dpkg-shlibdeps` sees none of it, so a package built from the linker's answer alone installs cleanly on a machine with no display stack and fails to start. The dependency list is written by hand and was measured by running the application and reading `/proc/PID/maps`.

**Amended: the package carries no maintainer scripts.** `shared-mime-info`, `desktop-file-utils`, and `hicolor-icon-theme` own dpkg triggers on the three directories the package writes into, so the caches are rebuilt without a `postinst` asking. They are dependencies for that as much as for anything they provide at run time.

**macOS.** An application bundle with `CFBundleDocumentTypes` and an exported type declaration conforming to `public.zip-archive`, which a container is.

**Amended: the bundle needs an identifier of its own, and a minimum system version.** `com.excelano.slipcase` names the format and `com.excelano.slipcase-desktop` names the application, because they are different things and conflating them would move the bundle identifier if the format were ever renamed. The second is the reverse-DNS of `APP_ID`, so the binary, the Linux desktop entry's basename, and the bundle all say `slipcase-desktop`. The role is `Editor` rather than `Viewer`, because `§5` writes edited metadata back and `Viewer` would be a claim to the platform that is not true; it is bounded, since the only type claimed is the one the bundle exports. And `LSMinimumSystemVersion` is 12.0, which `§3`'s one function turns out to cost: `URLForApplicationToOpenContentType:` is macOS 12 and later, and below it that selector does not exist. The declaration binds the bundle but not a bare executable, which Cargo builds for 10.12; `MACOSX_DEPLOYMENT_TARGET=12.0` moves it, measured as `minos 12.0`.

**Amended: a double-clicked container is refused on macOS, with a dialog.** `§7` stage 4 assumed association was the whole of the work, and it is not. A person double-clicking one gets *The document could not be opened. Slipcase cannot open files in the "Slipcase container" format.* The association is not what fails: that dialog names this bundle's own `UTTypeDescription`, and `URLForApplicationToOpenURL` on the file returns `Slipcase.app`. Delivery fails. macOS hands an opened document over as an Apple Event rather than as `argv[1]`, `main` reads `std::env::args_os().nth(1)`, and measured, `open some.slpc` launches Slipcase with no arguments at all; AppKit finds nothing willing to take the document and refuses the event. This was first recorded here as opening an empty window, which was measured from a command line where the process starts and the status is zero and the dialog is invisible. The empty window is real and is the smaller half. Receiving it needs `application:openURLs:` on an `NSApplicationDelegate`, and winit 0.30.13 installs its own delegate carrying only `applicationDidFinishLaunching:` and `applicationWillTerminate:`, sets it itself, and exposes no hook; eframe adds nothing. Implementing the method here would mean `unsafe impl NSApplicationDelegate` in this crate's source, which `§2`'s rule forbids without exception. It is therefore not worked around and belongs upstream. The association still earns its place without it: the type resolves, the icon is the bundle's, and the application's own Open dialog works.

**Amended again: the double-click was fixed, and the paragraph above was wrong about why it could not be.** It is right that `NSApplication` has one delegate, that winit owns it, and that implementing `application:openURLs:` here would need unsafe code. What it missed is that the delegate is not the only way in. `NSAppleEventManager` takes a handler for `kAEOpenDocuments` directly, a notification has any number of observers where a delegate is singular, and neither displaces anything of winit's. David agreed the one exception to `§2`'s rule, and `src/opened_document.rs` is it.

The moment of registration is the whole problem and was found by measuring the two that fail. Registering before `NSApplication` exists is overwritten, because AppKit installs its own handler for this event while starting up and that handler is the one that refuses the document: with the registration there, neither a cold launch nor a container double-clicked into a running window arrived. Registering from `eframe`'s creation closure is too late for the launch itself: a container double-clicked into a running window arrived and the one that started the process did not. Between them is `applicationWillFinishLaunching:`, which is where Apple's documentation says to install Apple Event handlers, reached here through a notification observer rather than a delegate method. An instrumented run put `HANDLER FIRED` before the creation closure ran, which is the ordering that explains both failures.

A container opened this way also records its folder for the Open dialog, the same as one chosen in the dialog, because to a person it is the same act.

**Amended: Launch Services and Spotlight do not agree about a registered type.** After `lsregister -f`, `lsregister -dump` reports `com.excelano.slipcase` as exported and tagged `.slpc`, and asking through this application's own `opens_with` returns Slipcase with `isDeclared` true. `mdls -name kMDItemContentType` on the same kind of file still reports the synthesised `dyn.ah62d4rv4ge81g5duqq`, on a file created after registration and after `mdimport` was run against it by hand. The registration record carries an `untrusted` flag, which is what an unsigned bundle's exported type gets, and that is the likeliest reason. Untested, because testing it needs a signature, and recorded rather than guessed at.

**Amended: they agree once the bundle is signed, and the suspicion was right.** Measured 2026-08-25 against a bundle signed with an Apple Development certificate and registered from a build directory: `lsregister -dump` now flags `com.excelano.slipcase` `trusted` where it flagged it `untrusted`, `mdls -name kMDItemContentType` reports the declared type rather than the synthesised one, and `kMDItemKind` reports `Slipcase container`, which is the bundle's own `UTTypeDescription`. So an unsigned bundle's exported type is what Spotlight was refusing, exactly as the paragraph above guessed and declined to assert. A distribution certificate is not what fixes it; any signature is, which is why this cost nothing to answer once signing was in the packaging.

**Amended: an unsigned bundle is rejected by Gatekeeper and runs here anyway.** `spctl -a -t exec` reports `rejected`, `source=no usable signature`. A bundle built on this machine carries no `com.apple.quarantine` attribute, so `open` launches it with no prompt, which is why the build-and-test loop works. A bundle that reached a person by download would carry that attribute and be refused. Signing and notarization stay out of scope, and any distribution beyond this machine needs both.

**Amended: the Mac App Store was measured before it was chosen, and it costs two changes.** `§8` says nothing about a channel, and signing was recorded above as out of scope on the evidence of `spctl` alone. The Store is worth more than that evidence assumed: Finder offers *Search App Store* by document type, which is what a person who has been sent a container actually does, and outside the Store that search returns nothing. Every Store binary is sandboxed, so the question is what a sandbox refuses, and three paths were run against one. The handover survives — `opener` forks `/usr/bin/open`, the child inherits the sandbox, Launch Services is reachable over Mach IPC from inside it, and a claim here that the exec would be denied was wrong. `Destination::in_place` does not: it creates a randomly-named sibling of the container, and the grant a person gives through the open panel covers the file and not its directory. `NSFileManager.replaceItemAt…` is the answer and needs no sibling, taking its replacement from the application's own container. Carrying provenance does not survive either, for the reason `§5` now records. Neither change is cosmetic and both are David's, so the channel stays open rather than chosen. The account is not the obstacle: it already ships two iOS applications and no distribution certificate was needed to measure any of this.

**Amended: the channel was then chosen, and it is the Store.** The paragraph above left it open because both changes were David's to take, and both were taken the same day: `src/staging.rs` for the save and `§5`'s rule for provenance, each measured under a sandbox before and after. What the decision then bought and cost is recorded in `packaging/macos/README.md` rather than restated here, with three things worth naming. Signing stopped being out of scope, because it never really was — the App Sandbox is inert until the entitlement is inside a signature, so every measurement already depended on it, and `build-app.sh --sign` now does it with the entitlements the repository carries. The bundle is universal, `lipo`-joined from two `--target` builds, and the script refuses one that lost an architecture or whose slices disagree with the `LSMinimumSystemVersion` the property list declares — measured, because the first universal build said 10.12 and 11.0 under a plist saying 12.0. And `LSApplicationCategoryType` is declared, which App Store Connect requires and which `§8` had no reason to think about while there was no channel.

**Windows.** The extension and the media type registered by the installer.

**Amended: the Windows installer is two PowerShell scripts, and it is per-user.** One sentence stood here for a whole platform, so most of this was decided while building it. MSI through WiX, Inno Setup, and NSIS were each rejected for the same reason: every one needs a toolchain that is not on a stock Windows and not in this repository's build, to produce a package that would do what forty lines of registry writes do — the application is one executable, one icon, and no runtime files of its own. They become worth building when there is a channel to ship through, the way `packaging/debian` exists because the apt repository does; there is no such channel for Windows and inventing what it will want is guessing. Everything is written under `HKEY_CURRENT_USER`, which is the counterpart of the Linux script's default of `~/.local` and needs no administrator; there is no all-users variant, because the machine-wide half of every key needs elevation and a script that sometimes needs it is worse than one that never does.

**Amended: a stale `UserChoice` is the dead association, not the ProgID.** The thing an uninstaller leaves behind is not the class key it forgot to delete but `Explorer\FileExts\.slpc\UserChoice`, which "always open with" writes and which outranks every other key. Removing the class keys and leaving it points the extension at a ProgID that no longer exists, and Windows does not then fall back to the machine-wide association — it treats the extension as having none at all. Measured, and it is the same behaviour §3's amendment records for `opens_with`, seen from the other side. Uninstalling was checked with a `UserChoice` deliberately in place, and it removes it.

**Amended: Windows has no `APP_ID`, and the window icon is not a resource.** `with_app_id` is Wayland's `xdg_toplevel.set_app_id`, and neither egui, eframe, nor winit turns it into anything on Windows — measured by reading all three. Windows' own notion is the AppUserModelID, and setting one needs `SetCurrentProcessExplicitAppUserModelID`, a raw call `#![forbid(unsafe_code)]` puts out of reach. It is therefore left alone on both sides: with neither the process nor the Start menu shortcut declaring one, Windows derives both from the executable's path, they agree, and pinning and grouping work. Setting it on the shortcut alone would have broken that pairing rather than fixed it. The window's icon has the same shape of problem and a different answer: Windows reads it from a resource compiled into the executable, `rc.exe` and `windres` are the tools for that and §2 keeps both out of the build, so the `.ico` is carried by `include_bytes!` and handed to the window at startup instead. That is why a rasterized icon is a committed artifact in a repository that otherwise holds only sources.

**Amended: the first frame Windows ever drew showed two defects, and neither was reachable from a test.** The executable was console-subsystem, so a file manager launching it opened a black console window behind the application; and the window carried the generic default icon. Both were found by installing, double-clicking a container, and looking at the screen — the walkthrough §7 asks for and `CHECKLIST.md` now records. 61 tests and the whole conformance corpus would have passed with both in place.

**Not the command-line tool's pipeline.** The release configuration here is this repository's own.

---

## 9. Non-goals

**A batch mode, a library view, or anything that walks a directory tree.**

**Signing, encryption, and fixity.** SPEC §5 leaves all three out of this version of the format.

---

## License

MIT, matching `slpc-rust` and the specification's tooling.
