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

use slipcase_desktop::Opened;

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

    // Only a conformant container has a payload to extract.
    if c.expect == "accept" && !extracts(c, &opened, scratch, report) {
        ok = false;
    }

    if ok {
        report.agreed += 1;
    }
    Ok(())
}

/// Whether the payload came out whole, and whether a refusal was one SPEC §2.5
/// allows.
fn extracts(c: &Case, opened: &Opened, scratch: &Path, report: &mut Report) -> bool {
    match opened.extract_to(scratch) {
        Ok(path) => {
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
            report.unextractable.push(format!("{}: {u}", c.id));
            true
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
