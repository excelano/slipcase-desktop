//! What the window knows about a container.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

pub mod opens_with;
pub mod provenance;
pub mod tree;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use slpc::toml_edit::{Datetime, DocumentMut, InlineTable, Item, Key, Table, Value};
use slpc::Verdict;

/// How much of a copy has happened, and whether it should stop.
///
/// Two handles onto the same counters, so the thread doing the copying and the
/// one drawing the window can hold one each. DESIGN.md §6 asks that a very
/// large payload be extractable with a duration: something to watch and
/// something to press.
#[derive(Clone, Default)]
pub struct Watch {
    done: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
}

impl Watch {
    /// A watch on a copy that has not started.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bytes written so far.
    #[must_use]
    pub fn done(&self) -> u64 {
        self.done.load(Ordering::Relaxed)
    }

    /// Ask the copy to stop. It stops at the end of the chunk it is on.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Whether stopping has been asked for.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn advance(&self, n: usize) {
        self.done.fetch_add(n as u64, Ordering::Relaxed);
    }
}

/// What became of an extraction that did not fail.
pub enum Extracted {
    /// The payload is on disk, here.
    Done(PathBuf),
    /// Stopping was asked for, and the part of the file that had been written
    /// is gone.
    Cancelled,
}

/// The size of one chunk, and so how long a cancel waits to be noticed.
const CHUNK: usize = 64 * 1024;

/// Copy a container's payload into a directory, watchably.
///
/// Takes a path rather than an [`Opened`] because the thread doing this holds
/// nothing else: `Container::payload` borrows its container, so no reader can
/// be sent across a thread and the worker has to open the container itself.
/// Reopening an [`Opened`] there would re-run the platform's type query, which
/// starts processes.
///
/// Nothing part-written survives a failure or a cancel. A half-copied file left
/// under the payload's own name is one somebody finds later and takes for the
/// payload.
///
/// # Errors
///
/// Returns whatever the library says about reading the container, and any error
/// from writing the file.
pub fn extract(container: &Path, into: &Path, watch: &Watch) -> slpc::Result<Extracted> {
    let source = container;
    let mut container = slpc::Container::open(container)?;
    // `Container::open` has already put this name through
    // `slpc::check_payload_name`, which rejects every separator and every
    // traversal, so joining it onto a directory cannot leave that directory.
    let out = into.join(container.payload_name());
    copy_out(&mut container, source, &out, watch)
}

/// Copy a container's payload to a path somebody chose, watchably.
///
/// The other half of DESIGN.md §5's extract: [`extract`] puts the payload where
/// the container names it, for handing to the platform, and this puts it where
/// a person named it, which is the explicit action. The name is theirs, so it
/// goes through no check of the library's: `check_payload_name` says what a
/// member may be called inside a container, and this file is leaving one.
///
/// # Errors
///
/// Returns whatever the library says about reading the container, and any error
/// from writing the file.
pub fn extract_at(container: &Path, out: &Path, watch: &Watch) -> slpc::Result<Extracted> {
    let source = container;
    let mut container = slpc::Container::open(container)?;
    copy_out(&mut container, source, out, watch)
}

/// The part both extractions share.
fn copy_out(
    container: &mut slpc::Container<std::fs::File>,
    source: &Path,
    out: &Path,
    watch: &Watch,
) -> slpc::Result<Extracted> {
    // Asked for before the file is created, so a container that refuses leaves
    // nothing behind at all.
    let mut payload = container.payload()?;

    let outcome = copy(&mut payload, out, watch).and_then(|copied| {
        // Inside the cleanup below rather than after it, because a payload
        // whose provenance could not be carried must not be left on disk under
        // the name a person is about to be handed. `provenance::carry` fails
        // only where the platform gates opening on a mark the container
        // carried, so an error here is exactly the laundering case.
        if matches!(copied, Extracted::Done(_)) {
            crate::provenance::carry(source, out)?;
        }
        Ok(copied)
    });
    if !matches!(outcome, Ok(Extracted::Done(_))) {
        // Including where the path was one somebody chose over a file they
        // already had. `File::create` truncated it before the first byte was
        // read, so those contents are gone either way, and a part-written file
        // under the name they chose is the worse thing to leave.
        let _ = std::fs::remove_file(out);
    }
    outcome
}

/// The copy itself, in chunks, stopping when asked.
fn copy(payload: &mut impl Read, out: &Path, watch: &Watch) -> slpc::Result<Extracted> {
    let mut file = std::fs::File::create(out)?;
    let mut buffer = vec![0u8; CHUNK];

    loop {
        if watch.is_cancelled() {
            return Ok(Extracted::Cancelled);
        }
        let n = payload.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])?;
        watch.advance(n);
    }

    file.flush()?;
    Ok(Extracted::Done(out.to_owned()))
}

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
    /// The document as it was parsed, for telling whether it has been edited.
    ///
    /// Compared against rather than the bytes in the container, because two of
    /// the corpus's 37 conformant containers do not re-serialize to the bytes
    /// they came from: a leading byte order mark is dropped and CRLF line
    /// endings come back as LF. Comparing against the stored bytes would call
    /// those two edited the moment they were opened, and §5 says a container
    /// nothing has changed in is not written.
    as_parsed: Option<String>,
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
    /// Why this build cannot decode the payload, where it cannot.
    ///
    /// SPEC §2.5 puts encryption and compression method outside conformance, so
    /// this is a fact about the build and not a verdict on the container:
    /// DESIGN.md §6's last row is conformant and out of reach at once. Asked
    /// before anything is offered rather than read off a failure afterwards,
    /// which is the difference between a button that is not offered and a
    /// button that does not work.
    pub unreadable: Option<String>,
}

impl Payload {
    /// Whether this build can decode the payload.
    ///
    /// Not a promise that extraction will succeed: the library says only that a
    /// decoder exists, and truncated bytes, a failed checksum, and an i/o error
    /// are all still ahead. It is enough to decide what to offer.
    #[must_use]
    pub fn can_be_decoded(&self) -> bool {
        self.unreadable.is_none()
    }
}

/// What became of a save.
pub enum Saved {
    /// Written, read back, and conformant. The container on disk is the new one.
    Written,
    /// Nothing in the document had changed, so nothing was written at all.
    /// DESIGN.md §5.
    Unchanged,
    /// What was written did not read back as a conformant container, so nothing
    /// was replaced and what is on disk is untouched.
    Refused(Verdict),
}

/// What a new key starts as.
///
/// The scalar types SPEC §2.2 leaves unconstrained, and a table to put them in.
/// An array and an array of tables are structure inside structure and are not
/// offered yet.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NewKey {
    /// An empty string.
    Text,
    /// Zero.
    Integer,
    /// Zero.
    Float,
    /// False.
    Boolean,
    /// The epoch, which is a date somebody will replace rather than a guess at
    /// the one they meant.
    Datetime,
    /// An empty table to put keys in.
    Table,
}

impl NewKey {
    /// The kinds an inline table can hold.
    ///
    /// It holds values, and a table is not one. A table inside an inline table
    /// would have to be another inline table, which is structure inside
    /// structure and is not offered any more than an array is.
    pub const SCALARS: [Self; 5] = [
        Self::Text,
        Self::Integer,
        Self::Float,
        Self::Boolean,
        Self::Datetime,
    ];

    /// Every kind, in the order a picker offers them.
    pub const ALL: [Self; 6] = [
        Self::Text,
        Self::Integer,
        Self::Float,
        Self::Boolean,
        Self::Datetime,
        Self::Table,
    ];

    /// What it is called where somebody chooses it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Boolean => "boolean",
            Self::Datetime => "date and time",
            Self::Table => "table",
        }
    }

    /// What it starts as, where only a value will do.
    fn as_value(self) -> Option<Value> {
        match self.item() {
            Item::Value(v) => Some(v),
            _ => None,
        }
    }

    fn item(self) -> Item {
        match self {
            Self::Text => Item::Value(Value::from("")),
            Self::Integer => Item::Value(Value::from(0_i64)),
            Self::Float => Item::Value(Value::from(0.0_f64)),
            Self::Boolean => Item::Value(Value::from(false)),
            Self::Datetime => Item::Value(Value::from(
                "1970-01-01T00:00:00Z"
                    .parse::<Datetime>()
                    .expect("the epoch is a datetime"),
            )),
            Self::Table => Item::Table(Table::new()),
        }
    }
}

/// Add a key to a table.
///
/// Refuses an empty name and one already there: inserting over an existing key
/// would replace it, and losing a value to a name collision is not what
/// pressing Add asks for.
pub fn add_key(t: &mut Table, name: &str, kind: NewKey) -> bool {
    if name.is_empty() || t.contains_key(name) {
        return false;
    }
    t.insert(name, kind.item());
    true
}

/// Remove a key, and everything under it where it is a table.
pub fn remove_key(t: &mut Table, name: &str) -> bool {
    t.remove(name).is_some()
}

/// Rename a key, keeping its place in the document and the decor it carried.
///
/// `toml_edit` has no rename. Removing and re-inserting would put the key at the
/// end, and DESIGN.md §4 says authoring order carries intent, so every entry is
/// taken out in order and put back with the renamed one rebuilt under its new
/// name. `remove_entry` hands back the `Key` itself, so the comments and
/// whitespace attached to it come along.
pub fn rename_key(t: &mut Table, from: &str, to: &str) -> bool {
    if to.is_empty() || from == to || !t.contains_key(from) || t.contains_key(to) {
        return false;
    }

    let names: Vec<String> = t.iter().map(|(k, _)| k.to_owned()).collect();
    let mut taken: Vec<(Key, Item)> = Vec::with_capacity(names.len());
    for name in &names {
        if let Some(entry) = t.remove_entry(name) {
            taken.push(entry);
        }
    }

    for (key, item) in taken {
        if key.get() == from {
            let renamed = Key::new(to)
                .with_leaf_decor(key.leaf_decor().clone())
                .with_dotted_decor(key.dotted_decor().clone());
            t.insert_formatted(&renamed, item);
        } else {
            t.insert_formatted(&key, item);
        }
    }
    true
}

/// Add a key to an inline table.
///
/// The same refusals as [`add_key`], and one more: a kind an inline table
/// cannot hold.
pub fn add_inline_key(t: &mut InlineTable, name: &str, kind: NewKey) -> bool {
    if name.is_empty() || t.contains_key(name) {
        return false;
    }
    match kind.as_value() {
        Some(value) => {
            t.insert(name, value);
            true
        }
        None => false,
    }
}

/// Remove a key from an inline table.
pub fn remove_inline_key(t: &mut InlineTable, name: &str) -> bool {
    t.remove(name).is_some()
}

/// Rename a key in an inline table, keeping its place and its decor.
///
/// The same rebuild [`rename_key`] does, for the same reason: there is no
/// rename, and re-inserting would move the key to the end of a line somebody
/// wrote in an order they chose.
pub fn rename_inline_key(t: &mut InlineTable, from: &str, to: &str) -> bool {
    if to.is_empty() || from == to || !t.contains_key(from) || t.contains_key(to) {
        return false;
    }

    let names: Vec<String> = t.iter().map(|(k, _)| k.to_owned()).collect();
    let mut taken: Vec<(Key, Value)> = Vec::with_capacity(names.len());
    for name in &names {
        if let Some(entry) = t.remove_entry(name) {
            taken.push(entry);
        }
    }

    for (key, value) in taken {
        if key.get() == from {
            let renamed = Key::new(to)
                .with_leaf_decor(key.leaf_decor().clone())
                .with_dotted_decor(key.dotted_decor().clone());
            t.insert_formatted(&renamed, value);
        } else {
            t.insert_formatted(&key, value);
        }
    }
    true
}

/// Change a value, keeping the decor it was written with.
///
/// Dropping a new `Item` over an old one discards its decor, which is the
/// whitespace and the comments attached to it, so the value is assigned into
/// and its decor put back afterwards. DESIGN.md §5.
pub fn set_value(slot: &mut Value, new: Value) {
    let decor = slot.decor().clone();
    *slot = new;
    *slot.decor_mut() = decor;
}

/// Why a chosen file cannot become a payload, where it cannot.
///
/// The same checks `Repack::payload_file` makes, asked at the moment somebody
/// chooses the file rather than at the moment they press Save. A name SPEC §2.3
/// forbids is a fact about the choice, and a person should hear it while they
/// still have the dialog in mind.
///
/// Says nothing about the one refusal this cannot see: a container already
/// holding another member under that name. That needs the container's member
/// list, which is not public, so it stays a failure Save reports.
#[must_use]
pub fn why_not_a_payload(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str();
    let Some(name) = name else {
        return Some(format!(
            "{} has a name that is not UTF-8, and payload.file is a TOML string",
            path.display()
        ));
    };
    match slpc::check_payload_name(name) {
        Ok(()) => None,
        Err(why) => Some(format!("{name} cannot be a payload's name: {why}")),
    }
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
        // Borrows shared and decompresses nothing, so this costs the card the
        // central directory entry it already has.
        let unreadable = container.check_payload_readable().err().map(|u| u.to_string());
        Some(Self {
            name,
            size,
            opens_with,
            unreadable,
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
        let as_parsed = metadata.as_ref().map(DocumentMut::to_string);

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
            as_parsed,
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

    /// Whether the metadata document has changed since it was parsed.
    #[must_use]
    pub fn metadata_edited(&self) -> bool {
        match (&self.metadata, &self.as_parsed) {
            (Some(doc), Some(as_parsed)) => doc.to_string() != *as_parsed,
            _ => false,
        }
    }

    /// Write the edits back into the container.
    ///
    /// `replacing` is a file to store as the payload, from DESIGN.md §5's
    /// second explicit action. Both edits go out in one write: they are two
    /// members of one archive, and writing them separately would rewrite the
    /// container twice and give a failure between the two a container carrying
    /// half of what was asked for.
    ///
    /// The sequence is the one DESIGN.md §5 asks for, and the order is what
    /// keeps a failure from costing the only copy of a container. `Repack`
    /// carries every member through, so members this build does not recognise
    /// survive as SPEC §3 requires. `Destination::in_place` writes to a
    /// temporary file beside the target, so nothing has been replaced yet.
    /// `Destination::written` hands that file back and it is validated there.
    /// Only then does `commit` rename it into place, and a `Destination`
    /// dropped without committing takes its temporary file with it.
    ///
    /// # Errors
    ///
    /// Returns whatever the library says about reading the container, reading
    /// the replacement payload, or writing the result. A replacement that reads
    /// back as anything but conformant is [`Saved::Refused`] rather than an
    /// error: nothing failed, and nothing was replaced.
    pub fn save(&self, replacing: Option<&Path>) -> slpc::Result<Saved> {
        let Some(document) = &self.metadata else {
            return Ok(Saved::Unchanged);
        };
        let edited = self.metadata_edited();
        if !edited && replacing.is_none() {
            return Ok(Saved::Unchanged);
        }

        let mut destination = slpc::Destination::in_place(&self.path)?;
        {
            let source = std::fs::File::open(&self.path)?;
            let mut repack = slpc::Repack::new(source);
            // Only where it was edited. Handing the document over re-serializes
            // it, and §5 does not re-serialize what nobody touched: two of the
            // corpus's conformant containers come back changed by the round
            // trip alone. A payload replaced under a new name still moves
            // `payload.file`, which the library does from the stored bytes.
            if edited {
                repack = repack.metadata(document);
            }
            if let Some(file) = replacing {
                repack = repack.payload_file(file)?;
            }
            repack.write(destination.writer())?;
        }

        // Read back before anything is replaced, which is the difference
        // between replacing the only copy of a container on faith and doing it
        // on evidence.
        let verdict = slpc::validate(destination.written()?)?;
        if !verdict.is_conformant() {
            return Ok(Saved::Refused(verdict));
        }

        destination.commit()?;
        Ok(Saved::Written)
    }

    /// Extract the payload into a directory, and say where it landed.
    ///
    /// Streamed rather than buffered whole: a payload is a file of arbitrary
    /// size, and `io::copy` moves it through a buffer of its own choosing.
    ///
    /// The failure that is not a defect is [`slpc::Error::Unsupported`], which
    /// is what a conformant container whose payload is encrypted or compressed
    /// by a method this build lacks comes back with. SPEC §2.5 puts both
    /// outside conformance, so the container is sound and the bytes are still
    /// out of reach.
    ///
    /// # Errors
    ///
    /// Returns whatever the library says about reading the container, and any
    /// error from writing the file.
    pub fn extract_to(&self, dir: &Path) -> slpc::Result<PathBuf> {
        match extract(&self.path, dir, &Watch::new())? {
            Extracted::Done(path) => Ok(path),
            // Nothing asked this one to stop: the watch it was given is one
            // nobody else holds.
            Extracted::Cancelled => unreachable!("an unwatched copy cannot be cancelled"),
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
            unreadable: None,
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

#[cfg(test)]
mod extraction_tests {
    use super::Opened;
    use slpc::toml_edit::DocumentMut;

    /// A container this test built itself, so nothing here needs the
    /// conformance corpus checked out. The payload is large enough to cross
    /// `io::copy`'s buffer several times, which is the part of streaming that a
    /// small fixture would not reach.
    #[test]
    fn a_payload_extracts_byte_for_byte() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = dir.path().join("built-by-the-test.slpc");

        let payload: Vec<u8> = (0..100_000u32).map(|i| u8::try_from(i % 251).unwrap()).collect();
        let metadata: DocumentMut = "title = \"built by the test\"\n"
            .parse()
            .expect("valid TOML");

        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &payload[..], metadata, &mut bytes).expect("packs");
        std::fs::write(&container, &bytes).expect("writes the container");

        let opened = Opened::open(&container);
        assert_eq!(opened.verdict_word(), "accept");

        let card = opened.payload.as_ref().expect("a conformant container has a card");
        assert_eq!(card.name, "report.pdf");
        assert_eq!(card.size, u64::try_from(payload.len()).unwrap());

        let into = dir.path().join("out");
        std::fs::create_dir(&into).expect("a directory to extract into");
        let out = opened.extract_to(&into).expect("extracts");

        // Into the directory it was given, under the name the container gave.
        assert_eq!(out, into.join("report.pdf"));
        assert_eq!(std::fs::read(&out).expect("reads it back"), payload);
    }

    /// The defect this catches is extraction laundering provenance: a container
    /// that arrived from somewhere, and a payload leaving it as though this
    /// machine had made it. On Linux the mark gates nothing, so what is checked
    /// here is that the carrying is wired into the extraction path at all —
    /// the platforms where it does gate opening run the same code down the
    /// same call.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_payload_leaves_a_downloaded_container_still_saying_so() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = dir.path().join("downloaded.slpc");

        let metadata: DocumentMut = "title = \"downloaded\"\n".parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &b"payload"[..], metadata, &mut bytes).expect("packs");
        std::fs::write(&container, &bytes).expect("writes the container");
        xattr::set(&container, "user.xdg.origin.url", b"https://example.invalid/a.slpc")
            .expect("marking the container as downloaded");

        let into = dir.path().join("out");
        std::fs::create_dir(&into).expect("a directory to extract into");
        let out = Opened::open(&container).extract_to(&into).expect("extracts");

        assert_eq!(
            xattr::get(&out, "user.xdg.origin.url").expect("reading the payload"),
            Some(b"https://example.invalid/a.slpc".to_vec()),
            "the payload left the container saying nothing about where it came from",
        );
    }

    /// The watch counts every byte, and a payload that is not a whole number
    /// of chunks still finishes at its declared size.
    #[test]
    fn progress_reaches_the_declared_size() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = dir.path().join("built-by-the-test.slpc");

        // Not a multiple of the chunk, so the last read is a short one.
        let payload = vec![7u8; 300_000];
        let metadata: DocumentMut = "title = \"watched\"\n".parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &payload[..], metadata, &mut bytes).expect("packs");
        std::fs::write(&container, &bytes).expect("writes");

        let into = dir.path().join("out");
        std::fs::create_dir(&into).expect("a directory");

        let watch = super::Watch::new();
        assert_eq!(watch.done(), 0);

        let out = super::extract(&container, &into, &watch).expect("extracts");
        assert!(matches!(out, super::Extracted::Done(_)));
        assert_eq!(watch.done(), u64::try_from(payload.len()).unwrap());
    }

    /// A cancel leaves nothing behind. A half-copied file under the payload's
    /// own name is one somebody finds later and takes for the payload.
    #[test]
    fn a_cancel_leaves_nothing_behind() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = dir.path().join("built-by-the-test.slpc");

        let payload = vec![7u8; 300_000];
        let metadata: DocumentMut = "title = \"stopped\"\n".parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &payload[..], metadata, &mut bytes).expect("packs");
        std::fs::write(&container, &bytes).expect("writes");

        let into = dir.path().join("out");
        std::fs::create_dir(&into).expect("a directory");

        let watch = super::Watch::new();
        watch.cancel();

        let out = super::extract(&container, &into, &watch).expect("does not fail");
        assert!(matches!(out, super::Extracted::Cancelled));
        assert_eq!(std::fs::read_dir(&into).expect("reads").count(), 0);
    }

    /// The copy stops at the end of the chunk it is on rather than part way
    /// through one. A reader that asks to stop while it is being read makes
    /// that exact, where a thread racing the copy would not.
    #[test]
    fn a_copy_stops_at_the_end_of_its_chunk() {
        struct CancelsWhileRead<'a> {
            watch: &'a super::Watch,
            left: usize,
        }
        impl std::io::Read for CancelsWhileRead<'_> {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if self.left == 0 {
                    return Ok(0);
                }
                let n = buf.len().min(self.left);
                self.left -= n;
                self.watch.cancel();
                Ok(n)
            }
        }

        let dir = tempfile::tempdir().expect("a temporary directory");
        let out = dir.path().join("report.pdf");
        let watch = super::Watch::new();
        let mut reader = CancelsWhileRead {
            watch: &watch,
            left: 10 * super::CHUNK,
        };

        let outcome = super::copy(&mut reader, &out, &watch).expect("does not fail");

        assert!(matches!(outcome, super::Extracted::Cancelled));
        // One chunk written, and the nine that would have followed are not.
        assert_eq!(watch.done(), u64::try_from(super::CHUNK).unwrap());
    }

    /// Nothing is written for a payload that cannot be read. The reader is
    /// asked for before the file is created, so a refusal leaves no empty file
    /// where a person would later find one and take it for the payload.
    #[test]
    fn a_container_that_is_not_one_leaves_no_file_behind() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = dir.path().join("not-a-container.slpc");
        std::fs::write(&container, b"this is not an archive").expect("writes");

        let into = dir.path().join("out");
        std::fs::create_dir(&into).expect("a directory to extract into");

        let opened = Opened::open(&container);
        assert_eq!(opened.verdict_word(), "reject");
        assert!(opened.extract_to(&into).is_err());
        assert_eq!(std::fs::read_dir(&into).expect("reads").count(), 0);
    }
}

#[cfg(test)]
mod save_tests {
    use super::{set_value, Opened, Saved};
    use slpc::toml_edit::{DocumentMut, Value};

    const METADATA: &str = "\
# a leading comment
title = \"before\"   # beside the title
zzz = \"written first\"
aaa = \"written second\"
";

    fn build(dir: &std::path::Path, metadata: &str) -> std::path::PathBuf {
        let path = dir.join("built-by-the-test.slpc");
        let document: DocumentMut = metadata.parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &b"payload"[..], document, &mut bytes).expect("packs");
        std::fs::write(&path, &bytes).expect("writes");
        path
    }

    /// DESIGN.md §5: a container nothing has changed in is not written. Checked
    /// on the bytes, because "not written" is a claim about the file.
    #[test]
    fn a_container_nothing_changed_in_is_not_written() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = build(dir.path(), METADATA);
        let before = std::fs::read(&path).expect("reads");

        let opened = Opened::open(&path);
        assert!(!opened.metadata_edited());
        assert!(matches!(opened.save(None).expect("saves"), Saved::Unchanged));

        assert_eq!(std::fs::read(&path).expect("reads"), before);
    }

    /// A byte order mark does not survive a parse and a re-serialization, so a
    /// container carrying one must not be called edited the moment it is
    /// opened. This is the case the comparison against the parsed document
    /// rather than the stored bytes exists for.
    #[test]
    fn a_container_with_a_byte_order_mark_is_not_written_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = build(dir.path(), METADATA);

        // Put a mark on it, which no document can carry through a parse.
        let with_mark = {
            let plain = slpc::Container::open(&path).expect("opens");
            let mut bytes = "\u{feff}".as_bytes().to_vec();
            bytes.extend_from_slice(plain.metadata_bytes());
            bytes
        };
        let marked = dir.path().join("marked.slpc");
        {
            let source = std::fs::File::open(&path).expect("opens");
            let out = std::fs::File::create(&marked).expect("creates");
            slpc::rewrite_metadata_bytes(source, &with_mark, out).expect("rewrites");
        }
        let before = std::fs::read(&marked).expect("reads");

        let opened = Opened::open(&marked);
        assert_eq!(opened.verdict_word(), "accept");
        assert!(!opened.metadata_edited(), "a mark is not an edit");
        assert!(matches!(opened.save(None).expect("saves"), Saved::Unchanged));

        assert_eq!(std::fs::read(&marked).expect("reads"), before);
    }

    /// One value changes and nothing else does: the comments stay where they
    /// were, and the keys stay in the order they were written rather than
    /// sorted.
    #[test]
    fn an_edit_changes_the_value_and_leaves_the_rest() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = build(dir.path(), METADATA);

        let mut opened = Opened::open(&path);
        let document = opened.metadata.as_mut().expect("a document");
        set_value(
            document["title"].as_value_mut().expect("a value"),
            Value::from("after"),
        );

        assert!(opened.metadata_edited());
        assert!(matches!(opened.save(None).expect("saves"), Saved::Written));

        let again = Opened::open(&path);
        assert_eq!(again.verdict_word(), "accept");
        let written = again.metadata.as_ref().expect("a document").to_string();

        assert!(written.contains("title = \"after\""), "{written}");
        assert!(written.contains("# a leading comment"), "{written}");
        assert!(written.contains("# beside the title"), "{written}");
        assert!(
            written.find("zzz").unwrap() < written.find("aaa").unwrap(),
            "written order, not sorted: {written}"
        );

        // The payload came through untouched, which is `Repack`'s doing rather
        // than this code's, and is the reason for using it. SPEC §3.
        let into = dir.path().join("out");
        std::fs::create_dir(&into).expect("a directory");
        let out = again.extract_to(&into).expect("extracts");
        assert_eq!(std::fs::read(out).expect("reads"), b"payload");
    }
}

#[cfg(test)]
mod structure_tests {
    use super::{add_key, remove_key, rename_key, NewKey};
    use slpc::toml_edit::DocumentMut;

    const DOC: &str = "\
# above first
first = \"one\"   # beside first
second = 2

[third]
inner = true
";

    fn doc() -> DocumentMut {
        DOC.parse().expect("valid TOML")
    }

    fn keys(d: &DocumentMut) -> Vec<String> {
        d.as_table().iter().map(|(k, _)| k.to_owned()).collect()
    }

    /// DESIGN.md §4: authoring order carries intent. A rename is where
    /// `toml_edit` would quietly move the key to the end, having no rename of
    /// its own.
    #[test]
    fn a_rename_keeps_its_place_and_its_comments() {
        let mut d = doc();
        assert!(rename_key(d.as_table_mut(), "first", "primary"));

        assert_eq!(keys(&d), ["primary", "second", "third"]);

        let written = d.to_string();
        assert!(written.contains("# above first"), "{written}");
        assert!(written.contains("# beside first"), "{written}");
        assert!(written.contains("primary = \"one\""), "{written}");
        assert!(!written.contains("first ="), "{written}");
    }

    /// A table renamed keeps its place too, and its contents come with it.
    #[test]
    fn a_table_can_be_renamed() {
        let mut d = doc();
        assert!(rename_key(d.as_table_mut(), "third", "provenance"));

        assert_eq!(keys(&d), ["first", "second", "provenance"]);
        assert!(d.to_string().contains("inner = true"));
    }

    /// A rename that would land on a name already there is refused rather than
    /// replacing it: losing a value to a collision is not what renaming asks
    /// for.
    #[test]
    fn a_rename_onto_an_existing_key_is_refused() {
        let mut d = doc();
        assert!(!rename_key(d.as_table_mut(), "first", "second"));
        assert!(!rename_key(d.as_table_mut(), "first", ""));
        assert!(!rename_key(d.as_table_mut(), "absent", "anything"));

        assert_eq!(keys(&d), ["first", "second", "third"]);
        assert!(d.to_string().contains("second = 2"));
    }

    /// A key added to a document that already has tables stays at the root.
    ///
    /// It goes last in the map, after the table, and `toml_edit` still writes it
    /// above the table's header. That is the difference between map order and
    /// document order, and it is the one that matters: a bare key written after
    /// `[third]` would be a key inside `third` rather than a key of the
    /// document, which is a different container.
    #[test]
    fn a_key_added_to_a_document_with_tables_stays_at_the_root() {
        let mut d = doc();
        assert!(add_key(d.as_table_mut(), "author", NewKey::Text));
        assert_eq!(keys(&d), ["first", "second", "third", "author"]);

        let written = d.to_string();
        assert!(written.contains("author = \"\""), "{written}");
        assert!(
            written.find("author").unwrap() < written.find("[third]").unwrap(),
            "an added key must not fall inside the last table: {written}"
        );

        // Read back rather than trusted: this is where it would go wrong.
        let back: DocumentMut = written.parse().expect("still parses");
        assert!(back.as_table().contains_key("author"));
        assert!(!back["third"]
            .as_table()
            .expect("a table")
            .contains_key("author"));
    }

    #[test]
    fn a_key_already_there_is_not_added_over() {
        let mut d = doc();
        assert!(!add_key(d.as_table_mut(), "first", NewKey::Integer));
        assert!(!add_key(d.as_table_mut(), "", NewKey::Integer));
        assert!(d.to_string().contains("first = \"one\""));
    }

    /// Every kind a picker offers produces a document that still parses.
    #[test]
    fn every_kind_of_new_key_is_valid_toml() {
        for kind in NewKey::ALL {
            let mut d = doc();
            assert!(add_key(d.as_table_mut(), "added", kind), "{kind:?}");
            let written = d.to_string();
            written
                .parse::<DocumentMut>()
                .unwrap_or_else(|e| panic!("{kind:?} wrote something unparseable: {e}\n{written}"));
        }
    }

    #[test]
    fn removing_a_table_takes_what_is_under_it() {
        let mut d = doc();
        assert!(remove_key(d.as_table_mut(), "third"));
        assert_eq!(keys(&d), ["first", "second"]);
        assert!(!d.to_string().contains("inner"));
        assert!(!remove_key(d.as_table_mut(), "third"));
    }
}

#[cfg(test)]
mod inline_tests {
    use super::{add_inline_key, remove_inline_key, rename_inline_key, NewKey};
    use slpc::toml_edit::DocumentMut;

    fn doc() -> DocumentMut {
        "owner = { name = \"D. Anderson\", team = \"consulting\" }\n"
            .parse()
            .expect("valid TOML")
    }

    fn owner(d: &mut DocumentMut) -> &mut slpc::toml_edit::InlineTable {
        d["owner"].as_inline_table_mut().expect("an inline table")
    }

    /// The bug this closes: the buttons were drawn inside an inline table and
    /// the change was thrown away, so `owner` could be removed and `owner.name`
    /// could not.
    #[test]
    fn a_key_inside_an_inline_table_can_be_removed() {
        let mut d = doc();
        assert!(remove_inline_key(owner(&mut d), "name"));
        assert!(!remove_inline_key(owner(&mut d), "name"));

        let written = d.to_string();
        assert!(!written.contains("name"), "{written}");
        assert!(written.contains("team = \"consulting\""), "{written}");
    }

    /// Renamed in place, not moved to the end of a line somebody wrote in an
    /// order they chose.
    #[test]
    fn a_key_inside_an_inline_table_keeps_its_place_when_renamed() {
        let mut d = doc();
        assert!(rename_inline_key(owner(&mut d), "name", "who"));

        let written = d.to_string();
        assert!(
            written.find("who").unwrap() < written.find("team").unwrap(),
            "{written}"
        );
        assert!(written.contains("who = \"D. Anderson\""), "{written}");

        // Refused for the same reasons as anywhere else.
        assert!(!rename_inline_key(owner(&mut d), "who", "team"));
        assert!(!rename_inline_key(owner(&mut d), "who", ""));
    }

    /// An inline table holds values, so a table is not among the kinds offered
    /// and is refused if it arrives anyway.
    #[test]
    fn an_inline_table_takes_values_and_not_tables() {
        let mut d = doc();
        assert!(add_inline_key(owner(&mut d), "since", NewKey::Integer));
        assert!(!add_inline_key(owner(&mut d), "nested", NewKey::Table));
        assert!(!add_inline_key(owner(&mut d), "name", NewKey::Text));

        assert!(!NewKey::SCALARS.contains(&NewKey::Table));

        let written = d.to_string();
        assert!(written.contains("since = 0"), "{written}");
        written.parse::<DocumentMut>().expect("still parses");
    }
}

#[cfg(test)]
mod replacement_tests {
    use super::{extract, extract_at, why_not_a_payload, Extracted, Opened, Saved, Watch};
    use slpc::toml_edit::DocumentMut;
    use std::path::{Path, PathBuf};

    /// A container built by the test, so nothing here needs the corpus.
    fn packed(dir: &Path, metadata: &str, name: &str, payload: &[u8]) -> PathBuf {
        let path = dir.join("built-by-the-test.slpc");
        let document: DocumentMut = metadata.parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader(name, payload, document, &mut bytes).expect("packs");
        std::fs::write(&path, &bytes).expect("writes the container");
        path
    }

    /// DESIGN.md §5's extract, as the explicit action: the file lands under the
    /// name somebody chose and not under the one the container carries.
    #[test]
    fn an_extraction_goes_where_it_was_told() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let payload = vec![3u8; 200_000];
        let container = packed(dir.path(), "title = \"chosen\"\n", "report.pdf", &payload);

        let out = dir.path().join("somewhere/else.bin");
        std::fs::create_dir(dir.path().join("somewhere")).expect("a directory");

        let watch = Watch::new();
        let landed = extract_at(&container, &out, &watch).expect("extracts");

        match landed {
            Extracted::Done(at) => assert_eq!(at, out),
            Extracted::Cancelled => panic!("nothing asked it to stop"),
        }
        assert_eq!(std::fs::read(&out).expect("reads it back"), payload);
        assert_eq!(watch.done(), u64::try_from(payload.len()).unwrap());
    }

    /// A cancel takes the part-written file with it, wherever it was going. The
    /// path here is one somebody chose, which is the case the scratch directory
    /// never reaches.
    #[test]
    fn a_cancelled_extraction_leaves_nothing_where_it_was_told() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = packed(dir.path(), "title = \"stopped\"\n", "report.pdf", &vec![9u8; 200_000]);

        let out = dir.path().join("half-a-payload.bin");
        let watch = Watch::new();
        watch.cancel();

        assert!(matches!(
            extract_at(&container, &out, &watch).expect("stops"),
            Extracted::Cancelled
        ));
        assert!(!out.exists(), "a part-written file is one somebody finds later");
    }

    /// Replacing the payload under a new name moves `payload.file` with it, and
    /// changes nothing else about the document.
    #[test]
    fn a_replaced_payload_takes_payload_file_with_it() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = packed(
            dir.path(),
            "title = \"before\" # beside the string\n",
            "report.pdf",
            b"the old payload",
        );

        let chosen = dir.path().join("report-v2.pdf");
        std::fs::write(&chosen, b"the new payload").expect("writes the replacement");

        let opened = Opened::open(&container);
        assert!(!opened.metadata_edited(), "nothing was typed into it");
        assert!(matches!(
            opened.save(Some(&chosen)).expect("saves"),
            Saved::Written
        ));

        let again = Opened::open(&container);
        assert_eq!(again.verdict_word(), "accept");

        let card = again.payload.as_ref().expect("a conformant container has a card");
        assert_eq!(card.name, "report-v2.pdf");
        assert_eq!(card.size, 15);

        let document = again.metadata.as_ref().expect("a document").to_string();
        assert!(document.contains("report-v2.pdf"), "{document}");
        assert!(!document.contains("report.pdf"), "{document}");
        // The one key the replacement may move, and no other part of the file.
        assert!(document.contains("# beside the string"), "{document}");

        let out = dir.path().join("out");
        std::fs::create_dir(&out).expect("a directory");
        extract(&container, &out, &Watch::new()).expect("extracts");
        assert_eq!(
            std::fs::read(out.join("report-v2.pdf")).expect("reads"),
            b"the new payload"
        );
    }

    /// A replacement alone does not re-serialize the metadata. DESIGN.md §5.
    ///
    /// The fixture's metadata has CRLF line endings, which a parse and a
    /// re-serialization does not reproduce: this is one of the two shapes in
    /// the conformance corpus that comes back changed by the round trip alone.
    /// Handing the document to `Repack` when nobody edited it would rewrite
    /// every line ending in a container whose payload was the only thing asked
    /// about.
    #[test]
    fn replacing_only_the_payload_leaves_the_metadata_byte_for_byte() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = packed(dir.path(), "title = \"placeholder\"\n", "report.pdf", b"old");

        let crlf: &[u8] =
            b"slipcase_version = \"1.0\"\r\ntitle = \"before\"\r\n\r\n[payload]\r\nfile = \"report.pdf\"\r\n";
        let mut out = std::io::Cursor::new(Vec::new());
        slpc::Repack::new(std::fs::File::open(&container).expect("opens"))
            .metadata_bytes(crlf)
            .write(&mut out)
            .expect("writes");
        std::fs::write(&container, out.into_inner()).expect("writes the container");
        assert_eq!(
            slpc::Container::open(&container).expect("opens").metadata_bytes(),
            crlf,
            "the fixture starts with the bytes this is about"
        );

        // The same name, so `payload.file` has nothing to move to either.
        let chosen = dir.path().join("report.pdf");
        std::fs::write(&chosen, b"the new payload").expect("writes the replacement");

        let opened = Opened::open(&container);
        assert!(!opened.metadata_edited());
        assert!(matches!(
            opened.save(Some(&chosen)).expect("saves"),
            Saved::Written
        ));

        assert_eq!(
            slpc::Container::open(&container).expect("opens").metadata_bytes(),
            crlf,
            "nobody edited the metadata, so nothing rewrote it"
        );

        let into = dir.path().join("out");
        std::fs::create_dir(&into).expect("a directory");
        extract(&container, &into, &Watch::new()).expect("extracts");
        assert_eq!(
            std::fs::read(into.join("report.pdf")).expect("reads"),
            b"the new payload",
            "and the payload is the one that was chosen"
        );
    }

    /// Both edits go out in one write.
    #[test]
    fn a_metadata_edit_and_a_replacement_are_one_save() {
        use slpc::toml_edit::Value;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = packed(
            dir.path(),
            "title = \"before\" # kept\n",
            "report.pdf",
            b"the old payload",
        );

        let chosen = dir.path().join("report-v2.pdf");
        std::fs::write(&chosen, b"the new payload").expect("writes the replacement");

        let mut opened = Opened::open(&container);
        super::set_value(
            opened.metadata.as_mut().expect("a document")["title"]
                .as_value_mut()
                .expect("a value"),
            Value::from("after"),
        );
        assert!(opened.metadata_edited());
        assert!(matches!(
            opened.save(Some(&chosen)).expect("saves"),
            Saved::Written
        ));

        let again = Opened::open(&container);
        let document = again.metadata.as_ref().expect("a document").to_string();
        assert!(document.contains("\"after\""), "{document}");
        assert!(document.contains("# kept"), "{document}");
        assert!(document.contains("report-v2.pdf"), "{document}");
        assert_eq!(
            again.payload.as_ref().expect("a card").name,
            "report-v2.pdf"
        );
    }

    /// Nothing to write is still nothing to write.
    #[test]
    fn no_edit_and_no_replacement_writes_nothing() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = packed(dir.path(), "title = \"untouched\"\n", "report.pdf", b"payload");
        let before = std::fs::read(&container).expect("reads");

        let opened = Opened::open(&container);
        assert!(matches!(opened.save(None).expect("saves"), Saved::Unchanged));

        assert_eq!(std::fs::read(&container).expect("reads"), before);
    }

    /// A name SPEC §2.3 forbids is refused where the file was chosen, not where
    /// Save was pressed.
    #[test]
    fn a_file_that_cannot_be_a_payload_says_so() {
        assert_eq!(why_not_a_payload(Path::new("/anywhere/report.pdf")), None);

        let reserved = why_not_a_payload(Path::new("/anywhere/slipcase.metadata.toml"))
            .expect("the metadata member's own name is reserved");
        assert!(reserved.contains("slipcase.metadata.toml"), "{reserved}");

        // Legal in a Linux filename, forbidden by SPEC §2.3, so it is a file
        // somebody can genuinely choose and genuinely cannot store.
        let colon = why_not_a_payload(Path::new("/anywhere/notes:2026.txt"))
            .expect("a colon is not a member name");
        assert!(colon.contains("notes:2026.txt"), "{colon}");
    }
}

#[cfg(test)]
mod readable_tests {
    use super::Opened;
    use slpc::toml_edit::DocumentMut;

    /// An ordinary container says its payload can be read, and says it without
    /// reading the payload: the answer comes from the central directory entry
    /// the card already collected.
    #[test]
    fn a_plain_payload_reports_readable() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("built-by-the-test.slpc");
        let metadata: DocumentMut = "title = \"readable\"\n".parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &b"payload"[..], metadata, &mut bytes).expect("packs");
        std::fs::write(&path, &bytes).expect("writes");

        let card = Opened::open(&path)
            .payload
            .expect("a conformant container has a card");
        assert!(card.can_be_decoded());
        assert_eq!(card.unreadable, None);
    }
}
