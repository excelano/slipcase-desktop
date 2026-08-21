//! What the window knows about a container.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

pub mod tree;

use std::path::PathBuf;

use slpc::toml_edit::DocumentMut;
use slpc::Verdict;

/// A path, and what the library made of it.
pub struct Opened {
    /// The container as it was named, kept because the window shows it.
    pub path: PathBuf,
    /// What came back.
    pub outcome: Outcome,
    /// The metadata document, when the metadata member could be read and
    /// parsed as TOML.
    ///
    /// `slpc::metadata_of` parses that member alone and asks nothing else of
    /// it, so a document survives a container that fails SPEC §2.1 somewhere
    /// else entirely: a required key absent, `payload.file` naming no member or
    /// several, a version this build does not implement. Those are the rows of
    /// DESIGN.md §6 that show a verdict and a tree. The rows that show a
    /// verdict and nothing further are the ones where this is `None`.
    pub metadata: Option<DocumentMut>,
}

/// What opening a path produced.
///
/// Two arms over [`Verdict`]'s four. `slpc::validate` returns every verdict as
/// `Ok` and reserves `Err` for not being able to read the bytes at all, which
/// is a fact about the path rather than about a container. DESIGN.md §6 has no
/// row for it, because every row there is something a container can be and this
/// is something a path can be.
pub enum Outcome {
    /// The bytes could not be read, so there is nothing to judge.
    Unreadable(String),
    /// The library reached a verdict.
    Judged(Verdict),
}

impl Opened {
    /// Open a path and ask the library what it is.
    ///
    /// Returns no error of its own. Every way this can go wrong is one of the
    /// states DESIGN.md §6 requires the window to render rather than crash on.
    #[must_use]
    pub fn open(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        // Two reads rather than one. `Container::read` fails the payload check
        // before it yields a document, so the tree for a container that failed
        // that check has to be asked for separately.
        let metadata = std::fs::File::open(&path)
            .ok()
            .and_then(|f| slpc::metadata_of(f).ok());

        let outcome = match std::fs::File::open(&path) {
            Err(e) => Outcome::Unreadable(e.to_string()),
            Ok(f) => match slpc::validate(f) {
                Ok(v) => Outcome::Judged(v),
                // Always `Error::Io`: the library documents that everything a
                // container itself can be comes back as a verdict.
                Err(e) => Outcome::Unreadable(e.to_string()),
            },
        };
        Self {
            path,
            outcome,
            metadata,
        }
    }

    /// The container's name on disk, for the window's heading.
    #[must_use]
    pub fn name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        )
    }

    /// The line the window shows.
    ///
    /// [`Verdict`] states itself in full sentences, so a container that was
    /// read carries its own wording here and this adds none.
    #[must_use]
    pub fn verdict_line(&self) -> String {
        match &self.outcome {
            Outcome::Unreadable(why) => format!("cannot be read: {why}"),
            Outcome::Judged(v) => v.to_string(),
        }
    }

    /// This application's answer, in the conformance corpus's vocabulary.
    ///
    /// `manifest.toml` states one of the first four per case. The last two are
    /// answers no case may expect: one is a path that was never a container,
    /// and the other is a verdict added to the library after this was written.
    #[must_use]
    pub fn verdict_word(&self) -> &'static str {
        match &self.outcome {
            Outcome::Judged(Verdict::Conformant) => "accept",
            Outcome::Judged(Verdict::NonConformant(_)) => "reject",
            Outcome::Judged(Verdict::Undetermined(_)) => "undetermined",
            Outcome::Judged(Verdict::OutOfScope(_)) => "out-of-scope",
            Outcome::Unreadable(_) => "unreadable",
            // [`Verdict`] is non-exhaustive. A fifth answer is named rather
            // than folded into one of the four, because folding it would
            // report a container as something the library did not say it was.
            Outcome::Judged(_) => "unknown-verdict",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Opened, Outcome};

    /// The state DESIGN.md §6 has no row for, and the one the conformance
    /// corpus cannot reach: every case there is a container, and this is a path
    /// that is not one. Nothing else exercises it.
    #[test]
    fn a_path_that_is_not_there_is_unreadable() {
        let missing = std::env::current_dir()
            .expect("a working directory")
            .join("no-such-container-3f9a.slpc");
        let opened = Opened::open(missing);

        assert_eq!(opened.verdict_word(), "unreadable");
        assert!(matches!(opened.outcome, Outcome::Unreadable(_)));
        // Not silently blank: the window has a line to show for this.
        assert!(opened.verdict_line().starts_with("cannot be read: "));
    }

    /// A directory opens as a file on Linux and fails on the first read, so it
    /// reaches the same state by a different route.
    #[test]
    fn a_directory_is_unreadable() {
        let here = std::env::current_dir().expect("a working directory");
        let opened = Opened::open(&here);

        assert_eq!(opened.verdict_word(), "unreadable");
        // The last component, whatever this checkout is called. Naming the
        // repository here would fail in a clone under any other name.
        let want = here.file_name().expect("a named directory").to_string_lossy();
        assert_eq!(opened.name(), want);
    }
}
