//! The metadata document, as a tree.
//!
//! One renderer per TOML type rather than per schema. Past the two keys SPEC
//! §2.2 requires, the specification defines no vocabulary, so there is nothing
//! to special-case and no allowlist to write. DESIGN.md §4.
//!
//! Scalars are editable and structure is not: a key can have its value changed,
//! and adding, deleting, or renaming one is deferred. The two keys SPEC §2.2
//! requires are shown and not edited. `payload.file` names the member the
//! container is built around, and changing it without renaming that member
//! leaves a container naming a payload that is not there; `slipcase_version` is
//! the claim the whole verdict rests on, and `Repack` refuses to write a
//! document disagreeing with the version it implements.

use eframe::egui::{self, Ui};
use slpc::toml_edit::{Array, Datetime, DocumentMut, InlineTable, Item, RawString, Table, Value};

use crate::set_value;

/// The width a key is given before its value starts, so values line up down a
/// section without a grid, which cannot hold rows of mixed height in document
/// order.
const KEY_WIDTH: f32 = 190.0;

/// The width an integer is right-aligned within. DESIGN.md §4 asks for the
/// alignment; a number needs a column of its own to be aligned inside.
const NUMBER_WIDTH: f32 = 120.0;

/// The width a value is edited in.
const VALUE_WIDTH: f32 = 320.0;

/// Render a metadata document, and let its scalars be edited.
pub fn render(ui: &mut Ui, doc: &mut DocumentMut) {
    // Comments after the last item attach to no key, so no row can carry them.
    // Dropping them would tell a reader their file holds less than it does.
    let trailing = comment_lines(Some(doc.trailing()));

    let mut path: Vec<String> = Vec::new();
    table(ui, doc.as_table_mut(), &mut path);

    if !trailing.is_empty() {
        ui.add_space(8.0);
        ui.separator();
        for line in trailing {
            ui.label(comment_text(&line));
        }
    }
}

/// Every entry of a table, in the order the document wrote them.
fn table(ui: &mut Ui, t: &mut Table, path: &mut Vec<String>) {
    for (key, item) in t.iter_mut() {
        let name = key.get().to_owned();
        let above = comment_lines(key.leaf_decor().prefix());
        path.push(name.clone());
        entry(ui, &name, item, above, path);
        path.pop();
    }
}

/// One entry: a section for anything holding entries, a row for anything else.
fn entry(ui: &mut Ui, name: &str, item: &mut Item, above: Vec<String>, path: &mut Vec<String>) {
    match item {
        // A key that was removed. Nothing was written for it and nothing shows.
        Item::None => {}
        Item::Value(v) => value(ui, name, v, above, path),
        Item::Table(t) => {
            // A `[header]` carries its own comments rather than the key's.
            let comments = joined(&comment_lines(t.decor().prefix()));
            section(ui, name, comments.as_deref(), |ui| table(ui, t, path));
        }
        Item::ArrayOfTables(a) => {
            // Neither a section nor a leaf under §4's first sentence: a section
            // whose children are numbered sections, one per table.
            section(ui, name, joined(&above).as_deref(), |ui| {
                for (n, t) in a.iter_mut().enumerate() {
                    let comments = joined(&comment_lines(t.decor().prefix()));
                    let label = format!("[{n}]");
                    path.push(label.clone());
                    section(ui, &label, comments.as_deref(), |ui| table(ui, t, path));
                    path.pop();
                }
            });
        }
    }
}

/// One value: an inline table is a section, everything else is a row.
fn value(ui: &mut Ui, name: &str, v: &mut Value, above: Vec<String>, path: &mut Vec<String>) {
    // A comment after the value on its own line sits in the value's suffix.
    let mut comments = above;
    comments.extend(comment_lines(v.decor().suffix()));
    let comment = joined(&comments);

    match v {
        Value::InlineTable(t) => {
            section(ui, name, comment.as_deref(), |ui| inline_table(ui, t, path));
        }
        _ => row(ui, name, comment.as_deref(), |ui| scalar(ui, v, path)),
    }
}

/// An inline table's entries. TOML 1.1 lets one span lines, so its keys can
/// carry comments of their own.
fn inline_table(ui: &mut Ui, t: &mut InlineTable, path: &mut Vec<String>) {
    for (key, v) in t.iter_mut() {
        let name = key.get().to_owned();
        let above = comment_lines(key.leaf_decor().prefix());
        path.push(name.clone());
        value(ui, &name, v, above, path);
        path.pop();
    }
}

/// Whether this key is one SPEC §2.2 requires, and so one this shows without
/// letting it be edited.
fn is_required(path: &[String]) -> bool {
    let joined = path.join(".");
    joined == slpc::VERSION_KEY || joined == slpc::PAYLOAD_FILE_KEY
}

/// The widget a value gets, chosen by its TOML type and nothing else.
///
/// Reads the current value to seed the widget and returns a replacement rather
/// than writing through the borrow it is holding. [`set_value`] puts back the
/// decor the old value carried.
fn scalar(ui: &mut Ui, v: &mut Value, path: &[String]) {
    let editable = !is_required(path);
    let id = ui.make_persistent_id(path.join("."));

    let replacement = match &*v {
        Value::String(s) => {
            let mut text = s.value().clone();
            let field = egui::TextEdit::singleline(&mut text).desired_width(VALUE_WIDTH);
            ui.add_enabled(editable, field)
                .changed()
                .then(|| Value::from(text))
        }
        Value::Integer(i) => {
            // Right-aligned, which needs a column to be aligned within.
            let mut n = *i.value();
            let mut changed = false;
            ui.scope(|ui| {
                ui.set_min_width(NUMBER_WIDTH);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    changed = ui.add_enabled(editable, egui::DragValue::new(&mut n)).changed();
                });
            });
            changed.then(|| Value::from(n))
        }
        Value::Float(f) => {
            let mut x = *f.value();
            ui.add_enabled(editable, egui::DragValue::new(&mut x).speed(0.1))
                .changed()
                .then(|| Value::from(x))
        }
        Value::Boolean(b) => {
            let mut shown = *b.value();
            ui.add_enabled(editable, egui::Checkbox::new(&mut shown, ""))
                .changed()
                .then(|| Value::from(shown))
        }
        // All four shapes format themselves, and which one it is is written in
        // the value rather than in a wrapper this would have to unpack. A
        // half-typed one is not a datetime, so this is the one field whose
        // in-progress text has to outlive the frame it was typed in.
        Value::Datetime(d) => {
            let current = d.value().to_string();
            let typed = buffered_text(ui, id, &current, editable);
            let shown = typed.as_deref().unwrap_or(&current);
            match shown.parse::<Datetime>() {
                Ok(parsed) if typed.is_some() => Some(Value::from(parsed)),
                Ok(_) => None,
                Err(_) => {
                    ui.label(egui::RichText::new("not a date").italics().weak());
                    None
                }
            }
        }
        // Arrays and inline tables are structure. Shown, and edited when
        // structural editing lands.
        Value::Array(a) => {
            ui.label(array_text(a));
            None
        }
        Value::InlineTable(_) => None,
    };

    if let Some(new) = replacement {
        set_value(v, new);
    }
}

/// A text field whose in-progress contents survive between frames.
///
/// Re-seeded from the document whenever the field does not have focus, so a
/// buffer left over from another container cannot show a value that is not
/// there. While it does have focus, what was typed is what stays.
fn buffered_text(ui: &mut Ui, id: egui::Id, current: &str, editable: bool) -> Option<String> {
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(|| current.to_owned())
    } else {
        current.to_owned()
    };

    let field = egui::TextEdit::singleline(&mut text)
        .id(id)
        .desired_width(VALUE_WIDTH);
    if ui.add_enabled(editable, field).changed() {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
        Some(text)
    } else {
        None
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
        let mut doc = parsed();
        eframe::egui::__run_test_ui(|ui| render(ui, &mut doc));
    }

    /// The two keys SPEC §2.2 requires are shown and not edited, and nothing
    /// else is. A key of the same name nested under another table is a
    /// different key and is editable.
    #[test]
    fn only_the_required_keys_are_read_only() {
        let path = |parts: &[&str]| -> Vec<String> {
            parts.iter().map(|p| (*p).to_owned()).collect()
        };

        assert!(super::is_required(&path(&["slipcase_version"])));
        assert!(super::is_required(&path(&["payload", "file"])));

        assert!(!super::is_required(&path(&["title"])));
        assert!(!super::is_required(&path(&["payload", "size"])));
        assert!(!super::is_required(&path(&["elsewhere", "slipcase_version"])));
        assert!(!super::is_required(&path(&["payload"])));
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
