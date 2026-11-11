# The goldens

What the flyleaf editor draws and what it saves, held against a record. The
editor is `excelano/flyleaf`'s, so a change in either repository can alter what
this application draws or saves, and these goldens make that change fail rather
than pass unnoticed. `tests/golden.rs` is the test; this file is the record.

`fixtures/` holds the documents: flyleaf members copied out of the conformance
corpus's accept cases (comments and blank lines, a multi-line inline table,
nested tables, top-level keys), an every-type document, and a long-comment
document for the truncation case. `render/` holds every shape egui emitted for
each, at 1x, 1.5x, 2x and 3x, and once more at 1x after the edit script.
`edit/` holds what `to_string` would save after that script, which changes every
scalar, adds a key of every kind the editor offers, renames the first key and
removes the second in every table, inline table and array-of-tables element.

Regenerate with `GOLDEN_UPDATE=1 cargo test --test golden`, and only on
purpose. The render golden depends on egui's glyph metrics and so on egui's
version; the edit golden on `toml_edit`'s. A commit that regenerates either says
which decision it records.
