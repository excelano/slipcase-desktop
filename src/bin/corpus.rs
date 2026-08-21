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

use slipcase_desktop::Opened;

/// One case, as the manifest describes it.
struct Case {
    id: String,
    expect: String,
    note: String,
    file: PathBuf,
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

    let mut agreed = 0usize;
    let mut disagreements: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for c in &cases {
        let opened = Opened::open(&c.file);
        let got = opened.verdict_word();
        if got == c.expect {
            agreed += 1;
        } else {
            let mut detail = format!("  {}\n      {}", c.id, opened.verdict_line());
            if !c.note.is_empty() {
                detail.push_str("\n      ");
                detail.push_str(&c.note);
            }
            disagreements
                .entry(format!("expected {}, got {got}", c.expect))
                .or_default()
                .push(detail);
        }
    }

    let total = cases.len();
    if disagreements.is_empty() {
        println!("{total} cases, all agree.");
        return Ok(());
    }

    println!("{total} cases: {agreed} agree, {} did not.\n", total - agreed);
    for (what, which) in &disagreements {
        println!("=== {what}  ({} case{})", which.len(), s(which.len()));
        for line in which {
            println!("{line}");
        }
        println!();
    }
    Err(format!("{} of {total} cases did not agree", total - agreed))
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
