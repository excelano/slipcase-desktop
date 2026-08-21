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

## Status

Under construction. `DESIGN.md` is what this is and the order it is being built in.

## License

MIT
