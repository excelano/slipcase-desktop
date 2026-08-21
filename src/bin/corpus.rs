//! Run the slipcase conformance corpus against this application.
//!
//! Not a test. It needs a checkout of `excelano/slipcase` with its cases
//! generated, neither of which `cargo test` implies, and a test that has to
//! choose between skipping quietly and failing on a machine that was never
//! going to have those things is worse than a command run on purpose.
//!
//! What it checks that `slpc-rust`'s runner does not is the mapping DESIGN.md
//! §6 states from what the library answers to what the window shows. At slice 1
//! the window shows the verdict and nothing else, so the two overlap almost
//! entirely; the rows of §6 that add a tree and a card are checked here as
//! slices 2 and 3 land them. What is already this repository's own is that 77
//! containers built to break a reader produce a rendered state rather than a
//! panic.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use slpc::toml_edit::DocumentMut;
use slpc::Error;

use slipcase_desktop::{add_key, extract_at, rename_key, NewKey, Opened, Saved, Watch};

/// One disagreement, for the report: which case, what this build said about it,
/// and whatever the manifest had to say.
fn detail(c: &Case, said: &str) -> String {
    let mut out = format!("  {}\n      {said}", c.id);
    if !c.note.is_empty() {
        out.push_str("\n      ");
        out.push_str(&c.note);
    }
    out
}

/// One case, as the manifest describes it.
struct Case {
    id: String,
    expect: String,
    note: String,
    file: PathBuf,
}

/// The four answers a case may expect. An `expect` outside these is a manifest
/// this build cannot check rather than a case that failed.
const KNOWN: [&str; 4] = ["accept", "reject", "undetermined", "out-of-scope"];

/// Whether DESIGN.md §6 owes this container a tree past its verdict.
///
/// A conformant container's metadata member parsed, and so did that of one
/// declaring a version this build does not implement, so both show a tree.
/// `undetermined` is the answer given when the metadata member could not be
/// read, so there is nothing to show and showing something would contradict the
/// verdict. `reject` covers rows on both sides of the table: a container with no
/// metadata member has no tree, and one whose `payload.file` names nothing has
/// one. Nothing is owed there, and `None` says so.
fn owed_tree(expect: &str) -> Option<bool> {
    match expect {
        "accept" | "out-of-scope" => Some(true),
        "undetermined" => Some(false),
        _ => None,
    }
}

/// Whether DESIGN.md §6 owes this container a payload card.
///
/// One row of the table has one. A conformant container is the only kind with a
/// payload this build has located: a container declaring another version has a
/// payload the library deliberately did not look for, and every other row
/// failed before there was a payload to name.
fn owed_card(expect: &str) -> bool {
    expect == "accept"
}

/// An `s` where one is owed. `1 cases` is the tell of a program that was never
/// run on its failing path.
fn s(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

fn main() -> ExitCode {
    let Some(dir) = std::env::args_os().nth(1) else {
        eprintln!(
            "usage: corpus <conformance-directory>\n\n\
             The `conformance` directory of a checkout of excelano/slipcase."
        );
        return ExitCode::FAILURE;
    };
    match run(Path::new(&dir)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("corpus: {e}");
            ExitCode::FAILURE
        }
    }
}

/// What the run has seen.
#[derive(Default)]
struct Report {
    agreed: usize,
    trees: usize,
    cards: usize,
    extracted: usize,
    /// Conformant containers the Open button cannot serve. Not failures: SPEC
    /// §2.5 puts encryption and compression method outside conformance.
    unextractable: Vec<String>,
    rewritten: usize,
    renamed: usize,
    replaced: usize,
    /// Payloads the card refused before anything was pressed, in the same words
    /// extraction then used.
    foretold: usize,
    disagreements: BTreeMap<String, Vec<String>>,
}

impl Report {
    fn disagree(&mut self, what: String, c: &Case, said: &str) {
        self.disagreements.entry(what).or_default().push(detail(c, said));
    }

    /// Say what happened, and fail where anything did not agree.
    fn print(&self, total: usize) -> Result<(), String> {
        if self.disagreements.is_empty() {
            println!(
                "{total} cases, all agree. {} showed a metadata tree, {} a payload card.",
                self.trees, self.cards
            );
            println!(
                "{} of {} payloads extracted at their declared length.",
                self.extracted,
                self.extracted + self.unextractable.len()
            );
            for one in &self.unextractable {
                println!("  the Open button cannot serve {one}");
            }
            println!(
                "{} of those the card refused before anything was pressed, in the same words, and \
                 every payload it offered extracted.",
                self.foretold
            );
            println!(
                "{} rewritten through Repack and read back conformant, and untouched when nothing was edited.",
                self.rewritten
            );
            println!(
                "{} renamed a key and got the others back in the order they were written.",
                self.renamed
            );
            println!(
                "{} had their payload replaced, under its own name and under a new one.",
                self.replaced
            );
            return Ok(());
        }

        let failed = total - self.agreed;
        println!("{total} cases: {} agree, {failed} did not.\n", self.agreed);
        for (what, which) in &self.disagreements {
            println!("=== {what}  ({} case{})", which.len(), s(which.len()));
            for line in which {
                println!("{line}");
            }
            println!();
        }
        Err(format!("{failed} of {total} cases did not agree"))
    }
}

fn run(conformance: &Path) -> Result<(), String> {
    let cases = read_manifest(conformance)?;

    // A corpus that could not be found must never come out as agreement. The
    // counts below are worth reading only because these three refused first.
    if cases.is_empty() {
        return Err(format!(
            "{} describes no cases",
            conformance.join("manifest.toml").display()
        ));
    }
    missing_files(&cases)?;
    ungoverned_files(conformance, &cases)?;

    // One directory for all of them. Payload names repeat across the corpus, so
    // each extraction overwrites the last, which is all this needs: the check is
    // that the bytes arrived, not that they stayed.
    let scratch = tempfile::Builder::new()
        .prefix("slipcase-corpus-")
        .tempdir()
        .map_err(|e| format!("no temporary directory to extract into: {e}"))?;

    let mut report = Report::default();
    for c in &cases {
        check(c, scratch.path(), &mut report)?;
    }
    report.print(cases.len())
}

/// One case, against every column of DESIGN.md §6 this build has reached.
fn check(c: &Case, scratch: &Path, report: &mut Report) -> Result<(), String> {
    if !KNOWN.contains(&c.expect.as_str()) {
        return Err(format!(
            "{}: the manifest expects {:?}, which is not one of the four answers this build knows",
            c.id, c.expect
        ));
    }

    let opened = Opened::open(&c.file);
    let mut ok = true;

    let got = opened.verdict_word();
    if got != c.expect {
        ok = false;
        report.disagree(
            format!("the verdict: expected {}, got {got}", c.expect),
            c,
            &opened.verdict_line(),
        );
    }

    let shown = opened.metadata.is_some();
    if shown {
        report.trees += 1;
    }
    if let Some(owed) = owed_tree(&c.expect) {
        if shown != owed {
            ok = false;
            let said = if shown {
                "a tree, where §6 shows the verdict and nothing further"
            } else {
                "no tree, where §6 shows the verdict and the tree"
            };
            report.disagree(
                format!("the tree: a {} container showed {said}", c.expect),
                c,
                &opened.verdict_line(),
            );
        }
    }

    let card = opened.payload.is_some();
    if card {
        report.cards += 1;
    }
    if card != owed_card(&c.expect) {
        ok = false;
        let said = if card {
            "a card, where §6 gives one to a conformant container alone"
        } else {
            "no card, where §6 shows everything"
        };
        report.disagree(
            format!("the card: a {} container showed {said}", c.expect),
            c,
            &opened.verdict_line(),
        );
    }

    // Only a conformant container has a payload to extract, or metadata worth
    // writing back.
    if c.expect == "accept" {
        if !extracts(c, &opened, scratch, report) {
            ok = false;
        }
        if !rewrites(c, scratch, report) {
            ok = false;
        }
        if !replaces(c, scratch, report) {
            ok = false;
        }
    }

    if ok {
        report.agreed += 1;
    }
    Ok(())
}

/// Whether the payload can be replaced, DESIGN.md §5's second explicit action.
///
/// Twice, because the two cases are different writes. Under the payload's own
/// name the metadata has nothing to say, so it has to come back byte for byte:
/// the corpus holds a document with a byte order mark and one with CRLF line
/// endings, and both would be rewritten by a build that handed the document
/// over when nobody had edited it. Under a new name `payload.file` has to move
/// with the payload, which is the one key a replacement may change.
///
/// This runs on every conformant case, the encrypted payload included. Nothing
/// reads that member to replace it, so a container the Open button cannot serve
/// is still one whose payload can be swapped out.
///
/// On a copy. The corpus is the arbiter and nothing here may change it.
fn replaces(c: &Case, scratch: &Path, report: &mut Report) -> bool {
    let stem = c.id.replace('/', "-");
    let copy = scratch.join(format!("replace-{stem}.slpc"));
    if let Err(e) = std::fs::copy(&c.file, &copy) {
        report.disagree(
            "the replacement: the case could not be copied to write to".to_owned(),
            c,
            &e.to_string(),
        );
        return false;
    }

    let opened = Opened::open(&copy);
    let Some(payload) = &opened.payload else {
        report.disagree(
            "the replacement: a conformant container arrived without a card".to_owned(),
            c,
            &opened.verdict_line(),
        );
        return false;
    };
    let own_name = payload.name.clone();

    // Payload names repeat across the corpus, so each case gets a directory to
    // hold a file under the name its own container uses.
    let holding = scratch.join(format!("replacement-{stem}"));
    if let Err(e) = std::fs::create_dir_all(&holding) {
        report.disagree(
            "the replacement: no directory to hold the replacement file".to_owned(),
            c,
            &e.to_string(),
        );
        return false;
    }

    let same = holding.join(&own_name);
    let renamed = holding.join(REPLACEMENT);
    for (path, bytes) in [(&same, SAME_NAME), (&renamed, NEW_NAME)] {
        if let Err(e) = std::fs::write(path, bytes) {
            report.disagree(
                "the replacement: the replacement file could not be written".to_owned(),
                c,
                &e.to_string(),
            );
            return false;
        }
    }

    let before = match slpc::Container::open(&copy) {
        Ok(container) => container.metadata_bytes().to_vec(),
        Err(e) => {
            report.disagree(
                "the replacement: the copy could not be read".to_owned(),
                c,
                &e.to_string(),
            );
            return false;
        }
    };

    if !swapped(c, &copy, &same, &own_name, SAME_NAME, report) {
        return false;
    }

    // The one thing this write may not have touched.
    match slpc::Container::open(&copy) {
        Ok(container) if container.metadata_bytes() == before => {}
        Ok(_) => {
            report.disagree(
                "the replacement: replacing the payload rewrote metadata nobody edited".to_owned(),
                c,
                "the metadata member came back with different bytes",
            );
            return false;
        }
        Err(e) => {
            report.disagree(
                "the replacement: the replaced container could not be read".to_owned(),
                c,
                &e.to_string(),
            );
            return false;
        }
    }

    if !swapped(c, &copy, &renamed, REPLACEMENT, NEW_NAME, report) {
        return false;
    }

    // The old name is gone from the document as well as from the archive.
    let after = Opened::open(&copy);
    let document = after.metadata.as_ref().map(ToString::to_string).unwrap_or_default();
    if !document.contains(REPLACEMENT) {
        report.disagree(
            "the replacement: payload.file did not move with the payload".to_owned(),
            c,
            &document,
        );
        return false;
    }

    report.replaced += 1;
    true
}

/// Put a file in as the payload and read the container back.
fn swapped(
    c: &Case,
    copy: &Path,
    file: &Path,
    name: &str,
    bytes: &[u8],
    report: &mut Report,
) -> bool {
    let opened = Opened::open(copy);
    match opened.save(Some(file)) {
        Ok(Saved::Written) => {}
        Ok(Saved::Unchanged) => {
            report.disagree(
                "the replacement: a payload to write is something to write".to_owned(),
                c,
                "save said nothing had changed",
            );
            return false;
        }
        Ok(Saved::Refused(v)) => {
            report.disagree(
                "the replacement: what was written did not read back conformant".to_owned(),
                c,
                &v.to_string(),
            );
            return false;
        }
        Err(e) => {
            report.disagree("the replacement: the write failed".to_owned(), c, &e.to_string());
            return false;
        }
    }

    let after = Opened::open(copy);
    if after.verdict_word() != "accept" {
        report.disagree(
            "the replacement: the container is no longer conformant".to_owned(),
            c,
            &after.verdict_line(),
        );
        return false;
    }

    let Some(card) = &after.payload else {
        report.disagree(
            "the replacement: the replaced container has no card".to_owned(),
            c,
            &after.verdict_line(),
        );
        return false;
    };
    if card.name != name || card.size != bytes.len() as u64 {
        report.disagree(
            "the replacement: the card describes something other than what went in".to_owned(),
            c,
            &format!("{} at {} bytes, expecting {name} at {}", card.name, card.size, bytes.len()),
        );
        return false;
    }

    // And the bytes themselves, which is the only proof that the member holds
    // the file and not a reference to it.
    let out = copy.with_extension("payload");
    match extract_at(copy, &out, &Watch::new()) {
        Ok(_) => {}
        Err(e) => {
            report.disagree(
                "the replacement: the new payload would not come back out".to_owned(),
                c,
                &e.to_string(),
            );
            return false;
        }
    }
    match std::fs::read(&out) {
        Ok(got) if got == bytes => true,
        Ok(got) => {
            report.disagree(
                "the replacement: the payload that came back is not the one that went in".to_owned(),
                c,
                &format!("{} bytes back, {} in", got.len(), bytes.len()),
            );
            false
        }
        Err(e) => {
            report.disagree(
                "the replacement: the extracted payload could not be read".to_owned(),
                c,
                &e.to_string(),
            );
            false
        }
    }
}

/// What goes in under the payload's own name, and under a new one. Different
/// lengths, so a check that read the wrong one would not pass by accident.
const SAME_NAME: &[u8] = b"a payload put in under the name the container already used";
const NEW_NAME: &[u8] = b"a payload put in under a name of its own";

/// The new name, distinctive enough not to collide with a member the corpus
/// put in a container on purpose.
const REPLACEMENT: &str = "x-slipcase-desktop-replacement.bin";

/// Whether the container survives being written back: untouched when nothing
/// was edited, and still conformant when something was.
///
/// On a copy. The corpus is the arbiter and nothing here may change it.
fn rewrites(c: &Case, scratch: &Path, report: &mut Report) -> bool {
    let copy = scratch.join(format!("rewrite-{}.slpc", c.id.replace('/', "-")));
    if let Err(e) = std::fs::copy(&c.file, &copy) {
        report.disagree(
            "the rewrite: the case could not be copied to write to".to_owned(),
            c,
            &e.to_string(),
        );
        return false;
    }

    let untouched = Opened::open(&copy);
    let order_before = top_level_keys(&untouched);

    unedited_writes_nothing(c, &copy, &untouched, report)
        && edited_round_trips(c, &copy, &order_before, report)
        && rename_round_trips(c, &copy, &order_before, report)
}

/// The key added a moment ago, renamed, saved, and read back.
///
/// `rename_key` takes every entry out of the table and puts it back, which is
/// what keeps a renamed key in its place. Doing that to 37 real documents is
/// the check worth having: what could go wrong is the other keys moving.
fn rename_round_trips(
    c: &Case,
    copy: &Path,
    order_before: &[String],
    report: &mut Report,
) -> bool {
    let mut opened = Opened::open(copy);
    let Some(document) = opened.metadata.as_mut() else {
        return false;
    };
    if !rename_key(document.as_table_mut(), ADDED, RENAMED) {
        report.disagree(
            "the rename: the key added a moment ago would not rename".to_owned(),
            c,
            "rename_key refused a key it had just been shown",
        );
        return false;
    }

    match opened.save(None) {
        Ok(Saved::Written) => {}
        Ok(other) => {
            let said = match other {
                Saved::Unchanged => "a rename reported nothing to do".to_owned(),
                Saved::Refused(v) => format!("what was written did not validate: {v}"),
                Saved::Written => unreachable!(),
            };
            report.disagree("the rename: not written".to_owned(), c, &said);
            return false;
        }
        Err(e) => {
            report.disagree("the rename: the save failed".to_owned(), c, &e.to_string());
            return false;
        }
    }

    let again = Opened::open(copy);
    if again.verdict_word() != "accept" {
        report.disagree(
            "the rename: the container is no longer conformant".to_owned(),
            c,
            &again.verdict_line(),
        );
        return false;
    }

    let keys = top_level_keys(&again);
    if !keys.iter().any(|k| k == RENAMED) {
        report.disagree(
            "the rename: the new name is not there".to_owned(),
            c,
            &format!("{keys:?}"),
        );
        return false;
    }

    let others: Vec<String> = keys.into_iter().filter(|k| k != RENAMED).collect();
    if others != order_before {
        report.disagree(
            "the rename: it moved the other keys".to_owned(),
            c,
            &format!("{order_before:?} became {others:?}"),
        );
        return false;
    }

    report.renamed += 1;
    true
}

/// DESIGN.md §5: a container nothing has changed in is not written. Checked on
/// the bytes, because "not written" is a claim about the file.
fn unedited_writes_nothing(c: &Case, copy: &Path, untouched: &Opened, report: &mut Report) -> bool {
    let Ok(before) = std::fs::read(copy) else {
        return false;
    };

    match untouched.save(None) {
        Ok(Saved::Unchanged) => {}
        Ok(_) => {
            report.disagree(
                "the rewrite: an unedited container was written anyway".to_owned(),
                c,
                "DESIGN §5 says a container nothing has changed in is not written",
            );
            return false;
        }
        Err(e) => {
            report.disagree(
                "the rewrite: an unedited container would not save".to_owned(),
                c,
                &e.to_string(),
            );
            return false;
        }
    }

    if std::fs::read(copy).ok().as_ref() == Some(&before) {
        true
    } else {
        report.disagree(
            "the rewrite: an unedited save changed the file".to_owned(),
            c,
            "the bytes on disk moved with nothing edited",
        );
        false
    }
}

/// One unknown key added, saved, and read back: still conformant, the edit
/// still there, and the keys still in the order they were written.
fn edited_round_trips(
    c: &Case,
    copy: &Path,
    order_before: &[String],
    report: &mut Report,
) -> bool {
    let mut edited = Opened::open(copy);
    let Some(document) = edited.metadata.as_mut() else {
        report.disagree(
            "the rewrite: a conformant container had no document to edit".to_owned(),
            c,
            "every accept case parses its metadata member",
        );
        return false;
    };
    // SPEC §2.5 leaves unknown keys unconstrained, so adding one keeps every
    // case conformant.
    if !add_key(document.as_table_mut(), ADDED, NewKey::Text) {
        report.disagree(
            "the rewrite: a key could not be added".to_owned(),
            c,
            "add_key refused a name no case carries",
        );
        return false;
    }

    match edited.save(None) {
        Ok(Saved::Written) => {}
        Ok(Saved::Unchanged) => {
            report.disagree(
                "the rewrite: an edited container was not written".to_owned(),
                c,
                "a key was added and the save reported nothing to do",
            );
            return false;
        }
        Ok(Saved::Refused(v)) => {
            report.disagree(
                "the rewrite: what was written back did not validate".to_owned(),
                c,
                &v.to_string(),
            );
            return false;
        }
        Err(e) => {
            report.disagree("the rewrite: the save failed".to_owned(), c, &e.to_string());
            return false;
        }
    }

    // Read it back as a container rather than as the file just written.
    let again = Opened::open(copy);
    if again.verdict_word() != "accept" {
        report.disagree(
            "the rewrite: the rewritten container is no longer conformant".to_owned(),
            c,
            &again.verdict_line(),
        );
        return false;
    }
    if !again
        .metadata
        .as_ref()
        .is_some_and(|d| d.contains_key(ADDED))
    {
        report.disagree(
            "the rewrite: the edit did not survive the round trip".to_owned(),
            c,
            "the key added before saving is not in the container that came back",
        );
        return false;
    }

    // DESIGN.md §4: document order is preserved and never sorted. A rewrite is
    // where that would quietly stop being true.
    let order_after: Vec<String> = top_level_keys(&again)
        .into_iter()
        .filter(|k| k != ADDED)
        .collect();
    if order_after != order_before {
        report.disagree(
            "the rewrite: the keys came back in a different order".to_owned(),
            c,
            &format!("{order_before:?} became {order_after:?}"),
        );
        return false;
    }

    report.rewritten += 1;
    true
}

/// What the added key is renamed to, to exercise renaming on every case.
const RENAMED: &str = "x_slipcase_desktop_renamed";

/// The key this adds to prove a rewrite happened. SPEC §2.5 leaves unknown keys
/// unconstrained, so adding one keeps every case conformant.
const ADDED: &str = "x_slipcase_desktop_corpus";

/// The top-level keys of a container's metadata, in document order.
fn top_level_keys(opened: &Opened) -> Vec<String> {
    opened.metadata.as_ref().map_or_else(Vec::new, |d| {
        d.as_table().iter().map(|(k, _)| k.to_owned()).collect()
    })
}

/// Whether the payload came out whole, and whether a refusal was one SPEC §2.5
/// allows.
fn extracts(c: &Case, opened: &Opened, scratch: &Path, report: &mut Report) -> bool {
    // What the card said before anything was pressed, to be held against what
    // pressing it does. The pre-flight is only worth having if the two agree:
    // a card that greys the Open button on a payload that would have extracted
    // has taken away something that worked, and one that offers it on a
    // payload that cannot be decoded has put the refusal back where it was.
    let foretold = opened
        .payload
        .as_ref()
        .and_then(|p| p.unreadable.clone());

    match opened.extract_to(scratch) {
        Ok(path) => {
            if let Some(why) = &foretold {
                report.disagree(
                    "the pre-flight: the card refused a payload that then extracted".to_owned(),
                    c,
                    why,
                );
                return false;
            }
            let declared = opened.payload.as_ref().map_or(0, |p| p.size);
            let arrived = std::fs::metadata(&path).map(|m| m.len()).unwrap_or_default();
            if arrived == declared {
                report.extracted += 1;
                return true;
            }
            report.disagree(
                "the payload: extracted a different length than the central directory declared"
                    .to_owned(),
                c,
                &format!("{arrived} bytes arrived, {declared} declared"),
            );
            false
        }
        // A sound container whose payload this build cannot decode. Not a
        // disagreement, and the population the Open button cannot serve.
        Err(Error::Unsupported(u)) => {
            let said = u.to_string();
            match &foretold {
                Some(why) if *why == said => {
                    report.foretold += 1;
                    report.unextractable.push(format!("{}: {u}", c.id));
                    true
                }
                Some(why) => {
                    report.disagree(
                        "the pre-flight: the card refused for a different reason".to_owned(),
                        c,
                        &format!("the card said {why}, extraction said {said}"),
                    );
                    false
                }
                None => {
                    report.disagree(
                        "the pre-flight: the card offered a payload that cannot be decoded"
                            .to_owned(),
                        c,
                        &said,
                    );
                    false
                }
            }
        }
        Err(e) => {
            report.disagree(
                "the payload: a conformant container would not extract".to_owned(),
                c,
                &e.to_string(),
            );
            false
        }
    }
}

/// The cases the manifest describes.
fn read_manifest(conformance: &Path) -> Result<Vec<Case>, String> {
    let path = conformance.join("manifest.toml");
    let text = std::fs::read_to_string(&path).map_err(|e| {
        format!(
            "cannot read {}: {e}. Point this at the `conformance` directory of a checkout of excelano/slipcase.",
            path.display()
        )
    })?;
    let doc: DocumentMut = text
        .parse()
        .map_err(|e| format!("{} is not valid TOML: {e}", path.display()))?;

    let listed = doc
        .get("case")
        .and_then(slpc::toml_edit::Item::as_array_of_tables)
        .ok_or_else(|| format!("{} has no [[case]] tables", path.display()))?;

    let mut cases = Vec::with_capacity(listed.len());
    for (n, t) in listed.iter().enumerate() {
        let string = |key: &str| t.get(key).and_then(|v| v.as_str()).map(str::to_owned);
        let id = string("id").ok_or_else(|| format!("case {n} has no id"))?;
        let expect = string("expect").ok_or_else(|| format!("{id} has no expected verdict"))?;
        // A case whose subject is the container's name on disk says so.
        let file = conformance
            .join("cases")
            .join(string("filename").unwrap_or_else(|| format!("{id}.slpc")));

        cases.push(Case {
            id,
            expect,
            note: string("note").unwrap_or_default(),
            file,
        });
    }
    Ok(cases)
}

/// Refuse a corpus whose cases have not been generated.
fn missing_files(cases: &[Case]) -> Result<(), String> {
    let absent: Vec<&str> = cases
        .iter()
        .filter(|c| !c.file.exists())
        .map(|c| c.id.as_str())
        .collect();
    match absent.len() {
        0 => Ok(()),
        // All of them, which is what an ungenerated corpus looks like.
        n if n == cases.len() => Err(
            "no case files are there. Run `python3 generate.py` in that directory first; `cases/` is generated and is not committed."
                .to_owned(),
        ),
        n => Err(format!(
            "the manifest describes {n} case{} with no file: {}",
            s(n),
            absent.join(", ")
        )),
    }
}

/// Refuse a corpus holding containers the manifest says nothing about.
///
/// Reporting agreement on the cases that were described, while files sat beside
/// them unread, is the same false pass as reporting it on no cases at all.
fn ungoverned_files(conformance: &Path, cases: &[Case]) -> Result<(), String> {
    let described: BTreeSet<&Path> = cases.iter().map(|c| c.file.as_path()).collect();

    let mut loose = Vec::new();
    let mut dirs = vec![conformance.join("cases")];
    while let Some(d) = dirs.pop() {
        let entries =
            std::fs::read_dir(&d).map_err(|e| format!("cannot read {}: {e}", d.display()))?;
        for e in entries {
            let e = e.map_err(|e| format!("cannot read {}: {e}", d.display()))?;
            let p = e.path();
            if p.is_dir() {
                dirs.push(p);
            } else if p.extension().is_some_and(|x| x == "slpc") && !described.contains(p.as_path())
            {
                loose.push(p.display().to_string());
            }
        }
    }

    if loose.is_empty() {
        Ok(())
    } else {
        loose.sort();
        Err(format!(
            "cases/ holds {} container{} the manifest does not describe and nothing would have checked: {}",
            loose.len(),
            s(loose.len()),
            loose.join(", ")
        ))
    }
}
