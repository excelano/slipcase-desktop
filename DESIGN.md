# slipcase-desktop — Design Document

**Implements:** Slipcase specification 1.0, through the `slpc` library rather than directly.
**Section references:** `SPEC §2.3` is the specification in `excelano/slipcase`. `slpc-rust §4.4` is the design document in that repository. A bare `§4` is this document. All three number their sections independently, so none is safe to read from context.

---

## 1. What this repository is

A desktop application that opens a `.slpc` file, shows its metadata, hands the payload to whatever application the operating system has registered for it, and makes a container out of a file a person chooses. It is presented to a person as **Slipcase**; the crate and the binary are `slipcase-desktop`, so nothing on `PATH` collides with the command-line tool.

The application parses no containers. Every read, every write, and every verdict comes from `slpc`, the library in `excelano/slpc-rust`: making a container is `slpc::pack_reader` the way opening one is `slpc::validate`. Where it needs behaviour the library lacks, the behaviour goes into the library.

The specification in `excelano/slipcase` is the authority on the format, and this document neither restates nor amends it. Read `SPEC.md` before implementing anything. Do not infer format rules from this document.

---

## 2. Dependencies

- **The framework** — egui, through `eframe`. Chosen on `§3`'s decision to carry no preview surface; with a document renderer in scope the answer would be the platform's own toolkit.
- **The library** — `slpc`, from crates.io at a published version. Never a path dependency in a committed manifest.
- **The metadata editor** — `flyleaf` and `flyleaf-core`, from `excelano/flyleaf`, from crates.io. One editor across the Excelano applications rather than two that drift. It depends on the same `toml_edit` major that `slpc` re-exports and must, since a `DocumentMut` from one is handed to the other.
- **File dialogs** — `rfd`, whose default features are `xdg-portal` and `wayland`, both pure Rust. Its `gtk3` feature links C through `gtk-sys` and must stay off.
- **Launching the payload** — `opener`, which hands a path to the platform's own mechanism.
- **TOML** — none taken directly. `slpc` re-exports the `toml_edit` it is built on.
- **The macOS type question** — `objc2`, `objc2-foundation`, `objc2-app-kit` and `objc2-uniform-type-identifiers`. Launch Services is a C API and `forbid(unsafe_code)` is a property of this crate's own source, so the question goes through bindings that carry the unsafe on our behalf, as `rfd` and `opener` already do. Every call `src/opens_with.rs` makes into them is a safe function. Their features are named rather than taken: `cargo add` enables 176 features of `objc2-foundation`, and compiling that has been seen to segfault inside LLVM's debug information emission.
- **The desktop's light and dark setting, on Linux only** — `zbus`, pure Rust and no build script. It is already compiled on this target under `accesskit_unix`, which arrives through `egui-winit`, so it adds no crate. `default-features` is off, so `async-io` is named rather than taken, which is the feature `accesskit` already turns on.

**Nothing compiles C.** Across the fleet this is a preference, and `~/notes/pure_rust_preference.md` holds the stance and what taking C costs; this application keeps it as a rule for itself, because a build that needs a Rust toolchain and nothing else is what this section is for.

**The check is the artefact, and it is scoped to this tree.** `cargo tree -i cc` is not the check and never was: `cc` arrives under `wayland-backend` and `pkg-config` under `wayland-sys`, both through `eframe` and `rfd`, and both are compiled as ordinary Rust crates, so that command's red is the normal state. The outcome is what the rule means:

    cargo build --release
    target=$(cargo metadata --format-version 1 --no-deps |
        sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')
    ours=$(cargo tree --prefix none | awk '{print $1}' | sort -u)
    find "${target}/release/build" \( -name '*.o' -o -name '*.a' \) |
        while read -r artefact; do
            package=$(basename "$(dirname "$(dirname "$artefact")")" |
                sed 's/-[0-9a-f]\{16\}$//')
            printf '%s\n' "$ours" | grep -qx "$package" && echo "$artefact"
        done                                       # must print nothing
    ldd "${target}/release/slipcase-desktop"       # libc, libgcc, libm

Each artefact is asked which package it belongs to, because `[build] target-dir` can point the whole fleet at one directory and an unscoped `find` then reads other projects' build scripts. Cargo's build directory is `<package>-<16 hex>`, and the count is anchored in the `sed`, because a name stripped by a looser pattern is a package quietly excused from the rule. `linux.yml` keeps the unscoped `find` deliberately: a runner's target directory is its own, so nothing foreign can appear in it.

The margin is one feature. `wayland-backend` builds two C shims when its `log` feature is on, and `wayland-sys` probes pkg-config when `dlopen` is off. Cargo unifies features across the graph, so a new dependency can turn either on without an edit here, which is the day the check has to work.

**Each platform has its own artefact check, because each had the same defect.** A dependency on what the toolchain hands you is invisible from inside the toolchain, and no platform's check sees another's.

`.cargo/config.toml` sets `+crt-static` for `x86_64-pc-windows-msvc` alone, which links the Visual C++ runtime and the UCRT into the binary rather than taking `VCRUNTIME140.dll` from the Visual C++ Redistributable, which is not part of Windows. `packaging/windows/check-imports.ps1` walks the PE import table and refuses any DLL not known to ship with Windows; `build-msix.ps1` will not package a binary that fails it and `windows.yml` runs it on every push. What it costs is stated rather than skipped: a CRT security fix needs Slipcase rebuilt, where a dynamically linked one would take it from a serviced DLL. Declaring a `Microsoft.VCLibs.140.00.UWPDesktop` framework dependency is the alternative, and it answers the paperwork while leaving the application unable to start until the machine acquires something.

`packaging/linux/check-libraries.sh` covers what the running application *opens*, which `ldd` says nothing about and which here is the whole display stack. It runs the window under both backends — a Wayland session never opens libX11 or libxkbcommon-x11 and an X11 session never opens libwayland-client, so no single run exercises more than half the `Depends` list — reads `/proc/PID/maps`, and refuses any library belonging to a package the declared `Depends` does not transitively reach. It carries one recorded exception, `mesa-vulkan-drivers`: with no Vulkan driver visible the application starts and draws through GL, which is what the `libvulkan1 | libgl1` alternative is for.

`packaging/macos/build-app.sh` refuses to bundle an executable importing a symbol from a system framework that the framework's own public headers do not declare. It asks a question rather than matching a list of names Apple has already caught somebody with. `Cargo.toml`'s `[patch.crates-io]` is what keeps `winit` from declaring `_CGSSetWindowBackgroundBlurRadius`, which review reads out of the symbol table whether or not anything calls it and which fat LTO with `-Wl,-dead_strip` does not remove. **A `cargo update` or an `eframe` bump off winit 0.30.13 makes that patch stop applying**, which is the moment to check whether the release it lands on carries the `private-apple-apis` feature gate.

**There is one build script and it compiles nothing.** On `windows-msvc` `build.rs` prints `/MANIFEST:EMBED` and `/MANIFESTINPUT:…` as linker arguments, so the linker already linking the binary embeds `packaging/windows/slipcase-desktop.manifest`. It adds no build dependency. What that buys is that DPI awareness is declared before any of this program's code runs rather than set a moment into it by winit, and that a tool reading the binary can see it — the Windows App Certification Kit reads the manifest rather than the process. What was rejected in `packaging/windows/README.md` is the *resource compiler*, `rc.exe` and `windres`, and embedding through the linker needs none; a second use of this build script is a decision rather than a precedent.

It cross-compiles. `cargo check --target x86_64-pc-windows-msvc` succeeds on a Linux machine carrying no MSVC toolchain, because the build script reads `CARGO_CFG_TARGET_OS` and `CARGO_CFG_TARGET_ENV` rather than `cfg!`, which in a build script answers about the machine doing the building.

**The minimum supported Rust version is 1.95, and it comes from `eframe`** rather than from anything written here. It rises whenever egui raises its own.

## 3. Shape

A single window.

**The metadata is the window.** It gets the space rather than a panel down one side.

**The payload is a card**: its filename, its size, what the operating system says would open it, and the three things that can be done with it. **Open** extracts it and launches whatever the platform registered. **Extract** puts it where the user says and launches nothing. **Replace** puts another file in its place. It is not previewed.

**Showing a container asks for focus on Open**, so a double-click followed by Enter is the whole of the interaction. The focus is asked for once and only where the button is enabled: every frame would pin it there and leave the tree and the Save unreachable from the keyboard, and a focus ring on a disabled button says press me about a button that cannot be pressed. Opening the payload automatically was rejected — it would make a container an autorun archive, which is the thing store review exists to catch.

**The window follows the desktop's light and dark setting, and on Linux it has to ask.** `winit` answers `system_theme()` from `should_use_dark_mode()` on Windows and from `NSApplication`'s `effectiveAppearance` on macOS, and returns `None` on Linux unconditionally, so egui falls back to dark whatever the desktop is set to. `src/system_theme.rs` reads the XDG desktop portal's `org.freedesktop.appearance color-scheme` and follows it, and is empty on the two platforms where the toolkit answers, since pinning a preference there would replace a live answer with a stale one. It follows rather than reading once, because `ThemeChanged` gives the other two platforms that for free.

**No preference is read as light.** The specification defines 0 as no preference, 1 as prefer dark, 2 as prefer light. GNOME's Settings offers Light and Dark and spells them `default` and `prefer-dark`, so a person choosing Light produces 0 and never 2; `prefer-light` has to be set by hand through `gsettings`. Reading 0 as *leave it dark* would give every GNOME user who chose Light the dark card. Light is also what GTK does with `ADW_COLOR_SCHEME_DEFAULT`, so this application looks like its neighbours. The cost is a desktop that is genuinely dark while declaring no preference, where this draws light — and where its GTK neighbours draw light too. Where the portal cannot be reached, nothing is chosen and egui's own fallback stands.

**The card asks before it offers.** Where this build cannot decode the payload, it says so in the card and does not offer the two actions that would have to. Replace stays, because writing over a member does not read it.

**There is no preview surface.** It keeps this a small, pure-Rust, single-binary application. Text and image preview may be added later; PDF, office, and CAD are out permanently.

**What the card says about type is what the platform says.** The application ships no table mapping filenames to types. Where the platform will not answer, the card says nothing rather than guessing, and the Open button still works. The platforms do not answer alike and that is allowed: Linux declines `data.bin` because nothing claims the name, while macOS names Archive Utility, because `com.apple.macbinary-archive` is declared for that extension whether or not the payload is one.

On Linux the question goes to `xdg-mime`. On macOS it goes to Launch Services; where it knows nothing of an extension it synthesises a dynamic type rather than declining, and nothing claims a synthesised type, so the card is silent for the right reason without this code inspecting the type. On Windows it goes to the registry rather than to `AssocQueryString`, along the path the shell itself takes: a per-user `UserChoice` beats the machine-wide association, the `ProgID` either names is looked up for a plain application name, and where the `ProgID` only names an executable the name comes from `Applications\<exe>\FriendlyAppName` or the shell's `MuiCache`. `AssocQueryString` is a raw FFI call and `forbid(unsafe_code)` stays, and a friendly name is as often a resource reference — `@C:\Windows\system32\notepad.exe,-469` — as a string, which would need `SHLoadIndirectString`, another raw call; both shapes are refused rather than shown to a person. The registry also answers more often: across the 260 extensions one machine had an entry for, the two never disagreed, and the registry named an application for 18 that `AssocQueryString` declined.

**On Windows, saying nothing is the common answer.** Of those 260 extensions, 232 got no answer from either route, most being claimed by a packaged application that is not installed. The card saying nothing about a `.png` is the platform having nothing to say, not the question going unasked.

**`forbid` is `deny` in one crate root, for one module.** `src/lib.rs`, where containers are read and written, is `#![forbid(unsafe_code)]` and that does not move. `src/main.rs` is `#![deny(unsafe_code)]`, which differs only in that it can be lifted beneath, and exactly one module lifts it: `src/opened_document.rs`. An `unsafe` block anywhere else in `src/main.rs` is still a hard error naming that `deny`.

**Accessibility.** `eframe` enables AccessKit by default and it stays on.

---

## 4. Rendering the metadata

Arbitrary TOML, with no schema knowledge. Past the two keys SPEC §2.2 requires, the specification defines no vocabulary, so there is nothing to special-case and no allowlist to write.

**The tree is `flyleaf::render`**, called once from `src/main.rs`. It draws the document as a collapsible tree — tables are sections, scalars and arrays are leaves — with one renderer per TOML type rather than per schema: table, array of tables, inline table, array, string, integer, float, boolean, and four datetime shapes. Every row and section carries a button reading its kind, whose menu offers the kinds the value can become; every comment is a field, edited on blur and removed when emptied, above its key. `tests/golden/` holds what the tree draws and what an edit saves, byte for byte, and is regenerated only by decision.

**Which keys are protected is this application's answer, not the tree's.** `RequiredKeys` in `src/lib.rs` answers whether a path is protected and how a protected string reads; the tree knows no key names. The two required keys are protected and their comments are editable, since protection is about a key's name and value.

**A string the tree will not let anybody edit is shown escaped.** SPEC §3 requires a member name be shown with the characters that reorder text escaped, and egui gives a bidirectional formatting character zero advance width, so an unescaped `report<U+202E>fdp.exe` draws as `reportfdp.exe` in the one field nobody is allowed to change. **An editable string is deliberately left alone**: escaping a value somebody can type into writes the escape back into their document the moment the field is touched, and §4's first sentence means the tree has no schema with which to tell a name from any other string.

**An integer is not right-aligned.** Every value starts in the same column. Right-aligning a number puts it against the window's edge, far from its own key and from every value above and below it, and knowing which key a number belongs to is worth more in a metadata document than arithmetic legibility.

**Document order is preserved and never sorted.** Authoring order carries intent. **Comments are shown beside the key they attach to.**

**The tree renders every entry, and that is why this application sets its own limit.** A `ScrollArea` that draws all of its contents is right for a metadata document — the format defines two keys and SPEC §2.2's example is four lines — and wrong for a document written to be expensive: against the densest conformant metadata anybody can write, a container costs about 8.7 KB for every key, of which roughly 1.3 KB is the parsed document and the rest is what egui retains for a row it has been shown. A megabyte of metadata is 891 MB resident, and every such container is conformant.

`slpc` bounds the metadata member at 1 MiB by default, chosen against what parsing costs. This application bounds it at 256 KiB, chosen against what rendering costs, which is the larger number and which the library has no way to know: `LIMITS` in `src/lib.rs` carries the measurement. A container over it is reported undetermined, which is SPEC §6's answer and is not a verdict against the file. A clipped `ScrollArea` is the alternative and is not wrong, only harder — `show_rows` wants a uniform row height and this tree has sections, comment lines and rows that grow with what is typed — and it would buy a limit that still has to exist, because the parse is 1.3 KB of the 8.7 whatever draws it.

---

## 5. Editing

**Metadata: yes.** Through `toml_edit`, so comments, key order, and untouched whitespace survive. Written back through `slpc::Repack` into a `slpc::Destination`, which is the pair the command-line tool's `repack` verb is built on. Not by shelling out to `slipcase`, and not by rebuilding the archive by hand — SPEC §3 requires that members an implementation does not recognize survive a rewrite.

**Nothing the user did not edit is re-serialized.** Dropping a new `Item` over an old one discards its decor, which is the whitespace and the comments attached to it, so a value is changed by assigning into it and restoring the decor it had.

**Whether a document has been edited is decided against itself as parsed**, not against the bytes in the container. `toml_edit` does not quite reproduce a parsed document: a leading byte order mark is dropped and CRLF line endings return as LF, both of which SPEC §2.2 permits, so two of the conformance corpus's conformant containers would be called edited the moment they were opened. The document is handed to `Repack` only where somebody edited it, so a container whose payload alone was replaced keeps its metadata member byte for byte.

**The metadata has undo and redo.** The document is `flyleaf_core::Document`, which holds the edited baseline and a history beside it, a copy of the rendered text per step with edits to one row coalesced. The window records after every frame, takes Ctrl+Z and Ctrl+Shift+Z before anything draws so a focused field cannot answer them, and tells the widget to forget what is half typed first, since a key field commits its buffer on blur. Undo and Redo sit beside Save. The baseline is built from the tree's text rather than the container's bytes, for the reason the paragraph above gives.

**The container is read back before it replaces anything.** `Destination::written` hands back what was just written, and the application validates it there. **A container nothing has changed in is not written.**

**On macOS the rewrite does not wait beside the container.** `Destination::in_place` asks for a randomly-named sibling and renames it over the original, which is what makes the replacement atomic, and under the App Sandbox that sibling cannot be created: the grant a person gives through the open panel covers the file they chose and not the directory holding it. `src/staging.rs` is the one place the three platforms differ. Linux and Windows keep `in_place`. macOS reserves the rewrite through `Destination::new`, validates it there exactly as before, and lands it with `-[NSFileManager replaceItemAtURL:withItemAtURL:…]`, which is the call Apple sanctions for replacing a file a person chose. Both arms use `slpc` for reserving, writing, reading back and committing, and differ only in which public constructor they ask for. No unsafe: the binding is a safe function in `objc2-foundation`.

**Where the rewrite waits is the platform's choice, not this application's.** `replaceItemAtURL:` wants both of its ends on one volume, and `tempfile::TempDir` answers `TMPDIR`, so a container on any other volume refused with `NSCocoaErrorDomain` 512 over `EXDEV` — measured against mounted images formatted APFS, HFS+, FAT32 and exFAT, all four. The directory is asked for with `NSItemReplacementDirectory` and `appropriateForURL:`, which lands on the volume the container is on; for a container on the boot volume it returns one inside the same per-user temporary area, so the sandbox property this module exists for is unchanged.

**New container… asks which file goes in and where the container goes, and nothing else.** It packs through `slpc::pack_reader` and shows the result. The metadata it writes is empty, and the two keys SPEC §2.2 requires are the library's to fill in. What a person wants to say about the payload they say in the tree afterwards: §4's first sentence is that there is no schema here, so a form asking for anything else would be this application inventing a vocabulary the specification does not define.

Neither question may be guessed. Which file goes in is the whole of what is being asked for, and where the container lands is a location — `slipcase pack` defaults to writing beside the payload, which is a convention a command line can afford and a button cannot. The second dialog is a save dialog, so the platform asks before overwriting and the answer to that question is the only permission this needs. The naming convention is offered rather than imposed: the dialog is prefilled with the payload's name and `.slpc` after it. Nothing here reads a container's name to find out what is inside it.

**The payload's mark comes with it, and without that, packing is a way to launder a download.** A container this process writes carries no mark of its own, so packing a gated file, opening the container — which the card would then call local — and pressing Open would hand the operating system an unmarked copy of a file it had gated, every step being somebody's ordinary use of the window. `create` carries the payload's mark onto the container, and `tests/handover.rs` walks the whole trip rather than the container alone, because what matters is the file the platform is handed at the end of it. A carry that fails is said and not fatal, which is this section's decision for `staging.rs` too: what opens a container is this application, which reports provenance rather than acting on it.

**One write at a time.** Making a container ends by showing it, exactly as opening one does, so a person who opened a container while one was being packed would watch it replaced a moment later, taking any unsaved edit with it. A pack under way disables the same presses the window already disables for a dialog, the card's three included.

**Payload: extract and replace, as explicit actions.** No temporary-file watching and no save interception. The user says when they are done and the application does not guess.

**A replacement waits for a Save.** Choosing the file is not writing it. It waits beside the metadata edits so that one press writes one container. Writing on the press would rewrite the same archive twice with a window between the two where a failure leaves half of what was asked for, and it would put an arbitrarily large write inside a file dialog.

**Opening extracts to a temporary directory owned by the process** and removed when it exits. **Extracting writes where the user named**, which is the difference between the two actions: the application chooses a location only when nobody asked it to. It never writes beside the container it opened.

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
| Conformant, payload encrypted or compressed by a method this build lacks | conformant | everything, and a refusal where the payload would have been |

**Every tree above comes from `slpc::metadata_of`**, which parses the metadata member alone; `Container::read` fails the payload check before it yields a document. The verdict comes from `slpc::validate`.

**The library reaches every one of these and the application renders it.** A container whose metadata cannot be read may not be reported conformant or non-conformant, and one declaring a version this build does not implement may not be reported conformant to a version it does.

**A payload of zero length is conformant** under SPEC §2.3 and the card says nothing about it beyond its size. **A very large payload** makes extraction work with a duration, so the Open button reports progress and can be cancelled.

**A conformant container's payload may still be out of reach.** SPEC §2.5 puts encryption and compression method outside conformance, so a container carrying an encrypted payload is sound and its bytes cannot be had. The card describes it in full, because the name and the size come from the central directory and need no decoder, and it states the refusal before anything is pressed: `Container::check_payload_readable` answers from that same entry, so asking costs nothing. Open and Extract are not offered; Replace is. The corpus holds the two answers against each other — every payload the card refuses must fail to extract in the same words, and every payload it offers must extract.

**A path that cannot be read at all is not a row above.** Every row there is something a container can be, and this is something a path can be. `slpc::validate` returns every verdict as `Ok` and reserves `Err` for not reaching the bytes: a path that is not there, a directory, a file the process may not read. The window states it in a line of its own and shows nothing further.

---

## 7. Provenance

**A payload leaves a container carrying what the container carried.** A container downloaded from the internet carries `com.apple.quarantine` on macOS and a `Zone.Identifier` stream on Windows, both consulted before a file is opened and both properties of the file rather than of its contents, so a payload written without them reaches its handler as something this machine made and the warning the platform would have shown never appears. That is the shape of defect that made disk images and archives a delivery vehicle.

**The mechanism is `slpc`'s and the policy is this application's.** `slpc::provenance::carry`, `provenance::arrived_from_elsewhere` and `Mark` live in the library, because `slipcase unpack` had the identical defect in the sibling doing the identical operation. What stays here is the use: `copy_out` decides that a payload whose provenance could not be carried is not left on disk, and the card decides to report rather than gate.

**Saving carries it too.** `Destination::in_place` replaces a container by renaming a fresh file over it, and a fresh file carries no mark, so editing one key in a downloaded container and pressing Save would strip what the platform had recorded — and every payload extracted afterwards would be unmarked, because carrying copies from the container. `slpc`'s `commit` carries the mark onto the replacement before the rename, and `fs` implies `provenance` so that asking for the ability to replace a container cannot quietly mean asking for the bug. `src/staging.rs`'s macOS arm does not use `in_place`, so it carries the mark itself, onto the staged file before `replaceItemAtURL:`, rather than trusting that call to preserve an attribute nobody has measured it preserving.

**Carrying fails only where the copy would be ungated.** `carry` fails where the platform gates opening on a mark, the source carries one, and the copy ends up carrying none; where the copy is already marked, by whatever put it there, it succeeds and says `Mark::AlreadyMarked`. Laundering is a payload reaching its handler looking like something this machine made, not the absence of one particular value. A copy the platform marked is gated, so the harm does not arise; what is lost is the detail of which agent downloaded what, which is a fact this application no longer knows rather than a control it has given up. The check asks the file and not the process, so it is one branch on all three platforms and nothing asks whether it is sandboxed.

**A mark this application wrote is not provenance.** Under a sandbox the platform marks whatever this process writes, so saving an edit marks the container, and a predicate asking only whether a mark exists would tell a person that a container they made here had arrived from elsewhere. `carry` needs to know whether the copy is **gated**; the card needs to know whether the container **came from somewhere**. The card's answer disregards a mark whose agent is this application, reading the agent field out of `flags;timestamp;agent;event-uuid` and comparing it against the running executable's own filename. That is a change of stance about a value otherwise treated as opaque, confined to one field read for one comparison: nothing is rewritten, and copying the value verbatim is still the rule. Every uncertainty answers that the file arrived from elsewhere, because over-reporting provenance costs a line of caution and under-reporting it is what this section exists to prevent.

**On Windows the two questions are two functions for a reason.** `std::fs::write` creates the stream and then writes into it, so a write that fails partway leaves a stream that exists carrying no `ZoneId`, and a stream with no `ZoneId` is not something the shell stops for — which would make the `AlreadyMarked` fallback call the copy safe and hand the payload over ungated.

**What the shell stops for is measured rather than reasoned about.** Refused: a `ZoneId` of 3, 4 or 99 in a `[ZoneTransfer]` section, in either case, with spaces around the `=`, with `\n` alone for a line ending, with no trailing line ending, and after other keys. Allowed: 0, 1, 2, -3, an empty value, and a `ZoneId` under any other section or under none, so the section header carries weight and a `ZoneId` that merely exists is not a gate. Where two `ZoneId` lines disagree the last one decides. The predicate is that, with one deliberate difference: a value that is not a number at all gates on the platform and reads here as no gate, because being wrong that way costs a refusal to extract and being wrong the other way is laundering. The card's question is unchanged and still over-reports, because anything written into that stream is evidence something wrote it and nothing on Windows writes one on this application's behalf.

**The card reports and nothing is disabled.** Disabling the Open button for a marked container buys nothing: both paths produce the same file with the same mark, the platform gates the handler rather than the launcher, and the cost falls entirely on the common case, a container that arrived by download being exactly the one somebody wants to look inside. It would also be this application substituting its own judgement about what is dangerous for the platform's, which §3 refuses to do about type. On Windows this is measured: a marked file in a temporary directory is treated exactly as one anywhere else, so a temporary copy is not a trusted copy. The warning is shown for file types the shell treats as risky and not for every payload — a PDF reaches its handler with the mark on it and no prompt.

**Under a sandbox the platform marks everything this application writes**, so a payload extracted from a container made on this machine still comes out marked. macOS consults the mark only when something is about to execute, so a PDF and a text file open without a word and a shell script is refused as *damaged and can't be opened*, with advice to bin a file that is fine. That is the platform's treatment of every sandboxed application's output. The unsandboxed build refuses the same payload for the other reason, the 0644 that `copy` writes, so an executable payload does not run from the Open button on either build.

**The card says so from the container rather than from the message.** *The payload is an executable file; the extracted copy will not be executable.* It appears when the payload member's stored mode carries an execute bit and not otherwise, so it is silent for a container written by a tool that records no mode — the shape §3 already requires of the platform's silence. Gated to Unix, because a mode bit is not what makes a file executable on Windows. It reads `Container::payload_mode`, which reads the external attributes off the central directory and answers only where the high sixteen bits carry something: `unix_mode()` is not a safe base, because `zip` invents `S_IFREG | 0o664` for an archive made on DOS and only an archive whose external attributes are entirely zero comes back `None`, so every container written by a Windows tool would be told confidently that its payload is not executable.

Three alternatives are rejected. **Preserving the execute bit on extraction** is the tempting one and is not the fix it looks like: under the sandbox the copy is quarantined whatever its mode, so the message is unchanged, and what it would change is that opening a container could produce an executable file — the wrong direction for an application whose provenance rule exists to stop a payload arriving as something this machine made. **Disabling the Open button** is what this section already declines. **Doing nothing** leaves a person advised to bin a file that is fine, when the container held the fact that explains it. What the line deliberately does not do is predict a platform's message; it says what is true of the file.

---

## 8. Packaging and association

**Naming.** The crate and the binary are `slipcase-desktop`; the application is Slipcase to a person. The release configuration here is this repository's own and not the command-line tool's.

**The media type is the registered one, and only on Linux is the provisional name kept.** IANA registered `application/vnd.excelano.slipcase+zip` on 2026-09-16 and recorded `application/x.slipcase+zip` as a deprecated alias; SPEC §4 names the registered type and this repository neither restates nor amends it. The type string appears in no container, so nothing already written changes and what changes is every place this repository states the type to a platform. shared-mime-info has an `<alias>` element, so `slipcase-common` declares the registered type and keeps the old name resolving to it, and an installation predating the registration goes on opening containers from a file manager. macOS and Windows have no such element: a second `public.mime-type` string is a second tag of equal standing, and a second `MIME\Database` key is a second claim on the extension rather than a pointer to the first. Each would have to be carried and removed for good, and what it would buy is the file that arrives carrying the old type and no name to be typed by — which is not how a file is typed on either platform, where the extension decides. The old name is therefore dropped on both, and `install.ps1` deletes the key an upgrade would otherwise leave behind.

**Extraction addresses a path the platform will accept.** A payload name can be legal, safe to join, and still not a file: Win32 resolves `CON`, `COM1`, `AUX`, `LPT1`, `PRN` and `NUL` to devices wherever they appear, so `File::create` returns `Ok`, `write_all` returns `Ok`, and no file exists afterwards, while `std::fs::read` never returns at all — it opens the console for reading and waits for input a windowed application will never supply. A path in the `\\?\` verbatim form is not parsed that way, so `slpc::payload_path` builds one from a canonicalized directory and the name stops being a device without anybody deciding it is a bad name. Nothing here holds a list of reserved names: which names are devices is Windows's to know.

It does not touch `extract_at`, because that path is one a person typed and §5 says the two halves of extraction differ in whose name it is. And it does not make the payload openable: `opener::open` on a device-named file fails with *the specified device name is invalid*, which is an error the application has a sentence for.

**A sentence this application composes is this application's to get right.** `opener::OpenError` keeps its `Display` to a category and puts the platform's words in `source()`, so `why` in `src/main.rs` prefers the source: the card says *The specified device name is invalid. (os error 1200)* rather than *IO error*. **And `slpc::display_path` takes the verbatim prefix off for display and nothing else does**, because *Extracted to* would otherwise show a person a spelling they have never seen and could not type; every filesystem call keeps the form that works. That is a presentation rule, not a naming one.

### Linux

A `shared-mime-info` XML declaring `application/vnd.excelano.slipcase+zip` with a glob on `*.slpc`, a desktop entry naming that type, and an icon. The glob is the only identification available: SPEC §4 reserves no magic bytes. Distribution is the Excelano apt repository with the runtime libraries declared as package dependencies.

**The media type is a subclass of `application/zip`**, so an archive tool can open a container and a desktop that knows nothing of slipcases still has something to offer. It costs the glob nothing: where a name matches `*.slpc` and the content sniffs as `application/zip`, shared-mime-info takes the glob, the magic-matched type being the parent rather than the child.

**The media type and the container icon are `slipcase-common`'s**, and both products depend on it. `slipcase-open` claims the same association, two packages cannot ship one path, and dpkg refuses the second install outright — so the type could be declared in one product or the other and not both, leaving every container on a machine with the other product drawing as `application-x-generic`. `sub-class-of application/zip` does not help there: it makes `content_type_is_a` answer true and carries no icon with it. What stays here is the application icon, which is a different role.

**The runtime libraries cannot be derived from the executable.** It links libc, libm and libgcc and nothing else; everything that draws a window is opened by name at run time, which is the other face of §2's claim about `wayland-sys` and `linux-raw-sys`. `dpkg-shlibdeps` sees none of it, so the dependency list is written by hand and measured by running the application and reading `/proc/PID/maps`.

**The package carries no maintainer scripts.** `desktop-file-utils` and `hicolor-icon-theme` own dpkg triggers on the two directories the package writes into, so the caches are rebuilt without a `postinst` asking. They are dependencies for that as much as for anything they provide at run time.

### macOS

An application bundle with `CFBundleDocumentTypes` and an exported type declaration conforming to `public.zip-archive`, which a container is. The role is `Editor` rather than `Viewer`, because §5 writes edited metadata back, and it is bounded, since the only type claimed is the one the bundle exports.

`com.excelano.slipcase` names the format and `com.excelano.slipcase-desktop` names the application, because they are different things and conflating them would move the bundle identifier if the format were ever renamed. `LSMinimumSystemVersion` is 12.0, which §3's one function costs: `URLForApplicationToOpenContentType:` is macOS 12 and later. The declaration binds the bundle but not a bare executable, which Cargo builds for 10.12, so `MACOSX_DEPLOYMENT_TARGET=12.0` moves it. `LSApplicationCategoryType` is declared, which App Store Connect requires.

**macOS hands an opened document over as an Apple Event rather than as `argv[1]`**, so `src/opened_document.rs` takes a `kAEOpenDocuments` handler from `NSAppleEventManager` directly. The delegate is not the way in: `NSApplication` has one, winit owns it and exposes no hook, and implementing `application:openURLs:` here would mean `unsafe impl NSApplicationDelegate` in this crate's source. A notification has any number of observers where a delegate is singular, and neither displaces anything of winit's.

**The moment of registration is the whole problem.** Registering before `NSApplication` exists is overwritten, because AppKit installs its own handler for this event while starting up and that handler is the one that refuses the document, so neither a cold launch nor a container double-clicked into a running window arrives. Registering from `eframe`'s creation closure is too late for the launch itself: the double-click into a running window arrives and the one that started the process does not. Between them is `applicationWillFinishLaunching:`, which is where Apple's documentation says to install Apple Event handlers, reached here through a notification observer. A container opened this way records its folder for the Open dialog, the same as one chosen in the dialog, because to a person it is the same act.

**An exported type is `untrusted` until the bundle is signed**, and Spotlight refuses an untrusted one: `mdls -name kMDItemContentType` reports the synthesised dynamic type rather than the declared one until there is a signature, at which point `lsregister -dump` flags the type `trusted` and `kMDItemKind` reports the bundle's own `UTTypeDescription`. Any signature does it; a distribution certificate is not what fixes it.

**The channel is the Mac App Store**, because Finder offers *Search App Store* by document type, which is what a person who has been sent a container actually does, and outside the Store that search returns nothing. Every Store binary is sandboxed, which is what §5's staging arm and §7's provenance rule are for; the handover itself survives, since `opener` forks `/usr/bin/open`, the child inherits the sandbox, and Launch Services is reachable over Mach IPC from inside it. The App Sandbox is inert until the entitlement is inside a signature, so signing is not separable from any of it, and `build-app.sh --sign` does it with the entitlements the repository carries. The bundle is universal, `lipo`-joined from two `--target` builds, and the script refuses one that lost an architecture or whose slices disagree with the `LSMinimumSystemVersion` the property list declares.

### Windows

The extension and the media type are registered by two PowerShell scripts, everything under `HKEY_CURRENT_USER`, which is the counterpart of the Linux script's default of `~/.local` and needs no administrator. There is no all-users variant, because the machine-wide half of every key needs elevation and a script that sometimes needs it is worse than one that never does.

MSI through WiX, Inno Setup and NSIS are each rejected for the same reason: every one needs a toolchain that is not on a stock Windows and not in this repository's build, to produce a package that would do what forty lines of registry writes do — the application is one executable, one icon, and no runtime files of its own.

**The channel is the Microsoft Store and the format is MSIX**, taken for the reason macOS took the Mac App Store: Windows offers to search the Store by file type when a person double-clicks something nothing is registered for. That does not revive WiX, which builds MSI. The two PowerShell scripts stay, because a Store listing is no reason to withdraw the per-user route from somebody who wants no account.

**A stale `UserChoice` is the dead association, not the ProgID.** What an uninstaller leaves behind is `Explorer\FileExts\.slpc\UserChoice`, which "always open with" writes and which outranks every other key; removing the class keys and leaving it points the extension at a ProgID that no longer exists, and Windows treats that as no association at all rather than falling back to the machine-wide one. It is the same behaviour §3 records for `opens_with`, seen from the other side.

**The UserChoice is deleted by name from its parent, and only when it names this application.** Explorer writes a *Deny SetValue* rule on that key so that no application can quietly take an extension over, and both `DeleteSubKeyTree` and `reg delete` open the key itself for writing before deleting it, so both fail against the rule — `reg delete` says *Access is denied* and .NET reads the same failure as the key being missing and returns quietly. Deleting the name from the parent needs DELETE on the child and nothing else, which the rule beside the deny allows, unelevated. Every delete in `uninstall.ps1` is read back, and a survivor throws: a delete that fails and a key that was never there must not look alike. `check-install.ps1` holds it, planting a `UserChoice` the way Explorer writes one, deny rule and all, and `windows.yml` runs it.

**Windows has no `APP_ID`, and the window icon is not a resource.** `with_app_id` is Wayland's `xdg_toplevel.set_app_id` and neither egui, eframe nor winit turns it into anything here. Windows' own notion is the AppUserModelID, and setting one needs `SetCurrentProcessExplicitAppUserModelID`, a raw call `forbid(unsafe_code)` puts out of reach. It is left alone on both sides: with neither the process nor the Start menu shortcut declaring one, Windows derives both from the executable's path, they agree, and pinning and grouping work — setting it on the shortcut alone would break that pairing rather than fix it. The window's icon has the same shape of problem and a different answer: Windows reads it from a resource compiled into the executable, §2 keeps `rc.exe` and `windres` out of the build, so the `.ico` is carried by `include_bytes!` and handed to the window at startup. That is why a rasterized icon is a committed artifact in a repository that otherwise holds only sources.

**The executable is GUI-subsystem**, or a file manager launching it opens a black console window behind the application.

---

## 9. Non-goals

**A batch mode, a library view, or anything that walks a directory tree.**

**Signing, encryption, and fixity.** SPEC §5 leaves all three out of this version of the format.

**A drop as a way in.** winit's Wayland backend carries no data-device plumbing, so a dropped file arrives only under X11, and a feature that works on one display server and is silently dead on the other is worse than none.

---

## 10. The language a person reads

The window draws in German where the desktop asks for German, and in English
everywhere else. The mechanism is the `potext` crate; `i18n` in `src/lib.rs` is
where this application's catalogue is declared. This section is what the rest of
the application is allowed to assume.

**A message is looked up by its English text, never by a key.** `t("Save")`
returns the German for *Save* or, where there is none, `Save` itself. So a call
site reads as the sentence a person sees, an untranslated message is the
original rather than a placeholder, and a catalogue that has fallen behind the
source degrades to English one message at a time.

**A translation that has gone stale is not shown.** When a message's English
changes, `msgmerge` carries the old German onto the new text and marks it
`#, fuzzy`; `potext` will not load a fuzzy entry, so the window falls back to
English until a person has looked at it. This is the property the whole choice
of format rests on, and it is why `.po` beat a key-value catalogue that has no
way to say *this was true of a sentence we no longer show*.

**`po/update-po.sh` is the only way the catalogues move.** It re-reads every
string out of `src/`, merges each catalogue, and refuses one that will not
compile. Run it after changing any sentence a person reads.

**`po/pseudo.sh` writes a run to do by hand.** The pseudolocale translates
nothing and changes everything, so a string that never went through `t`, and a
layout built to the width of English, both show themselves on sight.

**The tree carries its own strings.** `flyleaf::render` draws inside this window
and `main` hands it the tag it read from the platform, so the tree is in the
same language as the window around it. A language tag crosses, never a
catalogue, so a version skew between the two costs nothing. The TOML type names
the tree shows are translated: they are the editor's words rather than the
format's, and the tell is that it says *text* where TOML says *string*.

**A verdict stays in English.** `Outcome::Unreadable` translates *cannot be
read* and leaves the reason after the colon as `slpc` wrote it, and a judged
container states itself in `Verdict`'s own words. Restating either here would be
a table mapping the library's sentences to German, which is the library worked
around. Closing it means translating `slpc`, and that is a decision about that
repository.

**The store listings and the two other platforms' package metadata** are each a
separate object. A `.desktop` entry carries its own translations and this
repository's does; the macOS bundle wants `CFBundleLocalizations` and the MSIX
package a second language in `<Resources>` with `makepri /dq` agreeing; each
store's listing is a separate object in its own console that no manifest
controls. Those are edits inside the macOS and Windows arms.

---

## License

MIT, matching `slpc-rust` and the specification's tooling.
