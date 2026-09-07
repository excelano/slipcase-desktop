//! What the metadata editor draws and what it saves, held against a record.
//!
//! The editor is being extracted into `flyleaf`, and the claim each step of
//! that makes is "slipcase-desktop behaves as before". Nothing in the suite
//! could fail on that claim: the unit tests check behaviours one at a time, the
//! corpus runner checks only whether a tree appears, and the rest is
//! `CHECKLIST.md`, run by hand. These two tests turn the claim into a file
//! that diffs.
//!
//! Every fixture in `tests/golden/fixtures/` is rendered headlessly at four
//! scales and every shape egui emitted is written down with its kind, its
//! bounds and its text; and then a scripted sequence of every edit the editor
//! offers is run over it and what `to_string` would save is written down. Both
//! are compared against `tests/golden/render/` and `tests/golden/edit/`.
//!
//! Regenerate with `GOLDEN_UPDATE=1 cargo test --test golden`, and only on
//! purpose: a golden that changed is either a defect or a decision, and the
//! commit that regenerates it should say which. The render golden is tied to
//! egui's glyph metrics, so an egui bump regenerates it by decision.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui::{self, Shape};
use slipcase_desktop::tree::render;
use slipcase_desktop::{
    add_inline_key, add_key, remove_inline_key, remove_key, rename_inline_key, rename_key,
    set_value, NewKey,
};
use slpc::toml_edit::{Datetime, DocumentMut, InlineTable, Item, Table, Value};

/// The width every fixture is rendered in, in points.
const WIDTH: f32 = 900.0;

/// The scales the existing layout test runs at: 1x, a common laptop scale, a
/// Retina display, and the largest zoom anybody has reported using.
const SCALES: [f32; 4] = [1.0, 1.5, 2.0, 3.0];

/// The date every datetime is changed to. The epoch is what a new datetime key
/// starts as, so this is deliberately not that.
const EDITED_DATETIME: &str = "2026-09-07T12:00:00Z";

fn golden_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// Every fixture, by name, in a fixed order.
fn fixtures() -> Vec<(String, String)> {
    let dir = golden_dir().join("fixtures");
    let mut names: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("the fixtures directory")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no fixtures in {}", dir.display());
    names
        .into_iter()
        .map(|p| {
            let name = p.file_stem().expect("a stem").to_string_lossy().into_owned();
            let text = fs::read_to_string(&p).expect("a readable fixture");
            (name, text)
        })
        .collect()
}

/// Compare what was produced against the file, or rewrite the file when asked.
///
/// The failure names the first line that differs, because a golden of several
/// hundred lines is not read whole, and says how to regenerate.
fn hold(kind: &str, name: &str, ext: &str, produced: &str) {
    let path = golden_dir().join(kind).join(format!("{name}.{ext}"));
    if std::env::var_os("GOLDEN_UPDATE").is_some() {
        fs::write(&path, produced).expect("the golden is writable");
        return;
    }
    let Ok(recorded) = fs::read_to_string(&path) else {
        panic!(
            "no golden at {}; run `GOLDEN_UPDATE=1 cargo test --test golden` to record one",
            path.display()
        );
    };
    if recorded == produced {
        return;
    }
    let differs = recorded
        .lines()
        .zip(produced.lines())
        .position(|(a, b)| a != b)
        .map_or(recorded.lines().count().min(produced.lines().count()), |i| i);
    let was = recorded.lines().nth(differs).unwrap_or("<end>");
    let now = produced.lines().nth(differs).unwrap_or("<end>");
    panic!(
        "{kind} of `{name}` differs from {} at line {}:\n  recorded: {was}\n  produced: {now}\n\
         If this is a decision rather than a defect, `GOLDEN_UPDATE=1 cargo test --test golden` \
         and say why in the commit.",
        path.display(),
        differs + 1
    );
}

/// Every shape egui emitted for the tree, one line each, at one scale.
fn shapes_at(doc: &mut DocumentMut, ppp: f32) -> String {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(ppp);
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(WIDTH, 4000.0),
        )),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| {
        ui.set_max_width(WIDTH);
        render(ui, doc);
    });
    let shapes = std::mem::take(&mut output.shapes);
    output.drop_without_applying_deltas();

    let mut out = String::new();
    for clipped in &shapes {
        describe(&clipped.shape, ppp, &mut out);
    }
    out
}

/// One line per shape, flattening the groups egui nests.
fn describe(shape: &Shape, ppp: f32, out: &mut String) {
    if let Shape::Vec(inner) = shape {
        for s in inner {
            describe(s, ppp, out);
        }
        return;
    }
    let kind = match shape {
        Shape::Noop => return,
        Shape::Vec(_) => unreachable!("flattened above"),
        Shape::Circle(_) => "circle",
        Shape::Ellipse(_) => "ellipse",
        Shape::LineSegment { .. } => "line",
        Shape::Path(_) => "path",
        Shape::Rect(_) => "rect",
        Shape::Text(_) => "text",
        Shape::Mesh(_) => "mesh",
        Shape::QuadraticBezier(_) => "quadratic",
        Shape::CubicBezier(_) => "cubic",
        Shape::Callback(_) => "callback",
    };
    let r = shape.visual_bounding_rect();
    write!(
        out,
        "{ppp}x {kind} {:.1},{:.1} {:.1}x{:.1}",
        r.min.x,
        r.min.y,
        r.width(),
        r.height()
    )
    .expect("writing to a String");
    if let Shape::Text(t) = shape {
        write!(out, " {:?}", t.galley.text()).expect("writing to a String");
    }
    out.push('\n');
}

/// Change every scalar the editor lets a person change, in a way that shows
/// its type was read: strings gain a suffix, numbers move, booleans flip, and
/// every datetime becomes one date. Arrays are left alone, as the editor
/// leaves them; inline tables are entered.
fn edit_values(t: &mut Table) {
    for (_, item) in t.iter_mut() {
        match item {
            Item::Value(v) => edit_value(v),
            Item::Table(sub) => edit_values(sub),
            Item::ArrayOfTables(a) => {
                for sub in a.iter_mut() {
                    edit_values(sub);
                }
            }
            Item::None => {}
        }
    }
}

fn edit_value(v: &mut Value) {
    let new = match &*v {
        Value::String(s) => Some(Value::from(format!("{} (edited)", s.value()))),
        Value::Integer(i) => Some(Value::from(i.value() + 1)),
        Value::Float(f) => Some(Value::from(f.value() + 0.5)),
        Value::Boolean(b) => Some(Value::from(!b.value())),
        Value::Datetime(_) => Some(Value::from(
            EDITED_DATETIME
                .parse::<Datetime>()
                .expect("the edited datetime is one"),
        )),
        Value::Array(_) | Value::InlineTable(_) => None,
    };
    if let Some(new) = new {
        set_value(v, new);
    } else if let Value::InlineTable(t) = v {
        for (_, inner) in t.iter_mut() {
            edit_value(inner);
        }
    }
}

/// In every table, deepest first: add one key of each kind the editor offers,
/// rename the first key that was there, and remove the second.
fn edit_structure(t: &mut Table) {
    let names: Vec<String> = t.iter().map(|(k, _)| k.to_owned()).collect();
    for name in &names {
        match t.get_mut(name) {
            Some(Item::Table(sub)) => edit_structure(sub),
            Some(Item::ArrayOfTables(a)) => {
                for sub in a.iter_mut() {
                    edit_structure(sub);
                }
            }
            Some(Item::Value(Value::InlineTable(inline))) => edit_inline_structure(inline),
            _ => {}
        }
    }
    for kind in NewKey::ALL {
        assert!(add_key(t, &added_name(kind), kind), "add {kind:?}");
    }
    if let Some(first) = names.first() {
        assert!(rename_key(t, first, &format!("{first}_renamed")), "rename {first}");
    }
    if let Some(second) = names.get(1) {
        assert!(remove_key(t, second), "remove {second}");
    }
}

fn edit_inline_structure(t: &mut InlineTable) {
    let names: Vec<String> = t.iter().map(|(k, _)| k.to_owned()).collect();
    for name in &names {
        if let Some(Value::InlineTable(inner)) = t.get_mut(name) {
            edit_inline_structure(inner);
        }
    }
    for kind in NewKey::SCALARS {
        assert!(add_inline_key(t, &added_name(kind), kind), "add {kind:?}");
    }
    if let Some(first) = names.first() {
        assert!(
            rename_inline_key(t, first, &format!("{first}_renamed")),
            "rename {first}"
        );
    }
    if let Some(second) = names.get(1) {
        assert!(remove_inline_key(t, second), "remove {second}");
    }
}

fn added_name(kind: NewKey) -> String {
    format!("added_{}", kind.label().replace(' ', "_"))
}

/// A fixture that does not survive a parse and a `to_string` unchanged is a
/// bad fixture, not a finding: the edit golden would then record the parser's
/// changes as the editor's.
#[test]
fn every_fixture_round_trips_unedited() {
    for (name, text) in fixtures() {
        let doc: DocumentMut = text.parse().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(doc.to_string(), text, "{name} does not round-trip");
    }
}

/// The tree draws what it drew: every shape, where it was, with the text it
/// carried, at four scales.
///
/// Catches any change to what the editor puts on screen, which is the whole of
/// what a person sees of it: a row that moved, a comment that went missing, a
/// control that fell off the edge, a widget that changed type.
#[test]
fn the_tree_draws_what_it_drew() {
    for (name, text) in fixtures() {
        let mut doc: DocumentMut = text.parse().expect("a valid fixture");
        let mut out = String::new();
        for ppp in SCALES {
            out.push_str(&shapes_at(&mut doc, ppp));
        }
        hold("render", &name, "txt", &out);
    }
}

/// Every edit the editor offers, run over every fixture, saves what it saved.
///
/// Catches any change to what an edit does to the document: a comment lost on
/// a value change, a renamed key that moved, an added key that landed in the
/// wrong table, a type that serializes differently. The edited document is
/// also rendered once, so a row the edits produce is held too.
#[test]
fn every_edit_saves_what_it_saved() {
    for (name, text) in fixtures() {
        let mut doc: DocumentMut = text.parse().expect("a valid fixture");
        edit_values(doc.as_table_mut());
        edit_structure(doc.as_table_mut());
        let saved = doc.to_string();
        saved
            .parse::<DocumentMut>()
            .unwrap_or_else(|e| panic!("{name}: the edited document is not TOML: {e}"));
        hold("edit", &name, "toml", &saved);

        let mut edited: DocumentMut = saved.parse().expect("parsed a line above");
        hold("render", &format!("{name}.edited"), "txt", &shapes_at(&mut edited, 1.0));
    }
}
