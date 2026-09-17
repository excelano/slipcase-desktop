# slipcase-desktop

A desktop application for [Slipcase](https://slipcaseformat.org) containers.

A `.slpc` file is a ZIP archive holding a payload file of any type together with a TOML metadata document describing it. The two become one file, so copying, moving, or sending the payload carries its metadata along.

Slipcase opens a container, shows what is in it, and hands the payload to whatever application the operating system has registered for it. It makes one too: *New container…* asks for a file and writes a container around it, which the metadata editor then fills in. It parses no containers itself: every read, every write, and every verdict comes from [`slpc`](https://github.com/excelano/slpc-rust).

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
`packaging/store-listing.toml` carries both names and `packaging/submission-notes.md` the reasoning. And the
`?mt=12` is the Mac-software media type rather than decoration: without it the
link can route a visitor to the iOS store.

From crates.io, `cargo install slipcase-desktop` builds and installs the
binary alone, on any platform with a Rust toolchain. It opens a container from
the command line and does everything the packaged application does, but a
crate carries no desktop entry and no media type, so a double-click in a file
manager does not reach it until the two steps below have run as well.

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

Opening a container, editing its metadata, extracting and replacing the payload
and file association all ship on Linux, macOS and Windows. Making a container
out of a file ships on Linux.

The window draws in German where the desktop asks for German, with English
falling back wherever a sentence has none. `DESIGN.md` §10 is the mechanism and
the two places English still shows; `po/` holds the catalogues and the two
commands that keep them current.

## Testing

```
cargo test
cargo clippy --all-targets
```

The Slipcase conformance corpus is run as a command rather than as a test,
because it needs a checkout of `excelano/slipcase` with its cases generated:

```
cargo run --example corpus -- /path/to/slipcase/conformance
```

It puts every fixture through this application's own reading of them: the
verdict, whether a metadata tree and a payload card are shown, extraction at the
declared length, the pre-flight answer against what extraction then does, a full
rewrite round trip with key order preserved, a rename, a payload replacement
under two names, and every payload packed into a container of its own and read
back.

## License

MIT
