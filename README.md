# slipcase-desktop

A desktop application for [Slipcase](https://slipcaseformat.org) containers.

A `.slpc` file is a ZIP archive holding a payload file of any type together with a TOML metadata document describing it. The two become one file, so copying, moving, or sending the payload carries its metadata along.

Slipcase opens a container, shows what is in it, and hands the payload to whatever application the operating system has registered for it. It parses no containers itself: every read, every write, and every verdict comes from [`slpc`](https://github.com/excelano/slpc-rust).

The specification lives in [`excelano/slipcase`](https://github.com/excelano/slipcase) and is the authority on the format. <https://slipcaseformat.org> publishes it as pages.

## Build

```
cargo build --release
```

A Rust toolchain is all it needs. Nothing in the dependency tree compiles C.

## Install

**On macOS, Slipcase is on the
[Mac App Store](https://apps.apple.com/us/app/slipcase-desktop/id6806461555?mt=12);
on Windows it is in the Microsoft Store**, and on Linux it is in the Excelano apt
repository. That is where a person should get it rather than from here.
`https://excelano.com/slipcase/` is the product page for all three, and what
follows is the from-source route, which is what this repository is for.

Two things about that link. It lists as **Slipcase Desktop** rather than
*Slipcase*, because the name was taken on that store alone —
`packaging/store-listing.md` carries both names and the reasoning. And the
`?mt=12` is the Mac-software media type rather than decoration: without it the
link can route a visitor to the iOS store.

On Linux, the media type and then the desktop integration:

```
../slipcase-common/install.sh
./packaging/linux/install.sh
```

`slipcase-common` registers `application/x.slipcase+zip` against `*.slpc` and
ships the icon a container is drawn with, so a file manager knows what a
container is; this application's own entry says what opens one. The type is a
separate package because every Slipcase product needs it and only one of them
can ship it — two packages cannot install the same path.
`packaging/debian/build-deb.sh` builds the package the Excelano apt repository
ships. `packaging/README.md` has the detail.

## Status

All four stages of `DESIGN.md` §7 ship: opening a container and rendering every
state the design names, editing the metadata and writing it back, extracting and
replacing the payload, and file association.

Association ships on all three platforms, each built and walked through by hand
on the platform itself. `packaging/` holds what each decided and `CHECKLIST.md`
records what only a hand could test, along with the defects the tests and the
conformance corpus passed over — that list is the authority on how many, and
this sentence deliberately does not say.

Both macOS items this paragraph used to carry as unresolved are closed.
Spotlight and Launch Services agreed once the bundle was signed, measured
2026-08-25 and recorded in `packaging/macos/README.md`; the Gatekeeper line
described what an *unsigned* bundle does, which is expected rather than a
defect, and no released build is unsigned.

`DESIGN.md` is what this is and the order it is being built in. It is amended in
place where building it proved it wrong, and every amendment says what was
measured.

## Testing

```
cargo test
cargo clippy --all-targets
```

The Slipcase conformance corpus is run as a command rather than as a test,
because it needs a checkout of `excelano/slipcase` with its cases generated:

```
cargo run --bin corpus -- /path/to/slipcase/conformance
```

It puts every fixture through this application's own reading of them: the
verdict, whether a metadata tree and a payload card are shown, extraction at the
declared length, the pre-flight answer against what extraction then does, a full
rewrite round trip with key order preserved, a rename, and a payload
replacement under two names.

## License

MIT
