//! The metadata document, as a tree.
//!
//! One renderer per TOML type rather than per schema. Past the two keys SPEC
//! §2.2 requires, the specification defines no vocabulary, so there is nothing
//! to special-case and no allowlist to write. DESIGN.md §4.

use eframe::egui::{self, Ui};
use slpc::toml_edit::{Array, DocumentMut, InlineTable, Item, RawString, Table, Value};

/// The width a key is given before its value starts, so values line up down a
/// section without a grid, which cannot hold rows of mixed height in document
/// order.
const KEY_WIDTH: f32 = 190.0;

/// The width an integer is right-aligned within. DESIGN.md §4 asks for the
/// alignment; a number needs a column of its own to be aligned inside.
const NUMBER_WIDTH: f32 = 120.0;

/// Render a metadata document.
pub fn render(ui: &mut Ui, doc: &DocumentMut) {
    table(ui, doc.as_table());

    // Comments after the last item attach to no key, so no row can carry them.
    // Dropping them would tell a reader their file holds less than it does.
    let trailing = comment_lines(Some(doc.trailing()));
    if !trailing.is_empty() {
        ui.add_space(8.0);
        ui.separator();
        for line in trailing {
            ui.label(comment_text(&line));
        }
    }
}

/// Every entry of a table, in the order the document wrote them.
fn table(ui: &mut Ui, t: &Table) {
    for (name, item) in t {
        let above = t.key(name).map(|k| comment_lines(k.leaf_decor().prefix()));
        entry(ui, name, item, above.unwrap_or_default());
    }
}

/// One entry: a section for anything holding entries, a row for anything else.
fn entry(ui: &mut Ui, name: &str, item: &Item, above: Vec<String>) {
    match item {
        // A key that was removed. Nothing was written for it and nothing shows.
        Item::None => {}
        Item::Value(v) => value(ui, name, v, above),
        Item::Table(t) => {
            // A `[header]` carries its own comments rather than the key's.
            let comments = joined(&comment_lines(t.decor().prefix()));
            section(ui, name, comments.as_deref(), |ui| table(ui, t));
        }
        Item::ArrayOfTables(a) => {
            // Neither a section nor a leaf under §4's first sentence: a section
            // whose children are numbered sections, one per table.
            section(ui, name, joined(&above).as_deref(), |ui| {
                for (n, t) in a.iter().enumerate() {
                    let comments = joined(&comment_lines(t.decor().prefix()));
                    section(ui, &format!("[{n}]"), comments.as_deref(), |ui| table(ui, t));
                }
            });
        }
    }
}

/// One value: an inline table is a section, everything else is a row.
fn value(ui: &mut Ui, name: &str, v: &Value, above: Vec<String>) {
    // A comment after the value on its own line sits in the value's suffix.
    let mut comments = above;
    comments.extend(comment_lines(v.decor().suffix()));
    let comment = joined(&comments);

    match v {
        Value::InlineTable(t) => section(ui, name, comment.as_deref(), |ui| inline_table(ui, t)),
        _ => row(ui, name, comment.as_deref(), |ui| scalar(ui, v)),
    }
}

/// An inline table's entries. TOML 1.1 lets one span lines, so its keys can
/// carry comments of their own.
fn inline_table(ui: &mut Ui, t: &InlineTable) {
    for (name, v) in t {
        let above = t.key(name).map(|k| comment_lines(k.leaf_decor().prefix()));
        value(ui, name, v, above.unwrap_or_default());
    }
}

/// The widget a value gets, chosen by its TOML type and nothing else.
fn scalar(ui: &mut Ui, v: &Value) {
    match v {
        Value::String(s) => {
            ui.label(s.value().as_str());
        }
        Value::Integer(i) => {
            // Right-aligned, which needs a column to be aligned within.
            ui.scope(|ui| {
                ui.set_min_width(NUMBER_WIDTH);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(i.value().to_string());
                });
            });
        }
        Value::Float(f) => {
            ui.label(f.value().to_string());
        }
        Value::Boolean(b) => {
            // Disabled rather than absent: the state is the value, and nothing
            // in this slice edits anything.
            let mut shown = *b.value();
            ui.add_enabled(false, egui::Checkbox::new(&mut shown, ""));
        }
        // All four shapes format themselves, and which one it is is written in
        // the value rather than in a wrapper this would have to unpack.
        Value::Datetime(d) => {
            ui.label(d.value().to_string());
        }
        Value::Array(a) => {
            ui.label(array_text(a));
        }
        // An inline table reaches `section` instead and never arrives here.
        Value::InlineTable(_) => {}
    }
}

/// An array, as the leaf DESIGN.md §4 calls for.
fn array_text(a: &Array) -> String {
    a.to_string().trim().to_owned()
}

/// A collapsing section, open until a person closes it.
fn section(ui: &mut Ui, name: &str, comment: Option<&str>, body: impl FnOnce(&mut Ui)) {
    // A header is one piece of text, so a comment beside this key joins it
    // rather than sitting in a column of its own.
    let title = match comment {
        Some(c) => format!("{name}    # {c}"),
        None => name.to_owned(),
    };
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, body);
}

/// One row: the key, the value, and whatever the document said beside it.
fn row(ui: &mut Ui, name: &str, comment: Option<&str>, value: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.scope(|ui| {
            ui.set_min_width(KEY_WIDTH);
            ui.label(egui::RichText::new(name).strong());
        });
        value(ui);
        if let Some(c) = comment {
            ui.label(comment_text(c));
        }
    });
}

/// How a comment reads: quieter than the data it annotates, and still a comment.
fn comment_text(line: &str) -> egui::RichText {
    egui::RichText::new(format!("# {line}")).italics().weak()
}

/// The comment lines in a piece of decor, with their `#` and their surrounding
/// whitespace removed.
///
/// Decor holds blank lines and indentation as well as comments, so this keeps
/// only the lines that are comments. A parsed document has despanned decor, so
/// the text is there to read; one built in memory may not, and then there is
/// nothing to show.
fn comment_lines(raw: Option<&RawString>) -> Vec<String> {
    let Some(text) = raw.and_then(RawString::as_str) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|l| l.trim().strip_prefix('#').map(|c| c.trim().to_owned()))
        .collect()
}

/// Several comment lines as the one line a row has room for.
fn joined(lines: &[String]) -> Option<String> {
    if lines.is_empty() {
        None
    } else {
        Some(lines.join(" "))
    }
}

#[cfg(test)]
mod tests {
    use super::{array_text, comment_lines, render};
    use slpc::toml_edit::DocumentMut;

    /// Every type DESIGN.md §4 names, and every place a comment can sit.
    ///
    /// The conformance corpus tests the format rather than TOML's type space,
    /// so it carries no array of tables, array, boolean, float, or datetime.
    /// Those renderers have nothing else to exercise them.
    const EVERY_TYPE: &str = r#"# a document comment
slipcase_version = "1.0"   # the version

[payload]
file = "report.pdf"

[types]
text = "a string"
count = 44
ratio = 1.5
flag = true
offset_date_time = 1979-05-27T07:32:00Z
local_date_time = 1979-05-27T07:32:00
local_date = 1979-05-27
local_time = 07:32:00
list = [1, 2, 3]
inline = { a = 1, b = "two" }
# above a dotted key
dotted.key = "reached by a dot"

[[runs]]
id = 1

[[runs]]
id = 2

# a comment attached to nothing
"#;

    fn parsed() -> DocumentMut {
        EVERY_TYPE.parse().expect("the fixture is valid TOML")
    }

    /// Renders headlessly, which is enough to reach every arm and to fail on a
    /// panic in any of them.
    #[test]
    fn every_toml_type_renders() {
        let doc = parsed();
        eframe::egui::__run_test_ui(|ui| render(ui, &doc));
    }

    /// §4: document order is preserved and never sorted. Authoring order
    /// carries intent, and this fails if anything starts sorting.
    #[test]
    fn document_order_is_preserved() {
        let doc = parsed();
        let order: Vec<&str> = doc.as_table().iter().map(|(k, _)| k).collect();
        assert_eq!(order, ["slipcase_version", "payload", "types", "runs"]);
    }

    /// A comment above a key shares its decor with blank lines and indentation,
    /// and only the comment is wanted.
    #[test]
    fn a_comment_above_a_key_is_found_and_blank_lines_are_not() {
        let doc = parsed();
        let key = doc.as_table().key("slipcase_version").expect("the key");
        assert_eq!(comment_lines(key.leaf_decor().prefix()), ["a document comment"]);
    }

    /// A comment after the value on the same line sits in the value's suffix
    /// rather than in the next key's prefix.
    #[test]
    fn a_comment_after_a_value_is_found() {
        let doc = parsed();
        let v = doc.as_table()["slipcase_version"].as_value().expect("a value");
        assert_eq!(comment_lines(v.decor().suffix()), ["the version"]);
    }

    /// A comment after the last item attaches to no key, so no row can carry
    /// it. Decision C: it is shown unattached rather than dropped.
    #[test]
    fn a_comment_after_the_last_item_attaches_to_nothing() {
        let doc = parsed();
        assert_eq!(
            comment_lines(Some(doc.trailing())),
            ["a comment attached to nothing"]
        );
    }

    /// A comment above a dotted key attaches to the leaf segment rather than
    /// to the table the dot implies, so it shows beside `key` and not beside
    /// `dotted`. SPEC §2.2 requires `payload.file`, so this is not an edge.
    #[test]
    fn a_comment_above_a_dotted_key_attaches_to_its_leaf() {
        let doc = parsed();
        let types = doc.as_table()["types"].as_table().expect("a table");

        let outer = types.key("dotted").expect("the segment the dot implies");
        assert!(comment_lines(outer.leaf_decor().prefix()).is_empty());

        let implied = types["dotted"].as_table().expect("the implied table");
        let leaf = implied.key("key").expect("the leaf segment");
        assert_eq!(comment_lines(leaf.leaf_decor().prefix()), ["above a dotted key"]);
    }

    /// An array is a leaf, and its text is the array rather than its decor.
    #[test]
    fn an_array_renders_as_its_own_text() {
        let doc = parsed();
        let a = doc.as_table()["types"]["list"]
            .as_array()
            .expect("an array");
        assert_eq!(array_text(a), "[1, 2, 3]");
    }
}
