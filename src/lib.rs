//! What the window knows about a container.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

pub mod opens_with;
pub mod tree;

use std::path::{Path, PathBuf};

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
    /// The payload, when there is one this build can describe.
    ///
    /// Only a conformant container has one. DESIGN.md §6 gives the card to that
    /// row alone: a container declaring a version this build does not implement
    /// has a payload the library never located, and every other row failed
    /// before there was a payload to name.
    pub payload: Option<Payload>,
}

/// The payload, as the card states it.
pub struct Payload {
    /// The member `payload.file` names.
    pub name: String,
    /// Its length uncompressed, read from the central directory.
    pub size: u64,
    /// What the platform says would open it, where the platform will say.
    pub opens_with: Option<String>,
}

impl Payload {
    /// Describe the payload of a container already found conformant.
    fn of(path: &Path) -> Option<Self> {
        let container = slpc::Container::open(path).ok()?;
        let name = container.payload_name().to_owned();
        // Read from the central directory, so this decompresses nothing and a
        // payload whose compression or encryption this build cannot handle is
        // still described.
        let size = container.payload_size().ok()?;
        let opens_with = opens_with::opens_with(&name);
        Some(Self {
            name,
            size,
            opens_with,
        })
    }

    /// The size, stated plainly.
    ///
    /// A payload of zero length is conformant under SPEC §2.3, and the card
    /// says nothing about it beyond this. DESIGN.md §6.
    #[must_use]
    pub fn size_line(&self) -> String {
        let n = self.size;
        if n < 1024 {
            return format!("{n} {}", if n == 1 { "byte" } else { "bytes" });
        }
        // The exact count stays: a card that only said "1.2 MiB" would have
        // rounded away the number somebody opened the container to read.
        let units = ["KiB", "MiB", "GiB", "TiB", "PiB"];
        #[allow(clippy::cast_precision_loss)]
        let mut scaled = n as f64 / 1024.0;
        let mut unit = units[0];
        for next in &units[1..] {
            if scaled < 1024.0 {
                break;
            }
            scaled /= 1024.0;
            unit = next;
        }
        format!("{scaled:.1} {unit} ({n} bytes)")
    }
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
        // Only a conformant container is given a card, so this opens the file
        // a third time and only for the row of §6 that has one.
        let payload = match &outcome {
            Outcome::Judged(Verdict::Conformant) => Payload::of(&path),
            _ => None,
        };

        Self {
            path,
            outcome,
            metadata,
            payload,
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

#[cfg(test)]
mod payload_tests {
    use super::Payload;

    fn sized(size: u64) -> Payload {
        Payload {
            name: "report.pdf".to_owned(),
            size,
            opens_with: None,
        }
    }

    /// A payload of zero length is conformant under SPEC §2.3, and the card
    /// states its size and editorialises none of it. DESIGN.md §6.
    #[test]
    fn a_zero_length_payload_states_its_size() {
        assert_eq!(sized(0).size_line(), "0 bytes");
    }

    #[test]
    fn one_byte_is_not_one_bytes() {
        assert_eq!(sized(1).size_line(), "1 byte");
    }

    #[test]
    fn small_sizes_are_bytes_alone() {
        assert_eq!(sized(1023).size_line(), "1023 bytes");
    }

    /// The exact count survives the scaling: somebody opened the container to
    /// read the number, and 1.2 MiB has rounded it away.
    #[test]
    fn large_sizes_keep_their_exact_count() {
        assert_eq!(sized(1024).size_line(), "1.0 KiB (1024 bytes)");
        assert_eq!(sized(1_536).size_line(), "1.5 KiB (1536 bytes)");
        assert_eq!(sized(5_242_880).size_line(), "5.0 MiB (5242880 bytes)");
        assert_eq!(
            sized(3_221_225_472).size_line(),
            "3.0 GiB (3221225472 bytes)"
        );
    }
}
