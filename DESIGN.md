# slipcase-desktop — Design Document

**Status:** designed, not built.
**Document version:** 2026-08-20
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

**Nothing compiles C.** `cc`, `cmake`, `pkg-config`, and `bindgen` are all absent from the build-dependency tree. `wayland-sys` and `linux-raw-sys` are declaration crates that resolve their symbols at run time. A build needs a Rust toolchain and nothing else.

**It cross-compiles.** `cargo check --target x86_64-pc-windows-msvc` succeeds on a Linux machine carrying no MSVC toolchain.

**The minimum supported Rust version is 1.95, and it comes from `eframe`** rather than from anything written here. Measured, and expected to rise whenever egui raises its own.

---

## 3. Shape

A single window.

**The metadata is the window.** It gets the space rather than a panel down one side.

**The payload is a card**: its filename, its size, what the operating system says would open it, and an Open button that extracts it and launches that. It is not previewed.

**There is no preview surface.** It keeps this a small, pure-Rust, single-binary application. Text and image preview may be added later if they are wanted; PDF, office, and CAD are out permanently.

**What the card says about type is what the platform says.** The application ships no table mapping filenames to types. On Linux the question goes to `xdg-mime`, on macOS to Launch Services, on Windows to `AssocQueryString`, each behind one function returning an optional string. Where the platform will not answer, the card says nothing rather than guessing, and the Open button still works.

**Accessibility.** `eframe` enables AccessKit by default and it stays on.

---

## 4. Rendering the metadata

Arbitrary TOML, with no schema knowledge. Past the two keys SPEC §2.2 requires, the specification defines no vocabulary, so there is nothing to special-case and no allowlist to write.

The document renders as a collapsible tree: tables are sections, scalars and arrays are leaves.

**One renderer per TOML type, not per schema.** The types `toml_edit` distinguishes are table, array of tables, inline table, array, string, integer, float, boolean, and four datetime shapes: offset date-time, local date-time, local date, and local time. A datetime formats itself, a boolean is a checkbox, an integer is right-aligned, a string is a string.

**Document order is preserved and never sorted.** Authoring order carries intent.

**Comments are shown beside the key they attach to.**

---

## 5. Editing

**Metadata: yes.** Through `toml_edit`, so comments, key order, and untouched whitespace survive. Written back through `slpc::Repack` into a `slpc::Destination`, which is the pair the command-line tool's `repack` verb is built on. Not by shelling out to `slipcase`, and not by rebuilding the archive by hand — SPEC §3 requires that members an implementation does not recognize survive a rewrite.

**The container is read back before it replaces anything.** `Destination::written` hands back what was just written, and the application validates it there.

**Nothing the user did not edit is re-serialized.** `toml_edit` reproduces a parsed document byte for byte when nothing in it has changed. Dropping a new `Item` over an old one discards its decor, which is the whitespace and the comments attached to it, so a value is changed by assigning into it and restoring the decor it had.

**A container nothing has changed in is not written.**

**Payload: extract and replace, as explicit actions.** No temporary-file watching and no save interception. The user says when they are done and the application does not guess.

**Extraction goes to a temporary directory owned by the process** and removed when it exits. The application never writes beside the container it opened.

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

**Every tree above comes from `slpc::metadata_of`**, which parses the metadata member alone; `Container::read` fails the payload check before it yields a document. The verdict comes from `slpc::validate`.

**The library reaches every one of these and the application renders it.** A container whose metadata cannot be read may not be reported conformant or non-conformant, and one declaring a version this build does not implement may not be reported conformant to a version it does.

**A payload of zero length is conformant** under SPEC §2.3 and the card says nothing about it beyond its size.

**A very large payload** makes extraction work with a duration, so the Open button reports progress and can be cancelled.

---

## 7. Build order

1. **Open and look.** Open a container from an argument, from a file dialog, and from a drop; render the metadata tree; render the payload card with a working Open button; render every state in `§6`. **Ships here.**
2. **Editing the metadata**, and writing it back with `Repack` into a `Destination::in_place`.
3. **Extracting and replacing the payload**, as the two explicit actions `§5` describes.
4. **File association**, per platform, per `§8`.

Stage 1 is a whole program rather than a preview of one: it opens a container and shows what is in it.

---

## 8. Packaging and association

**Naming.** The crate and the binary are `slipcase-desktop`; the application is Slipcase to a person.

**Linux.** A `shared-mime-info` XML declaring `application/x.slipcase+zip` with a glob on `*.slpc`, a desktop entry naming that type, and an icon. The glob is the only identification available: SPEC §4 reserves no magic bytes. Distribution is the Excelano apt repository with the runtime libraries declared as package dependencies.

**macOS.** An application bundle with `CFBundleDocumentTypes` and an exported type declaration conforming to `public.zip-archive`, which a container is.

**Windows.** The extension and the media type registered by the installer.

**Not the command-line tool's pipeline.** The release configuration here is this repository's own.

---

## 9. Non-goals

**A batch mode, a library view, or anything that walks a directory tree.**

**Signing, encryption, and fixity.** SPEC §5 leaves all three out of this version of the format.

---

## License

MIT, matching `slpc-rust` and the specification's tooling.
