# slipcase-desktop — Design Document

**Status:** built through `§7` stage 3.
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
4. **File association**, per platform, per `§8`. **Shipped on Linux.** macOS and Windows are briefed in `HANDOFF-macos.md` and `HANDOFF-windows.md` and have never been built.

Stage 1 is a whole program rather than a preview of one: it opens a container and shows what is in it.

**Amended: stage 1 named a drop as a third way in, and there is none.** winit's Wayland backend carries no data-device plumbing, so a dropped file arrives only under X11 and nothing written here delivers it. The feature was removed rather than shipped working on one display server and silently dead on the other.

---

## 8. Packaging and association

**Naming.** The crate and the binary are `slipcase-desktop`; the application is Slipcase to a person.

**Linux.** A `shared-mime-info` XML declaring `application/x.slipcase+zip` with a glob on `*.slpc`, a desktop entry naming that type, and an icon. The glob is the only identification available: SPEC §4 reserves no magic bytes. Distribution is the Excelano apt repository with the runtime libraries declared as package dependencies.

**Amended: the media type is a subclass of `application/zip`.** A container is a ZIP, so an archive tool can open one and a desktop that knows nothing of slipcases still has something to offer. It costs the glob nothing: where a name matches `*.slpc` and the content sniffs as `application/zip`, shared-mime-info takes the glob, because the magic-matched type is the parent rather than the child. Before the type is installed a `.slpc` reports as `application/zip`, which is true and useless.

**Amended: the runtime libraries cannot be derived from the executable.** It links libc, libm, and libgcc, and nothing else. Everything that draws a window — the Wayland client library, the keyboard map library, the EGL and Vulkan loaders, the X11 libraries — is opened by name at run time, which is the other face of §2's claim that `wayland-sys` and `linux-raw-sys` resolve their symbols then. `dpkg-shlibdeps` sees none of it, so a package built from the linker's answer alone installs cleanly on a machine with no display stack and fails to start. The dependency list is written by hand and was measured by running the application and reading `/proc/PID/maps`.

**Amended: the package carries no maintainer scripts.** `shared-mime-info`, `desktop-file-utils`, and `hicolor-icon-theme` own dpkg triggers on the three directories the package writes into, so the caches are rebuilt without a `postinst` asking. They are dependencies for that as much as for anything they provide at run time.

**macOS.** An application bundle with `CFBundleDocumentTypes` and an exported type declaration conforming to `public.zip-archive`, which a container is.

**Amended: the bundle needs an identifier of its own, and a minimum system version.** `com.excelano.slipcase` names the format and `com.excelano.slipcase-desktop` names the application, because they are different things and conflating them would move the bundle identifier if the format were ever renamed. The second is the reverse-DNS of `APP_ID`, so the binary, the Linux desktop entry's basename, and the bundle all say `slipcase-desktop`. The role is `Editor` rather than `Viewer`, because `§5` writes edited metadata back and `Viewer` would be a claim to the platform that is not true; it is bounded, since the only type claimed is the one the bundle exports. And `LSMinimumSystemVersion` is 12.0, which `§3`'s one function turns out to cost: `URLForApplicationToOpenContentType:` is macOS 12 and later, and below it that selector does not exist. The declaration binds the bundle but not a bare executable, which Cargo builds for 10.12; `MACOSX_DEPLOYMENT_TARGET=12.0` moves it, measured as `minos 12.0`.

**Amended: a double-clicked container is refused on macOS, with a dialog.** `§7` stage 4 assumed association was the whole of the work, and it is not. A person double-clicking one gets *The document could not be opened. Slipcase cannot open files in the "Slipcase container" format.* The association is not what fails: that dialog names this bundle's own `UTTypeDescription`, and `URLForApplicationToOpenURL` on the file returns `Slipcase.app`. Delivery fails. macOS hands an opened document over as an Apple Event rather than as `argv[1]`, `main` reads `std::env::args_os().nth(1)`, and measured, `open some.slpc` launches Slipcase with no arguments at all; AppKit finds nothing willing to take the document and refuses the event. This was first recorded here as opening an empty window, which was measured from a command line where the process starts and the status is zero and the dialog is invisible. The empty window is real and is the smaller half. Receiving it needs `application:openURLs:` on an `NSApplicationDelegate`, and winit 0.30.13 installs its own delegate carrying only `applicationDidFinishLaunching:` and `applicationWillTerminate:`, sets it itself, and exposes no hook; eframe adds nothing. Implementing the method here would mean `unsafe impl NSApplicationDelegate` in this crate's source, which `§2`'s rule forbids without exception. It is therefore not worked around and belongs upstream. The association still earns its place without it: the type resolves, the icon is the bundle's, and the application's own Open dialog works.

**Amended again: the double-click was fixed, and the paragraph above was wrong about why it could not be.** It is right that `NSApplication` has one delegate, that winit owns it, and that implementing `application:openURLs:` here would need unsafe code. What it missed is that the delegate is not the only way in. `NSAppleEventManager` takes a handler for `kAEOpenDocuments` directly, a notification has any number of observers where a delegate is singular, and neither displaces anything of winit's. David agreed the one exception to `§2`'s rule, and `src/opened_document.rs` is it.

The moment of registration is the whole problem and was found by measuring the two that fail. Registering before `NSApplication` exists is overwritten, because AppKit installs its own handler for this event while starting up and that handler is the one that refuses the document: with the registration there, neither a cold launch nor a container double-clicked into a running window arrived. Registering from `eframe`'s creation closure is too late for the launch itself: a container double-clicked into a running window arrived and the one that started the process did not. Between them is `applicationWillFinishLaunching:`, which is where Apple's documentation says to install Apple Event handlers, reached here through a notification observer rather than a delegate method. An instrumented run put `HANDLER FIRED` before the creation closure ran, which is the ordering that explains both failures.

A container opened this way also records its folder for the Open dialog, the same as one chosen in the dialog, because to a person it is the same act.

**Amended: Launch Services and Spotlight do not agree about a registered type.** After `lsregister -f`, `lsregister -dump` reports `com.excelano.slipcase` as exported and tagged `.slpc`, and asking through this application's own `opens_with` returns Slipcase with `isDeclared` true. `mdls -name kMDItemContentType` on the same kind of file still reports the synthesised `dyn.ah62d4rv4ge81g5duqq`, on a file created after registration and after `mdimport` was run against it by hand. The registration record carries an `untrusted` flag, which is what an unsigned bundle's exported type gets, and that is the likeliest reason. Untested, because testing it needs a signature, and recorded rather than guessed at.

**Amended: an unsigned bundle is rejected by Gatekeeper and runs here anyway.** `spctl -a -t exec` reports `rejected`, `source=no usable signature`. A bundle built on this machine carries no `com.apple.quarantine` attribute, so `open` launches it with no prompt, which is why the build-and-test loop works. A bundle that reached a person by download would carry that attribute and be refused. Signing and notarization stay out of scope, and any distribution beyond this machine needs both.

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
