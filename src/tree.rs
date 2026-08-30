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

use std::borrow::Cow;

use eframe::egui::{self, Ui};
use slpc::toml_edit::{Array, Datetime, DocumentMut, InlineTable, Item, RawString, Table, Value};

use crate::{
    add_inline_key, add_key, remove_inline_key, remove_key, rename_inline_key, rename_key,
    set_value, NewKey,
};

/// What the button that removes a key is marked with.
///
/// A wastebasket, because the control is destructive and has to look it. The
/// first version used U+2715, which is in none of egui's default fonts and drew
/// a replacement box: an empty square beside rows that carry real checkboxes
/// for their booleans, which is how somebody comes to press one thinking it
/// toggles something. A test holds the glyph against the fonts.
const REMOVE: &str = "\u{1F5D1}";

/// The width a key is given before its value starts, so values line up down a
/// section without a grid, which cannot hold rows of mixed height in document
/// order.
const KEY_WIDTH: f32 = 190.0;

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
    // Taken before the loop borrows the table, so a row can say whether the
    // name being typed into it is one of its own siblings.
    let siblings: Vec<String> = t.iter().map(|(k, _)| k.to_owned()).collect();
    let mut change = None;

    for (key, item) in t.iter_mut() {
        let name = key.get().to_owned();
        let above = comment_lines(key.leaf_decor().prefix());
        path.push(name.clone());
        entry(ui, &name, item, above, path, &siblings, &mut change);
        path.pop();
    }

    add_row(ui, path, &siblings, &NewKey::ALL, &mut change);

    if let Some(change) = change {
        apply(t, change);
    }
}

/// One entry: a section for anything holding entries, a row for anything else.
fn entry(
    ui: &mut Ui,
    name: &str,
    item: &mut Item,
    above: Vec<String>,
    path: &mut Vec<String>,
    siblings: &[String],
    change: &mut Option<Change>,
) {
    match item {
        // A key that was removed. Nothing was written for it and nothing shows.
        Item::None => {}
        Item::Value(v) => value(ui, name, v, above, path, siblings, change),
        Item::Table(t) => {
            // A `[header]` carries its own comments rather than the key's.
            let comments = joined(&comment_lines(t.decor().prefix()));
            section(ui, name, comments.as_deref(), |ui| {
                // Inside the section rather than beside its header: a
                // `CollapsingHeader` draws its body as well as its title, and a
                // body laid out sideways is what putting one in a row gives.
                controls(ui, name, path, siblings, change);
                table(ui, t, path);
            });
        }
        Item::ArrayOfTables(a) => {
            // Neither a section nor a leaf under §4's first sentence: a section
            // whose children are numbered sections, one per table.
            section(ui, name, joined(&above).as_deref(), |ui| {
                controls(ui, name, path, siblings, change);
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
fn value(
    ui: &mut Ui,
    name: &str,
    v: &mut Value,
    above: Vec<String>,
    path: &mut Vec<String>,
    siblings: &[String],
    change: &mut Option<Change>,
) {
    // A comment after the value on its own line sits in the value's suffix.
    let mut comments = above;
    comments.extend(comment_lines(v.decor().suffix()));
    let comment = joined(&comments);

    match v {
        Value::InlineTable(t) => {
            section(ui, name, comment.as_deref(), |ui| {
                controls(ui, name, path, siblings, change);
                inline_table(ui, t, path);
            });
        }
        _ => row(ui, name, comment.as_deref(), path, siblings, change, |ui| {
            scalar(ui, v, path);
        }),
    }
}

/// An inline table's entries. TOML 1.1 lets one span lines, so its keys can
/// carry comments of their own.
///
/// Its keys can be added to, renamed, and removed like any other table's. They
/// could not at first, and the buttons were drawn anyway and did nothing, which
/// is how somebody comes to press one twice and wonder what is broken. Being
/// written on one line is a fact about how it is laid out and not about what
/// can be done to it.
fn inline_table(ui: &mut Ui, t: &mut InlineTable, path: &mut Vec<String>) {
    let siblings: Vec<String> = t.iter().map(|(k, _)| k.to_owned()).collect();
    let mut change = None;

    for (key, v) in t.iter_mut() {
        let name = key.get().to_owned();
        let above = comment_lines(key.leaf_decor().prefix());
        path.push(name.clone());
        value(ui, &name, v, above, path, &siblings, &mut change);
        path.pop();
    }

    // Values only: an inline table holds no tables.
    add_row(ui, path, &siblings, &NewKey::SCALARS, &mut change);

    if let Some(change) = change {
        apply_inline(t, change);
    }
}

/// Whether this key is one SPEC §2.2 requires, or one holding it.
///
/// A protected key is shown and not edited, renamed, or deleted. `payload` is
/// protected as well as `payload.file`, because deleting or renaming the table
/// takes the required key inside it with it, which the value being read-only
/// would not have stopped.
fn is_protected(path: &[String]) -> bool {
    let joined = path.join(".");
    [slpc::VERSION_KEY, slpc::PAYLOAD_FILE_KEY]
        .iter()
        .any(|required| *required == joined || required.starts_with(&format!("{joined}.")))
}

/// What a string value reads as in the tree.
///
/// A protected string is a display rather than a field — [`is_protected`]
/// disables the widget — and one of the two, `payload.file`, is a member name.
/// SPEC §3 requires a name be shown escaped, which the card does through
/// `slpc::display_name` and this did not: a payload called
/// `report<U+202E>fdp.exe` read `report\u{202E}fdp.exe` on the card and
/// `reportfdp.exe` two rows below it, because egui gives a bidirectional
/// formatting character zero advance width. The tree was showing the spoof the
/// escaping exists to prevent, under a card that was not.
///
/// Found by hand on Windows on 2026-08-29 against
/// `accept/payload-name-bidi-override`, while running the card's item 3 — which
/// asks about the card, so macOS and Linux had both ticked it without looking
/// two rows down. The code is shared and all three platforms had this.
///
/// **An editable string is deliberately left alone**, which is why this takes
/// the flag rather than escaping everything. Escaping a value somebody can type
/// into is lossy: the eight characters `\u{202E}` would be written back as
/// themselves the first time the field was touched, so a document merely
/// mentioning such a character would gain them. `src/main.rs` records the same
/// reasoning where the Extract-to dialog prefills a filename.
fn displayed(value: &str, editable: bool) -> Cow<'_, str> {
    if editable {
        Cow::Borrowed(value)
    } else {
        slpc::display_name(value)
    }
}

/// A change to a table's own entries, gathered while its rows are drawn and
/// made once the loop over them has let go of it.
enum Change {
    Delete(String),
    Rename(String, String),
    Add(String, NewKey),
}

/// Make the change the rows asked for.
fn apply(t: &mut Table, change: Change) {
    match change {
        Change::Delete(name) => {
            remove_key(t, &name);
        }
        Change::Rename(from, to) => {
            rename_key(t, &from, &to);
        }
        Change::Add(name, kind) => {
            add_key(t, &name, kind);
        }
    }
}

/// The same, for a table written on one line.
fn apply_inline(t: &mut InlineTable, change: Change) {
    match change {
        Change::Delete(name) => {
            remove_inline_key(t, &name);
        }
        Change::Rename(from, to) => {
            rename_inline_key(t, &from, &to);
        }
        Change::Add(name, kind) => {
            add_inline_key(t, &name, kind);
        }
    }
}

/// The widget a value gets, chosen by its TOML type and nothing else.
///
/// Reads the current value to seed the widget and returns a replacement rather
/// than writing through the borrow it is holding. [`set_value`] puts back the
/// decor the old value carried.
fn scalar(ui: &mut Ui, v: &mut Value, path: &[String]) {
    let editable = !is_protected(path);
    let id = ui.make_persistent_id(path.join("."));

    let replacement = match &*v {
        Value::String(s) => {
            let mut text = displayed(s.value(), editable).into_owned();
            let field = egui::TextEdit::singleline(&mut text).desired_width(VALUE_WIDTH);
            ui.add_enabled(editable, field)
                .changed()
                .then(|| Value::from(text))
        }
        Value::Integer(i) => {
            // Where every other value starts. DESIGN.md §4 asks for an integer
            // to be right-aligned; a column of one number, right-aligned while
            // the float beside it in the same widget is not, reads as a mistake
            // rather than as alignment.
            let mut n = *i.value();
            ui.add_enabled(editable, egui::DragValue::new(&mut n))
                .changed()
                .then(|| Value::from(n))
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
            let field = buffered_text(ui, id, &current, editable);
            // What the field is showing, rather than what was typed into it
            // this frame. A keystroke happens on one frame and the text stays
            // on screen for every frame after, so reading the keystroke made
            // the refusal below appear for a sixtieth of a second and vanish
            // while the text it was about was still sitting there.
            //
            // Trimmed, because a space either side is a typing artefact rather
            // than something somebody meant.
            match parse_datetime(field.text.trim()) {
                Some(parsed) if field.changed => Some(Value::from(parsed)),
                Some(_) => None,
                // Said in the colour this theme uses for things that are wrong.
                // A weak grey note beside a field is one nobody sees, and what
                // it is not saying is that the value is being refused: TOML
                // wants two digits in an hour, so `9:00:00` is not a time and
                // `09:00:00` is, which is not a difference anybody guesses at.
                None => {
                    ui.label(
                        egui::RichText::new("not a date or time; not saved")
                            .color(ui.visuals().error_fg_color),
                    );
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

/// Read a date or time, putting back the leading zero TOML wants on an hour.
///
/// `19:00` is a time and `9:00` is not, which is the format's grammar rather
/// than anything a person should carry in their head while typing into a field.
/// A single digit before the first colon has one reading and no other, so it
/// gets its zero and is read again. Nothing else is repaired: `18.00.00` and
/// `1800` stay refused, because guessing at those would be guessing.
fn parse_datetime(text: &str) -> Option<Datetime> {
    if let Ok(parsed) = text.parse::<Datetime>() {
        return Some(parsed);
    }

    // The hour is what runs up to the first colon, after a date and its `T`
    // where there is one.
    let (before, rest) = text.split_once(':')?;
    let hour_at = before.rfind(|c: char| !c.is_ascii_digit()).map_or(0, |i| i + 1);
    let (head, hour) = before.split_at(hour_at);
    if hour.len() != 1 {
        return None;
    }

    format!("{head}0{hour}:{rest}").parse::<Datetime>().ok()
}

/// A text field whose in-progress contents survive between frames.
///
/// Re-seeded from the document whenever the field does not have focus, so a
/// buffer left over from another container cannot show a value that is not
/// there. While it does have focus, what was typed is what stays.
/// What a text field is showing, and whether this frame changed it.
struct Field {
    /// What is on screen now, typed or seeded from the document.
    text: String,
    /// Whether a keystroke landed this frame.
    changed: bool,
}

fn buffered_text(ui: &mut Ui, id: egui::Id, current: &str, editable: bool) -> Field {
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data_mut(|d| d.get_temp::<String>(id))
            .unwrap_or_else(|| current.to_owned())
    } else {
        current.to_owned()
    };

    let field = egui::TextEdit::singleline(&mut text)
        .id(id)
        .desired_width(VALUE_WIDTH);
    let changed = ui.add_enabled(editable, field).changed();
    if changed {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
    }

    Field { text, changed }
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

/// One row: the key, the value, whatever the document said beside it, and the
/// way to remove it.
fn row(
    ui: &mut Ui,
    name: &str,
    comment: Option<&str>,
    path: &[String],
    siblings: &[String],
    change: &mut Option<Change>,
    value: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.scope(|ui| {
            ui.set_min_width(KEY_WIDTH);
            key_name(ui, name, path, siblings, change);
        });
        value(ui);
        // The comment is capped so it cannot eat the room the remove control
        // needs, and truncates instead.
        //
        // Uncapped, a comment long enough to fill the row pushed that control
        // clean out of the window, and the only way to reach it was to widen
        // the window. Found on Apple silicon by zooming, which shrinks the
        // space a row has in points; then reproduced at 1x on an ordinary
        // display with a one-line comment, which is what showed it was nothing
        // to do with zoom or with the machine.
        //
        // Anchoring the control to the right edge instead — a right-to-left
        // layout — fixes the clipping and was tried first. It spreads every row
        // to the full window width, which `an_integer_stays_beside_its_key`
        // forbids for its own reason, and that test caught it.
        if let Some(c) = comment {
            let room = (ui.available_width() - remove_room(ui, path)).max(0.0);
            ui.scope(|ui| {
                ui.set_max_width(room);
                ui.add(egui::Label::new(comment_text(c)).truncate());
            });
        }
        delete_button(ui, name, path, change);
    });
}

/// The width to keep clear for the control that removes a key, so a comment
/// cannot take it.
///
/// Asked of the style rather than written as a number, because the button is a
/// `small_button` and its size follows the spacing the theme sets: a constant
/// here would be right at one text size and wrong at every other, which is the
/// case the defect showed up in. Protected keys have no such control and get
/// the whole row.
fn remove_room(ui: &Ui, path: &[String]) -> f32 {
    if is_protected(path) {
        return 0.0;
    }
    let spacing = ui.spacing();
    spacing.interact_size.y + spacing.button_padding.x * 2.0 + spacing.item_spacing.x
}

/// A section's own name and the way to remove it, drawn inside the section.
fn controls(
    ui: &mut Ui,
    name: &str,
    path: &[String],
    siblings: &[String],
    change: &mut Option<Change>,
) {
    if is_protected(path) {
        return;
    }
    ui.horizontal(|ui| {
        ui.scope(|ui| {
            ui.set_min_width(KEY_WIDTH);
            key_name(ui, name, path, siblings, change);
        });
        delete_button(ui, name, path, change);
    });
}

/// The key, as a name to read or a name to change.
fn key_name(
    ui: &mut Ui,
    name: &str,
    path: &[String],
    siblings: &[String],
    change: &mut Option<Change>,
) {
    if is_protected(path) {
        ui.label(egui::RichText::new(name).strong());
        return;
    }

    let id = ui.make_persistent_id((path.join("."), "key"));
    let committed = key_field(ui, id, name);

    // Said while it is being typed rather than after, because a name already
    // taken is refused and the field would otherwise just spring back.
    let typed: Option<String> = ui.data_mut(|d| d.get_temp(id));
    if ui.memory(|m| m.has_focus(id)) {
        if let Some(t) = &typed {
            if t.is_empty() || (t != name && siblings.iter().any(|s| s == t)) {
                ui.label(egui::RichText::new("name taken").italics().weak());
            }
        }
    }

    if let Some(to) = committed {
        if !to.is_empty() && to != name && !siblings.contains(&to) {
            *change = Some(Change::Rename(name.to_owned(), to));
        }
    }
}

/// A field holding a key's name, which takes effect when it is left.
///
/// Not as it is typed: renaming `first` to `primary` one keystroke at a time
/// would rename it to `f` on the way, and then to `fi`, each one a key of its
/// own.
fn key_field(ui: &mut Ui, id: egui::Id, current: &str) -> Option<String> {
    let focused = ui.memory(|m| m.has_focus(id));
    let mut text = if focused {
        ui.data_mut(|d| d.get_temp::<String>(id))
            .unwrap_or_else(|| current.to_owned())
    } else {
        current.to_owned()
    };

    let field = egui::TextEdit::singleline(&mut text)
        .id(id)
        .desired_width(KEY_WIDTH - 28.0);
    let response = ui.add(field);
    if response.changed() {
        ui.data_mut(|d| d.insert_temp(id, text.clone()));
    }
    (response.lost_focus() && text != current).then_some(text)
}

/// The way to remove a key, where removing it is allowed.
fn delete_button(ui: &mut Ui, name: &str, path: &[String], change: &mut Option<Change>) {
    if is_protected(path) {
        return;
    }
    // One press, and nothing reaches the container until Save. DESIGN.md §5
    // keeps the writing explicit, and this keeps the removing that way too.
    if ui
        .small_button(REMOVE)
        .on_hover_text("Remove this key")
        .clicked()
    {
        *change = Some(Change::Delete(name.to_owned()));
    }
}

/// The row that adds a key: a name, what it starts as, and Add.
fn add_row(
    ui: &mut Ui,
    path: &[String],
    siblings: &[String],
    kinds: &[NewKey],
    change: &mut Option<Change>,
) {
    let id = ui.make_persistent_id((path.join("."), "add"));
    let mut name: String = ui.data_mut(|d| d.get_temp(id)).unwrap_or_default();
    let mut kind: NewKey = ui
        .data_mut(|d| d.get_temp(id.with("kind")))
        .filter(|k| kinds.contains(k))
        .unwrap_or(NewKey::Text);

    ui.horizontal(|ui| {
        ui.scope(|ui| {
            ui.set_min_width(KEY_WIDTH);
            let field = egui::TextEdit::singleline(&mut name)
                .id(id.with("name"))
                .hint_text("add a key")
                .desired_width(KEY_WIDTH - 28.0);
            if ui.add(field).changed() {
                ui.data_mut(|d| d.insert_temp(id, name.clone()));
            }
        });

        egui::ComboBox::from_id_salt(id.with("kind picker"))
            .selected_text(kind.label())
            .show_ui(ui, |ui| {
                for one in kinds.iter().copied() {
                    if ui.selectable_value(&mut kind, one, one.label()).clicked() {
                        ui.data_mut(|d| d.insert_temp(id.with("kind"), one));
                    }
                }
            });

        let taken = siblings.contains(&name);
        if ui
            .add_enabled(!name.is_empty() && !taken, egui::Button::new("Add"))
            .clicked()
        {
            *change = Some(Change::Add(name.clone(), kind));
            ui.data_mut(|d| d.insert_temp(id, String::new()));
        }
        if taken {
            ui.label(egui::RichText::new("name taken").italics().weak());
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
    use super::{array_text, comment_lines, displayed, render};
    use slpc::toml_edit::DocumentMut;

    /// A payload name whose bidirectional override the tree swallowed.
    ///
    /// Without the escape the field read `reportfdp.exe` — egui gives U+202E
    /// zero advance width — which is a name one character short of the file on
    /// disk, shown two rows under a card that escapes it. That is the spoof
    /// SPEC §3's escaping exists to prevent, and it was in the one field this
    /// application will not let anybody edit.
    #[test]
    fn a_protected_name_is_shown_escaped() {
        assert_eq!(
            displayed("report\u{202E}fdp.exe", false),
            "report\\u{202E}fdp.exe"
        );
    }

    /// Escaping a field somebody can type into writes the escape back.
    ///
    /// The eight characters `\u{202E}` are what `display_name` produces, and a
    /// `TextEdit` bound to them returns them as themselves the moment the field
    /// is touched. A container whose metadata merely mentions such a character
    /// would gain them, which is a rewrite of somebody's document to make a
    /// display safer that was already safe: nothing here is a member name.
    #[test]
    fn an_editable_string_is_shown_as_it_is() {
        assert_eq!(
            displayed("report\u{202E}fdp.exe", true),
            "report\u{202E}fdp.exe"
        );
    }

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

    /// TOML wants two digits in an hour. A field a person types into should
    /// not, so the one repair with a single reading is made and no other.
    #[test]
    fn an_hour_without_its_leading_zero_is_still_a_time() {
        let read = |s: &str| super::parse_datetime(s).map(|d| d.to_string());

        assert_eq!(read("9:00").as_deref(), Some("09:00"));
        assert_eq!(read("9:00:00").as_deref(), Some("09:00:00"));
        assert_eq!(read("2026-08-21T9:00:00").as_deref(), Some("2026-08-21T09:00:00"));

        // Already a time, and untouched.
        assert_eq!(read("19:00").as_deref(), Some("19:00"));
        assert_eq!(read("2026-08-21").as_deref(), Some("2026-08-21"));

        // Guessing at these would be guessing.
        assert_eq!(read("18.00.00"), None);
        assert_eq!(read("1800"), None);
        assert_eq!(read("9"), None);
        assert_eq!(read("abc:def"), None);
        assert_eq!(read(""), None);
    }

    /// The mark on the button that removes a key is one the fonts can draw.
    ///
    /// A glyph they cannot draw comes out as a replacement box, and an empty
    /// square beside rows carrying real checkboxes reads as one. That is how
    /// the first version of this button came to be pressed by somebody who
    /// thought it toggled something.
    ///
    /// Uses a real context rather than `__run_test_ui`, which loads no fonts at
    /// all and would report every glyph missing, including the ones that work.
    #[test]
    fn the_remove_button_has_a_glyph_the_fonts_can_draw() {
        let ctx = eframe::egui::Context::default();
        ctx.run_ui(eframe::egui::RawInput::default(), |_| {})
            .drop_without_applying_deltas();

        let font = eframe::egui::FontId::proportional(14.0);
        assert!(
            ctx.fonts_mut(|f| f.has_glyphs(&font, super::REMOVE)),
            "the fonts cannot draw {:?}, so it would come out as a box",
            super::REMOVE
        );
    }

    /// A row stays inside the width it was given.
    ///
    /// It did not once: an integer aligned against everything left in the row
    /// put the number at the window's edge and the button that removes it past
    /// that, off the end, and the tree overflowed the width it was handed.
    /// Measured rather than looked at, because nothing here can look.
    #[test]
    fn an_integer_stays_beside_its_key() {
        let doc: DocumentMut = "pages = 44\n".parse().expect("valid TOML");
        let mut doc = doc;

        let mut content = 0.0;
        eframe::egui::__run_test_ui(|ui| {
            ui.set_max_width(900.0);
            render(ui, &mut doc);
            content = ui.min_rect().width();
        });

        // The row is a key column, a number column, and a button. Anything near
        // the full 900 means the number was pushed against the edge.
        assert!(
            content < 600.0,
            "the tree spread to {content:.0} of 900 available"
        );
    }

    /// A long comment does not push the control that removes a key past the
    /// width the row was given.
    ///
    /// Without the cap in `row`, a comment took every point left in the row and
    /// the remove button was laid out beyond the right edge — off the window,
    /// and reachable only by widening it. Found on Apple silicon by zooming,
    /// which shrinks the points a row has; then reproduced at 1x with a
    /// one-line comment, so it is nothing to do with zoom or with the machine.
    ///
    /// **Asked of the button's own position, and it has to be.** The obvious
    /// measurement — the width the tree reports — cannot see this: egui holds
    /// `min_rect` to the maximum it was given, so it reads the same whether the
    /// button landed inside the row or a hundred points past the end of it.
    /// Written that way first, this test passed with the fix deliberately
    /// removed. So it goes through the shapes egui actually emitted and finds
    /// the one drawing the wastebasket.
    ///
    /// **Run at four scales, and that is what answers the `@2x` question.**
    /// `pixels_per_point` is `native_scale × zoom_factor`, so a Retina display
    /// at 2 and this display zoomed to 2 are the same number and the same
    /// rasterisation and layout path. This was carried as needing a
    /// high-density display nobody had; what it needed was a scale, and a
    /// scale is something a test can set. Glyph
    /// metrics do round differently at each — 888.6 points at 1 against 889.3
    /// at 3 — which is exactly the drift that could push a borderline row over
    /// the edge, and is why this asserts at each rather than at one.
    #[test]
    fn a_long_comment_leaves_room_for_the_remove_button() {
        for ppp in [1.0_f32, 1.5, 2.0, 3.0] {
            at_scale(ppp);
        }
    }

    /// The body of the test above, at one `pixels_per_point`.
    fn at_scale(ppp: f32) {
        const WIDTH: f32 = 900.0;

        let long = "x".repeat(400);
        let mut doc: DocumentMut = format!("# {long}\ntitle = \"t\"\n")
            .parse()
            .expect("valid TOML");

        let ctx = eframe::egui::Context::default();
        ctx.set_pixels_per_point(ppp);
        let input = eframe::egui::RawInput {
            screen_rect: Some(eframe::egui::Rect::from_min_size(
                eframe::egui::Pos2::ZERO,
                eframe::egui::vec2(WIDTH, 2000.0),
            )),
            ..Default::default()
        };

        let mut output = ctx.run_ui(input, |ui| {
            ui.set_max_width(WIDTH);
            render(ui, &mut doc);
        });
        // The shapes are what this is about; the texture deltas belong to a
        // painter there is not one of here, and egui panics if they are dropped
        // unhandled rather than discarded on purpose.
        let shapes = std::mem::take(&mut output.shapes);
        output.drop_without_applying_deltas();

        let mut found = None;
        for clipped in &shapes {
            if let eframe::egui::Shape::Text(text) = &clipped.shape {
                if text.galley.text().contains(super::REMOVE) {
                    let right = text.pos.x + text.galley.size().x;
                    found = Some(found.map_or(right, |r: f32| r.max(right)));
                }
            }
        }

        // Two shapes of the same failure, and the defect produces the first:
        // laid out past the edge, egui culls the glyph rather than drawing it
        // somewhere unreachable, so "not found" is the finding and not a broken
        // precondition. Both are spelled out because a reader of the failure
        // should not have to know that to understand it.
        match found {
            None => panic!(
                "at {ppp}x the remove button was not drawn at all — laid out past \
                 the {WIDTH:.0} points the row was given, so egui culled it. That \
                 is the defect: a control the window offers no way to reach."
            ),
            Some(right) => assert!(
                right <= WIDTH,
                "at {ppp}x the remove button reaches {right:.0} of {WIDTH:.0} \
                 available, so the comment took the room it needed"
            ),
        }
    }

    /// The keys SPEC §2.2 requires are shown and not edited, and so is the
    /// table holding one: deleting `[payload]` would take `payload.file` with
    /// it, which making the value read-only would not have stopped.
    ///
    /// A key of the same name under another table is a different key, and a
    /// sibling of a required key is not required.
    #[test]
    fn the_required_keys_and_what_holds_them_are_protected() {
        let path =
            |parts: &[&str]| -> Vec<String> { parts.iter().map(|p| (*p).to_owned()).collect() };

        assert!(super::is_protected(&path(&["slipcase_version"])));
        assert!(super::is_protected(&path(&["payload", "file"])));
        assert!(super::is_protected(&path(&["payload"])));

        assert!(!super::is_protected(&path(&["title"])));
        assert!(!super::is_protected(&path(&["payload", "size"])));
        assert!(!super::is_protected(&path(&[
            "elsewhere",
            "slipcase_version"
        ])));
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
