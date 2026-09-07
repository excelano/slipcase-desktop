# The goldens

What the metadata editor draws and what it saves, recorded before the editor
was extracted into `excelano/flyleaf`, so that every step of that extraction
can fail on "slipcase-desktop behaves as before" rather than assert it.
`tests/golden.rs` is the test; this file is the record.

`fixtures/` holds the documents: four metadata members copied out of the
conformance corpus's accept cases (comments and blank lines, a multi-line inline
table, nested tables, top-level keys), the every-type document from
`src/tree.rs`'s own tests, and a long-comment document for the truncation
case. `render/` holds every shape egui emitted for each, at 1x, 1.5x, 2x and
3x, and once more at 1x after the edit script. `edit/` holds what `to_string`
would save after that script, which changes every scalar, adds a key of every
kind the editor offers, renames the first key and removes the second in every
table, inline table and array-of-tables element.

Regenerate with `GOLDEN_UPDATE=1 cargo test --test golden`, and only on
purpose. The render golden depends on egui's glyph metrics and so on egui's
version; the edit golden on `toml_edit`'s. A commit that regenerates either says
which decision it records.

## Baseline, 2026-09-07

Recorded at commit `00dc14d` on Linux, before any of the extraction, so that
drift has a number to be measured against.

| Check | Result |
| --- | --- |
| `cargo test` | 87 passed: 59 in the library, 24 in the binary, 4 in `tests/handover.rs`; this file's tests add 3 |
| `cargo clippy --all-targets -- -D warnings` | silent |
| `cargo run --example corpus` | 88 cases, all agree; 73 showed a metadata tree, 41 a payload card |
| pure-Rust outcome check | no `.o` or `.a` under `release/build`; `ldd` names libc, libgcc_s and libm |
