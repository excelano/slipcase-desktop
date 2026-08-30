# slipcase-desktop

A desktop application for [slipcase](https://github.com/excelano/slipcase) containers.

A `.slpc` file is a ZIP archive holding a payload file of any type together with a TOML metadata document describing it. The two become one file, so copying, moving, or sending the payload carries its metadata along.

Slipcase opens a container, shows what is in it, and hands the payload to whatever application the operating system has registered for it. It parses no containers itself: every read, every write, and every verdict comes from [`slpc`](https://github.com/excelano/slpc-rust).

The specification lives in `excelano/slipcase` and is the authority on the format.

## Build

```
cargo build --release
```

A Rust toolchain is all it needs. Nothing in the dependency tree compiles C.

## Install

On Linux, the desktop integration and the media type:

```
./packaging/linux/install.sh
```

That registers `application/x.slipcase+zip` against `*.slpc`, so a file manager
knows what a container is and what opens one. `packaging/debian/build-deb.sh`
builds the package the Excelano apt repository ships. `packaging/README.md` has
the detail.

## Status

All four stages of `DESIGN.md` §7 ship: opening a container and rendering every
state the design names, editing the metadata and writing it back, extracting and
replacing the payload, and file association.

Association ships on all three platforms, each built and walked through by hand
on the platform itself. `packaging/` holds what each decided and
`CHECKLIST.md` records what only a hand could test — thirteen defects between them that the tests and the conformance corpus
passed over. Two things measured on macOS are recorded unresolved: an unsigned
bundle is refused by Gatekeeper, and Spotlight and Launch Services disagree
about a registered type.

`DESIGN.md` is what this is and the order it is being built in. It is amended in
place where building it proved it wrong, and every amendment says what was
measured.

## Testing

```
cargo test
cargo clippy --all-targets
```

The slipcase conformance corpus is run as a command rather than as a test,
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
