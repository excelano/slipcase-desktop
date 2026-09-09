//! What the window knows about a container.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

pub mod opens_with;
mod staging;

/// The messages this application draws, in the language the desktop asks for.
///
/// `potext::catalog!` declares the catalogue and its four lookups **in this
/// crate**, which is the point of it being a macro: a single global inside
/// `potext` would be one catalogue shared by everything linked against it, and
/// `flyleaf` — which draws the metadata tree inside this window — has to carry
/// its own. The only thing that crosses either boundary is a language tag.
/// DESIGN.md §10.
pub mod i18n {
    pub use potext::fill;

    potext::catalog!();
}

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use flyleaf::flyleaf_core::Document;
use slpc::Verdict;

use i18n::{fill, t, tn};

// The editor's operations come from `flyleaf-core` now, and are re-exported
// under the names this crate has always had so that nothing calling them
// moved.
pub use flyleaf::flyleaf_core::{
    add_inline_key, add_key, remove_inline_key, remove_key, rename_inline_key, rename_key,
    set_value, Kind,
};

/// What this application tells the tree about a metadata document: the two
/// keys SPEC §2.2 requires are shown and not edited, and so is the table
/// holding one.
///
/// `payload.file` names the member the container is built around, and
/// changing it without renaming that member leaves a container naming a
/// payload that is not there; `slipcase_version` is the claim the whole
/// verdict rests on, and `Repack` refuses to write a document disagreeing with
/// the version it implements. `payload` is protected as well as
/// `payload.file`, because deleting or renaming the table takes the required
/// key inside it with it, which the value being read-only would not have
/// stopped.
///
/// This was the tree's own knowledge until 2026-09-07. It is the one thing in
/// the tree that was about Slipcase rather than about TOML, so it moved here
/// before the tree moved out to `excelano/flyleaf`, where it now is.
pub struct RequiredKeys;

impl flyleaf::Policy for RequiredKeys {
    fn protected(&self, path: &[String]) -> bool {
        let joined = path.join(".");
        [slpc::VERSION_KEY, slpc::PAYLOAD_FILE_KEY]
            .iter()
            .any(|required| *required == joined || required.starts_with(&format!("{joined}.")))
    }

    /// One of the two protected strings, `payload.file`, is a member name, and
    /// SPEC §3 requires a name be shown escaped. The card does that through
    /// `slpc::display_name` and the tree did not: a payload called
    /// `report<U+202E>fdp.exe` read `report\u{202E}fdp.exe` on the card and
    /// `reportfdp.exe` two rows below it, because egui gives a bidirectional
    /// formatting character zero advance width. The tree was showing the spoof
    /// the escaping exists to prevent, under a card that was not.
    ///
    /// Found by hand on Windows on 2026-08-29 against
    /// `accept/payload-name-bidi-override`, while running the card's item 3 —
    /// which asks about the card, so macOS and Linux had both ticked it without
    /// looking two rows down. The code is shared and all three platforms had
    /// this.
    fn display_protected<'a>(&self, value: &'a str) -> std::borrow::Cow<'a, str> {
        slpc::display_name(value)
    }
}

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
    let mut container = slpc::Container::open_with(container, LIMITS)?;
    // Through the library rather than by joining. A name checked against
    // SPEC 2.3 cannot leave the directory, and leaving it was never the only
    // way for a name to fail to be a file: `slpc::payload_path` is where that
    // now lives, having been this application's `destination` until 0.3.5.
    let out = slpc::payload_path(into, container.payload_name())?;
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
    let mut container = slpc::Container::open_with(container, LIMITS)?;
    copy_out(&mut container, source, out, watch)
}

/// The part both extractions share.
///
/// **A file may legitimately already be at `out`, on both paths.** The one
/// somebody named in a save dialog may exist because the dialog asked them
/// about it and they said yes. The handover directory may hold that name
/// because it is one directory for a whole session and they opened a container
/// with the same payload name earlier — which the conformance corpus found the
/// moment this refused, twenty-five cases into a run that shares one scratch
/// directory. There was a parameter here for the difference and it had one
/// value at both call sites, which is not a difference.
///
/// Replacing is safe on both and for the same reason: `Destination` renames a
/// finished file over the destination, and a rename replaces what is at a path
/// rather than following it. The handover directory is additionally this
/// process's own, mode 0700, with nothing else able to put anything in it.
fn copy_out(
    container: &mut slpc::Container<std::fs::File>,
    source: &Path,
    out: &Path,
    watch: &Watch,
) -> slpc::Result<Extracted> {
    // Asked for before anything is reserved, so a container that refuses leaves
    // nothing behind at all.
    let mut payload = container.payload()?;

    // Through the library rather than `File::create`, and what that replaces is
    // a defect rather than a style. `File::create` follows a symbolic link at
    // the destination and truncates whatever is on the other end, so extracting
    // into a directory where somebody had planted one wrote the payload
    // somewhere this code never chose — and the cleanup then removed the *link*
    // and left the damage, having deleted the only evidence of where the bytes
    // went. Measured 2026-08-27: a container whose payload fails its checksum
    // reported failure, left an empty destination, and put 200,000 bytes into a
    // file two directories away.
    //
    // `Destination` writes to a temporary file beside the destination and
    // renames it into place, so nothing exists at `out` until the payload is
    // whole. A rename replaces a symbolic link rather than following it, and a
    // failure or a cancellation now leaves whatever was there untouched instead
    // of truncating it and then deleting it.
    let mut landing = slpc::Destination::new(out, true)?;

    if matches!(copy(&mut payload, landing.writer(), watch)?, Extracted::Cancelled) {
        // `landing` drops here and takes its temporary file with it. Nothing at
        // `out` was ever opened, which is what lets the window say that nothing
        // was left behind and be right.
        return Ok(Extracted::Cancelled);
    }
    landing.commit()?;

    // After the commit rather than before, because the mark belongs to the file
    // a person will open and `commit` is what makes that file exist under its
    // own name. `provenance::carry` fails only where the platform gates opening
    // on a mark the container carried, so an error here is exactly the
    // laundering case, and a payload that would open without the warning its
    // origin earned must not be left under the name it is about to be handed
    // to the system under. DESIGN.md §5.
    //
    // **This is the one path that does not leave the destination as it found
    // it**, and the comment above claimed otherwise until it was read back.
    // The commit has already replaced whatever was at `out`, so removing takes
    // the replacement away and leaves nothing where a file used to be. That is
    // §5's decision rather than an oversight — an ungated payload under a name
    // somebody is about to open is the worse thing to leave — but it is a real
    // cost and it belongs written down beside the code that pays it. Not
    // reachable on Linux, where `carry`'s arm cannot fail.
    if let Err(why) = slpc::provenance::carry(source, out) {
        let _ = std::fs::remove_file(out);
        return Err(why);
    }
    Ok(Extracted::Done(out.to_owned()))
}

/// The copy itself, in chunks, stopping when asked.
fn copy(payload: &mut impl Read, into: &mut std::fs::File, watch: &Watch) -> slpc::Result<Extracted> {
    let mut buffer = vec![0u8; CHUNK];

    loop {
        if watch.is_cancelled() {
            return Ok(Extracted::Cancelled);
        }
        let n = payload.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        into.write_all(&buffer[..n])?;
        watch.advance(n);
    }

    into.flush()?;
    // `Done` carries a path for the caller that finished an extraction, and
    // this is not it: `copy_out` owns the destination and names it after the
    // commit. Rather than a sentinel path nothing reads, say what is actually
    // known here — whether the copy ran to the end.
    Ok(Extracted::Done(PathBuf::new()))
}

/// What became of making a container.
pub enum Created {
    /// Written, read back, and conformant. The container is at this path.
    Written {
        /// Where it is.
        path: PathBuf,
        /// What the platform records about where the payload came from and
        /// would not put on the container, where it would not. The card reads
        /// that record, so a container it could not be written onto is one the
        /// card will call local whatever the payload was.
        provenance: Option<String>,
    },
    /// Stopping was asked for, and nothing was left at the destination.
    Cancelled,
    /// What was written did not read back as a conformant container, so nothing
    /// was put at the destination.
    Refused(Verdict),
}

/// A payload being read into a container: counted, and stoppable.
///
/// [`slpc::pack_reader`] asks a payload for nothing but `Read`, which is what
/// lets both of those live here rather than in the library. `pack_file` is the
/// shorter call and has nowhere to put either, so a two-gigabyte payload would
/// pack behind a window that had stopped answering.
struct Watched<'a, R> {
    inner: R,
    watch: &'a Watch,
}

impl<R: Read> Read for Watched<'_, R> {
    /// An error rather than a quiet end of file, which is the difference
    /// between a cancel that cannot be committed by accident and one that can.
    /// Reporting the end of the payload would have `pack_reader` finish
    /// successfully on a truncated one, and every caller from then on would be
    /// one forgotten check away from committing it.
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.watch.is_cancelled() {
            return Err(std::io::Error::other("packing was stopped"));
        }
        let n = self.inner.read(buffer)?;
        self.watch.advance(n);
        Ok(n)
    }
}

/// Make a container holding `payload`, at `into`, watchably.
///
/// The metadata is empty, and the two keys SPEC §2.2 requires are the
/// library's to write: `pack_reader` puts `slipcase_version` and
/// `payload.file` in whatever it is handed. What a person wants to say about
/// the payload they say in the tree afterwards, which is the editor DESIGN.md
/// §4 and §5 already describe — this application has no schema with which to
/// ask them for anything else, and inventing a form here would be inventing a
/// vocabulary the specification does not define.
///
/// `into` is replaced where something is there, because the save dialog that
/// named it has already asked. `Destination` writes to a temporary file beside
/// it and renames, so nothing exists at `into` until the container is whole and
/// has been read back — the same reasoning, and the same call, as [`extract_at`].
///
/// # Errors
///
/// Returns whatever the library says about reading the payload or writing the
/// container. A payload whose name SPEC §2.3 forbids is one of them, and
/// [`why_not_a_payload`] is how the window refuses it earlier and in the same
/// words.
pub fn create(payload: &Path, into: &Path, watch: &Watch) -> slpc::Result<Created> {
    let name = payload_name(payload)?;
    let source = std::fs::File::open(payload)?;
    let mut destination = slpc::Destination::new(into, true)?;

    let mut watched = Watched {
        inner: source,
        watch,
    };
    let packed = slpc::pack_reader(
        name,
        &mut watched,
        slpc::toml_edit::DocumentMut::new(),
        destination.writer(),
    );
    // Asked before the error is, because a cancel arrives as one and because a
    // cancel arriving at the last read looks like a payload that ended. Either
    // way `destination` drops here and takes its temporary file with it, so
    // nothing was ever at `into`.
    if watch.is_cancelled() {
        return Ok(Created::Cancelled);
    }
    packed?;

    // Read back before anything is put where a person will look for it, which
    // is what `save` does for the same reason: a container is replaced or
    // created on evidence rather than on faith.
    let verdict = slpc::validate_with(destination.written()?, LIMITS)?;
    if !verdict.is_conformant() {
        return Ok(Created::Refused(verdict));
    }
    destination.commit()?;

    // **Packing launders a download without this, and that is a defect rather
    // than a nicety.** A payload the platform marked as having arrived from
    // elsewhere goes into a container this process wrote, which carries no
    // mark; `copy_out` then extracts it, asks `provenance::carry` about a
    // container that records nothing, and hands the platform an unmarked copy
    // of a file it had gated. The mark is carried here so that the round trip
    // through a container is not a way to remove one.
    //
    // Not fatal, which is `staging.rs`'s decision for the same question and for
    // the same reason: `carry` refuses when the copy would be ungated where the
    // original was gated, and that rule is written for a payload about to be
    // handed to the system. This is a container, and what opens a container is
    // this application, which reports provenance rather than acting on it. So
    // the container is kept and what could not be carried is said, rather than
    // a container that validated being thrown away over a line on a card.
    let provenance = slpc::provenance::carry(payload, into)
        .err()
        .map(|why| why.to_string());

    Ok(Created::Written {
        path: into.to_owned(),
        provenance,
    })
}

/// What the payload will be called inside the container.
///
/// `payload.file` is a TOML string, so a filename that is not UTF-8 is one this
/// format cannot express. `pack_file` makes the same refusal and keeps it in
/// the library; this is here because the count and the cancel above need
/// `pack_reader`, which takes the name rather than working it out.
fn payload_name(path: &Path) -> slpc::Result<&str> {
    path.file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| {
            std::io::Error::other(fill(
                t("{file} has a name that is not UTF-8, and payload.file is a TOML string"),
                &[("file", &path.display().to_string())],
            ))
            .into()
        })
}

/// A path, and what the library made of it.
pub struct Opened {
    /// The container as it was named, kept because the window shows it.
    pub path: PathBuf,
    /// What came back.
    pub outcome: Outcome,
    /// The metadata document, when the metadata member could be read and
    /// parsed as TOML, with its edited baseline and its history: what
    /// `as_parsed` held here until flyleaf 0.2, and undo and redo besides.
    ///
    /// `slpc::metadata_of` parses that member alone and asks nothing else of
    /// it, so a document survives a container that fails SPEC §2.1 somewhere
    /// else entirely: a required key absent, `payload.file` naming no member or
    /// several, a version this build does not implement. Those are the rows of
    /// DESIGN.md §6 that show a verdict and a tree. The rows that show a
    /// verdict and nothing further are the ones where this is `None`.
    pub metadata: Option<Document>,
    /// The payload, when there is one this build can describe.
    ///
    /// Only a conformant container has one. DESIGN.md §6 gives the card to that
    /// row alone: a container declaring a version this build does not implement
    /// has a payload the library never located, and every other row failed
    /// before there was a payload to name.
    pub payload: Option<Payload>,
    /// Whether the platform records this container as having arrived from
    /// elsewhere.
    ///
    /// The card says so rather than acting on it. A person deciding whether to
    /// open a payload is better served knowing where the container came from
    /// than being stopped, and what the platform does about the mark is the
    /// platform's business — which is DESIGN.md §3's rule applied to
    /// provenance instead of to type.
    pub from_elsewhere: bool,
}

/// What this application is willing to spend before it knows what it is holding.
///
/// SPEC §6 requires a bound on the metadata member and leaves the number to the
/// implementation. `slpc`'s default is 1 MiB, chosen against what parsing costs;
/// this is a quarter of that, chosen against what *rendering* costs, which is
/// the larger number here and which the library has no way to know about.
///
/// Measured 2026-08-27, against the densest conformant metadata anybody can
/// write — shortest legal keys, shortest legal values — with the window up:
///
/// | metadata | keys | resident |
/// | --- | --- | --- |
/// | none (baseline) | — | 135 MB |
/// | 256 KiB | 40,052 | 347 MB |
/// | 1 MiB | 152,399 | 891 MB |
///
/// About 8.7 KB per key, of which roughly 1.3 KB is the parsed document and the
/// rest is what `egui` retains for a row it has been shown. DESIGN.md §4's tree
/// renders every entry rather than the visible ones, which is right for a
/// metadata document and wrong for a hostile one, and this is the bound that
/// keeps the second from mattering.
///
/// 256 KiB is generous against every legitimate document: the format defines two
/// keys, SPEC §2.2's example is four lines, and the largest container in the
/// conformance corpus carries 64 KiB. A container over it is reported
/// undetermined, which is SPEC §6's answer and not a verdict against the file.
const LIMITS: slpc::Limits = {
    let mut l = slpc::Limits::DEFAULT;
    l.metadata_bytes = 256 << 10;
    l
};

/// The payload, as the card states it.
pub struct Payload {
    /// The member `payload.file` names.
    pub name: String,
    /// Its length uncompressed, read from the central directory.
    pub size: u64,
    /// What the platform says would open it, where the platform will say.
    pub opens_with: Option<String>,
    /// Whether the container records the payload as an executable file.
    ///
    /// DESIGN.md §5: the card says so, and says that the extracted copy will
    /// not be. False where the container records no mode at all, which is every
    /// container a non-Unix writer produced, so the card is silent rather than
    /// confident about a question nothing answered — `slpc::Container::payload_mode`
    /// is what keeps that distinction, reading the external attributes rather
    /// than taking the ZIP crate's invented answer.
    ///
    /// False on Windows whatever the container records. A mode bit is not what
    /// makes a file executable there, so the sentence would be untrue.
    pub executable: bool,
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
        return Some(fill(
            t("{file} has a name that is not UTF-8, and payload.file is a TOML string"),
            &[("file", &path.display().to_string())],
        ));
    };
    match slpc::check_payload_name(name) {
        Ok(()) => None,
        Err(why) => Some(fill(
            t("{name} cannot be a payload's name: {reason}"),
            &[("name", name), ("reason", &why.to_string())],
        )),
    }
}

/// Whether the container records the payload as an executable file.
///
/// One `#[cfg]` pair rather than a runtime test, because the answer on Windows
/// is not *no mode was recorded* — a container written on Linux records one and
/// this reads it fine there. It is that the question does not apply: what makes
/// a file executable on Windows is its extension and the shell, not a permission
/// bit, so a sentence about the bit would be false however the container was
/// written. DESIGN.md §5.
///
/// The Unix arm asks the library rather than the archive, and the difference is
/// the point. `slpc::Container::payload_mode` answers `None` where the container
/// records nothing, where the ZIP crate's own `unix_mode` would invent
/// `0o664` for an archive made on DOS and hand back a confident answer to a
/// question nobody asked. `None` here is `false`, and `false` is a silent card.
#[cfg(unix)]
fn executable<R: std::io::Read + std::io::Seek>(container: &slpc::Container<R>) -> bool {
    // Any of the three bits. A payload executable by its group and not its owner
    // is still a payload that was executable where it came from.
    container
        .payload_mode()
        .ok()
        .flatten()
        .is_some_and(|mode| mode & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable<R: std::io::Read + std::io::Seek>(_container: &slpc::Container<R>) -> bool {
    false
}

impl Payload {
    /// Describe the payload of a container already found conformant.
    fn of(path: &Path) -> Option<Self> {
        let container = slpc::Container::open_with(path, LIMITS).ok()?;
        let name = container.payload_name().to_owned();
        // Read from the central directory, so this decompresses nothing and a
        // payload whose compression or encryption this build cannot handle is
        // still described.
        let size = container.payload_size().ok()?;
        let opens_with = opens_with::opens_with(&name);
        // Borrows shared and decompresses nothing, so this costs the card the
        // central directory entry it already has.
        let unreadable = container.check_payload_readable().err().map(|u| u.to_string());
        let executable = executable(&container);
        Some(Self {
            name,
            size,
            opens_with,
            executable,
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
            // The only plural in the application, and the reason `tn` exists.
            // German shares English's two forms and its rule, so the catalogue
            // chooses between them the same way this line used to.
            return fill(tn("{n} byte", "{n} bytes", n), &[("n", &n.to_string())]);
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
        // The unit is a placeholder rather than part of the sentence: KiB and
        // MiB are the same in every language, and a translator given the whole
        // line as text would be invited to translate them.
        fill(
            t("{size} {unit} ({n} bytes)"),
            &[
                ("size", &format!("{scaled:.1}")),
                ("unit", unit),
                ("n", &n.to_string()),
            ],
        )
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
        // The library hands back a tree, and the Document is built from the
        // tree's text: what the edited baseline was compared against before
        // flyleaf 0.2, and a parse of a document the format bounds at 256 KiB.
        // Compared against rather than the bytes in the container, because
        // two of the corpus's conformant containers do not re-serialize to
        // the bytes they came from, and §5 says a container nothing has
        // changed in is not written.
        let metadata = std::fs::File::open(&path)
            .ok()
            .and_then(|f| slpc::metadata_of_with(f, LIMITS).ok())
            .and_then(|tree| Document::parse(&tree.to_string()).ok());

        let outcome = match std::fs::File::open(&path) {
            Err(e) => Outcome::Unreadable(e.to_string()),
            Ok(f) => match slpc::validate_with(f, LIMITS) {
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

        let from_elsewhere = slpc::provenance::arrived_from_elsewhere(&path);

        Self {
            path,
            outcome,
            metadata,
            payload,
            from_elsewhere,
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
            Outcome::Unreadable(why) => {
                // The reason is `slpc`'s own sentence and stays in English: a
                // table mapping the library's wording to German here would be
                // the library worked around, which this repository does not do.
                // DESIGN.md §10 records the gap and what closes it.
                fill(t("cannot be read: {reason}"), &[("reason", why)])
            }
            Outcome::Judged(v) => v.to_string(),
        }
    }

    /// Whether the metadata document has changed since it was parsed.
    #[must_use]
    pub fn metadata_edited(&self) -> bool {
        self.metadata.as_ref().is_some_and(Document::edited)
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

        let mut destination = staging::Staged::over(&self.path)?;
        {
            let source = std::fs::File::open(&self.path)?;
            let mut repack = slpc::Repack::new(source);
            // Only where it was edited. Handing the document over re-serializes
            // it, and §5 does not re-serialize what nobody touched: two of the
            // corpus's conformant containers come back changed by the round
            // trip alone. A payload replaced under a new name still moves
            // `payload.file`, which the library does from the stored bytes.
            if edited {
                repack = repack.metadata(document.tree());
            }
            if let Some(file) = replacing {
                repack = repack.payload_file(file)?;
            }
            repack.write(destination.writer())?;
        }

        // Read back before anything is replaced, which is the difference
        // between replacing the only copy of a container on faith and doing it
        // on evidence.
        let verdict = slpc::validate_with(destination.written()?, LIMITS)?;
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

    /// A container whose payload member records `mode` in its external
    /// attributes.
    ///
    /// Written by patching what `pack_reader` produced rather than by pulling
    /// in a ZIP writer. This application parses no containers and depends on no
    /// ZIP crate, which `CLAUDE.md` states as a property rather than an
    /// accident, and a dev-dependency that reads like one is not worth a
    /// tidier fixture. What the patch produces is what it says on the label: a
    /// container some other tool wrote, carrying a mode this one never records.
    fn with_payload_mode(dir: &std::path::Path, mode: u32) -> std::path::PathBuf {
        let document = slpc::toml_edit::DocumentMut::new();
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &b"payload"[..], document, &mut bytes).expect("packs");

        // Walk the central directory to the payload's header and write the mode
        // into its external attributes, which sit at offset 38 of the 46-byte
        // fixed part. `pack_reader` records nothing there, so this is the only
        // thing in the file that changes.
        let eocd = bytes
            .windows(4)
            .rposition(|w| w == 0x0605_4B50u32.to_le_bytes())
            .expect("an end of central directory record");
        let mut at =
            u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
        loop {
            assert_eq!(
                bytes[at..at + 4],
                0x0201_4B50u32.to_le_bytes(),
                "walked off the central directory without finding the payload"
            );
            let n = u16::from_le_bytes(bytes[at + 28..at + 30].try_into().unwrap()) as usize;
            let e = u16::from_le_bytes(bytes[at + 30..at + 32].try_into().unwrap()) as usize;
            let c = u16::from_le_bytes(bytes[at + 32..at + 34].try_into().unwrap()) as usize;
            if &bytes[at + 46..at + 46 + n] == b"report.pdf" {
                bytes[at + 38..at + 42].copy_from_slice(&(mode << 16).to_le_bytes());
                break;
            }
            at += 46 + n + e + c;
        }

        let path = dir.join("with-a-mode.slpc");
        std::fs::write(&path, &bytes).expect("writes");
        path
    }

    /// A container whose metadata is past what this application will spend is
    /// undetermined, and shows no tree.
    ///
    /// The defect this catches is a window that opens whatever it is given and
    /// finds out afterwards. Measured 2026-08-27 before the bound: 256 KiB of
    /// dense metadata cost 347 MB resident and 1 MiB cost 891 MB, against a
    /// 135 MB baseline, because DESIGN.md §4's tree renders every entry rather
    /// than the visible ones and `egui` retains about 8.7 KB for each row it has
    /// been shown. All of it inside a container this corpus calls conformant.
    ///
    /// Undetermined rather than reject, which is the half an over-eager fix gets
    /// wrong: the bound is this application's and SPEC §6 is explicit that a
    /// reader must not publish its own configuration as a property of somebody
    /// else's file.
    #[test]
    fn metadata_past_what_this_application_will_spend_is_undetermined() {
        use std::fmt::Write as _;

        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("dense.slpc");

        // The densest conformant shape: shortest legal keys and values.
        let mut metadata = String::from("slipcase_version = \"1.0\"\n\n[payload]\nfile = \"report.pdf\"\n\n[b]\n");
        for i in 0..80_000u32 {
            let _ = writeln!(metadata, "k{i}=1");
        }
        assert!(
            metadata.len() as u64 > super::LIMITS.metadata_bytes,
            "the fixture has to be over the bound to test it"
        );

        let document: slpc::toml_edit::DocumentMut = metadata.parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &b"payload"[..], document, &mut bytes).expect("packs");
        std::fs::write(&path, &bytes).expect("writes");

        let opened = super::Opened::open(&path);
        assert_eq!(opened.verdict_word(), "undetermined");
        assert!(opened.metadata.is_none(), "no tree for a document not read");
        assert!(opened.payload.is_none(), "and no card");
    }

    /// Extraction does not write through a symbolic link at the destination.
    ///
    /// **The defect this catches destroyed a file and hid where it went.**
    /// `copy` used `File::create`, which follows a link and truncates whatever
    /// is on the other end, and `copy_out` then removed the *link* on failure —
    /// so a container extracted into a directory where somebody had planted one
    /// wrote its payload two directories away, reported failure, left an empty
    /// destination, and deleted the only evidence. Measured 2026-08-27 with a
    /// payload whose stored checksum is a lie, which `validate` calls
    /// conformant because nothing reads a payload to reach a verdict.
    ///
    /// Break `copy_out` back to `File::create` and this fails at the first
    /// assertion.
    #[test]
    #[cfg(unix)]
    fn extraction_does_not_write_through_a_symlink() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let victim = dir.path().join("victim.txt");
        std::fs::write(&victim, b"IRREPLACEABLE").expect("writes");

        let into = dir.path().join("dest");
        std::fs::create_dir(&into).expect("a directory");
        std::os::unix::fs::symlink(&victim, into.join("report.pdf")).expect("links");

        // A conformant container whose payload will fail its checksum.
        let container = dir.path().join("c.slpc");
        let mut bytes = Vec::new();
        slpc::pack_reader(
            "report.pdf",
            &b"the payload"[..],
            slpc::toml_edit::DocumentMut::new(),
            &mut bytes,
        )
        .expect("packs");
        let crc = bytes
            .windows(4)
            .position(|w| w == 0x0403_4B50u32.to_le_bytes())
            .expect("a local header");
        bytes[crc + 14..crc + 18].copy_from_slice(&0xDEAD_BEEFu32.to_le_bytes());
        std::fs::write(&container, &bytes).expect("writes");

        let _ = super::extract(&container, &into, &super::Watch::new());

        assert_eq!(
            std::fs::read(&victim).expect("the victim survives"),
            b"IRREPLACEABLE",
            "the payload was written through the link"
        );
    }

    /// Stopping leaves the file somebody chose exactly as it was.
    ///
    /// The window says *Stopped. Nothing was left behind*, and until
    /// 2026-08-27 that was false: the destination was truncated before a byte
    /// was read and then removed, so cancelling a copy over a file somebody had
    /// chosen to replace deleted it. Catches a return to opening the
    /// destination before the payload is whole.
    #[test]
    fn cancelling_leaves_the_chosen_file_alone() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let chosen = dir.path().join("mine.pdf");
        std::fs::write(&chosen, b"MINE").expect("writes");

        let container = dir.path().join("c.slpc");
        let mut bytes = Vec::new();
        slpc::pack_reader(
            "report.pdf",
            &b"the payload"[..],
            slpc::toml_edit::DocumentMut::new(),
            &mut bytes,
        )
        .expect("packs");
        std::fs::write(&container, &bytes).expect("writes");

        let watch = super::Watch::new();
        watch.cancel();

        assert!(matches!(
            super::extract_at(&container, &chosen, &watch).expect("does not fail"),
            super::Extracted::Cancelled
        ));
        assert_eq!(std::fs::read(&chosen).expect("still there"), b"MINE");
    }

    /// A payload stored executable is reported as one, on Unix.
    ///
    /// DESIGN.md §5: the card says the extracted copy will not be executable,
    /// and it has to know. Catches the field being wired to nothing, which is
    /// what it was until `slpc` 0.3.6 gave it something to read.
    #[test]
    #[cfg(unix)]
    fn an_executable_payload_is_reported_as_one() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = with_payload_mode(dir.path(), 0o100_755);
        let opened = super::Opened::open(&path);
        assert!(
            opened.payload.expect("a card").executable,
            "0o755 is executable"
        );
    }

    /// A payload stored without an execute bit is not.
    ///
    /// The other direction, and the one that would make the card shout at
    /// everybody. Catches a test of the mode being present rather than of what
    /// it says.
    #[test]
    #[cfg(unix)]
    fn an_ordinary_payload_is_not() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = with_payload_mode(dir.path(), 0o100_644);
        let opened = super::Opened::open(&path);
        assert!(!opened.payload.expect("a card").executable);
    }

    /// A container recording no mode at all says nothing.
    ///
    /// This is the case the card must be silent for. What it does *not* catch
    /// is the defect its first draft claimed: the ZIP crate's `unix_mode`
    /// invents `0o664` for an archive made on DOS, and `0o664 & 0o111` is zero,
    /// so `executable` stays false and this passes whether the mode was
    /// invented or absent. `a_container_recording_no_mode_says_nothing` in
    /// `slpc`'s own tests is what holds that — it asserts `payload_mode()` is
    /// `None` and fails against the invention. This holds the smaller thing it
    /// can: that a container recording nothing produces no line.
    #[test]
    fn a_container_recording_no_mode_says_nothing() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = with_payload_mode(dir.path(), 0);
        let opened = super::Opened::open(&path);
        assert!(!opened.payload.expect("a card").executable);
    }

    fn sized(size: u64) -> Payload {
        Payload {
            name: "report.pdf".to_owned(),
            size,
            opens_with: None,
            executable: false,
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

        // Into the directory it was given, under the name the container gave —
        // asked of the filesystem rather than of the two strings. On Windows
        // `slpc::payload_path` answers in the verbatim form and expands 8.3
        // short names, so on a runner whose `TEMP` is `C:\Users\RUNNER~1\…`
        // the same file has two spellings and only one comparison holds.
        assert_eq!(out.file_name().expect("a filename"), "report.pdf");
        assert_eq!(
            std::fs::canonicalize(&out).expect("the path resolves"),
            std::fs::canonicalize(into.join("report.pdf")).expect("so does the join"),
            "the payload did not land in the directory it was given"
        );
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

        let mut into = std::fs::File::create(&out).expect("a file to write into");
        let outcome = super::copy(&mut reader, &mut into, &watch).expect("does not fail");

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
            document.tree_mut()["title"].as_value_mut().expect("a value"),
            Value::from("after"),
        );

        assert!(opened.metadata_edited());
        assert!(matches!(opened.save(None).expect("saves"), Saved::Written));

        let again = Opened::open(&path);
        assert_eq!(again.verdict_word(), "accept");
        let written = again.metadata.as_ref().expect("a document").render();

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
mod policy_tests {
    use super::RequiredKeys;
    use flyleaf::Policy;

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

        assert!(RequiredKeys.protected(&path(&["slipcase_version"])));
        assert!(RequiredKeys.protected(&path(&["payload", "file"])));
        assert!(RequiredKeys.protected(&path(&["payload"])));

        assert!(!RequiredKeys.protected(&path(&["title"])));
        assert!(!RequiredKeys.protected(&path(&["payload", "size"])));
        assert!(!RequiredKeys.protected(&path(&[
            "elsewhere",
            "slipcase_version"
        ])));
    }

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
            RequiredKeys.display_protected("report\u{202E}fdp.exe"),
            "report\\u{202E}fdp.exe"
        );
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
        let watch = super::Watch::new();
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

        let document = again.metadata.as_ref().expect("a document").render();
        assert!(document.contains("report-v2.pdf"), "{document}");
        assert!(!document.contains("report.pdf"), "{document}");
        // The one key the replacement may move, and no other part of the file.
        assert!(document.contains("# beside the string"), "{document}");

        let out = dir.path().join("out");
        std::fs::create_dir(&out).expect("a directory");
        extract(&container, &out, &super::Watch::new()).expect("extracts");
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
        extract(&container, &into, &super::Watch::new()).expect("extracts");
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
            opened.metadata.as_mut().expect("a document").tree_mut()["title"]
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
        let document = again.metadata.as_ref().expect("a document").render();
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

#[cfg(all(test, target_os = "windows"))]
mod windows_extraction_tests {
    use super::{extract, Extracted, Watch};
    use slpc::toml_edit::DocumentMut;

    /// Every name Windows resolves to a device wherever it appears. `LPT1` and
    /// `PRN` are here even though they failed cleanly rather than hanging,
    /// because a clean failure is still a conformant container this build
    /// refuses, and `NUL` is here because it succeeded while discarding the
    /// bytes, which is the worst of the three answers.
    const DEVICE_NAMES: [&str; 6] = ["CON", "CON.txt", "con", "COM1", "AUX", "NUL"];

    fn container_named(dir: &std::path::Path, payload_name: &str, payload: &[u8]) -> std::path::PathBuf {
        let container = dir.join(format!("holds-{}.slpc", payload_name.replace('.', "-")));
        let metadata: DocumentMut = "title = \"built by the test\"\n".parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader(payload_name, payload, metadata, &mut bytes).expect("packs");
        std::fs::write(&container, &bytes).expect("writes the container");
        container
    }

    /// The defect this catches is extraction handing back the console device
    /// instead of a file. `CON` is a legal payload name — `SPEC.md` §2.3
    /// accepts it and the conformance corpus carries a case for it — and Win32
    /// resolves the name to a device wherever it appears, so the payload went
    /// to the console, no file was written, and anything reading the result
    /// back waited forever on input that never came. The corpus met it for the
    /// first time on 2026-08-26 and hung there rather than disagreeing.
    ///
    /// The directory is listed before the payload is read, and that order is
    /// deliberate: against the defect the listing is empty and the test fails
    /// there, where reading first would hang the whole suite instead.
    #[test]
    fn a_payload_named_for_a_device_extracts_as_an_ordinary_file() {
        for name in DEVICE_NAMES {
            let dir = tempfile::tempdir().expect("a temporary directory");
            let payload = format!("bytes for {name}").into_bytes();
            let container = container_named(dir.path(), name, &payload);

            let out = match extract(&container, dir.path(), &Watch::new()) {
                Ok(Extracted::Done(path)) => path,
                Ok(Extracted::Cancelled) => panic!("{name}: an unwatched copy was cancelled"),
                Err(e) => panic!("{name}: extraction failed: {e}"),
            };

            let listed: Vec<_> = std::fs::read_dir(dir.path())
                .expect("the directory")
                .map(|e| e.expect("an entry").file_name())
                .collect();
            assert!(
                listed.iter().any(|entry| entry == name),
                "{name}: nothing by that name is in the directory, so the payload \
                 went to a device rather than to a file. Listed: {listed:?}"
            );

            assert_eq!(
                std::fs::read(&out).expect("the extracted payload"),
                payload,
                "{name}: the extracted bytes are not the payload"
            );
        }
    }

    /// The defect this catches is the repair above being applied to the path a
    /// person chose. `extract_at` takes a name somebody typed into a save
    /// dialog, and prefixing that would show them a path they did not choose
    /// and did not write; the doc comments have said all along that the two
    /// halves of extraction differ in whose name it is. Catches a repair that
    /// went into `copy_out` instead of into `extract`.
    #[test]
    fn a_path_a_person_chose_is_left_as_they_wrote_it() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let container = container_named(dir.path(), "report.pdf", b"payload bytes");
        let chosen = dir.path().join("where-they-said.pdf");

        let out = match super::extract_at(&container, &chosen, &Watch::new()) {
            Ok(Extracted::Done(path)) => path,
            Ok(Extracted::Cancelled) => panic!("an unwatched copy was cancelled"),
            Err(e) => panic!("extraction failed: {e}"),
        };

        assert_eq!(out, chosen, "the path handed back is not the one chosen");
        assert!(
            !out.to_string_lossy().starts_with(r"\\?\"),
            "a person's own path came back in the verbatim form"
        );
    }

}

#[cfg(test)]
mod create_tests {
    use super::{Opened, Outcome};

    /// A container this application made is one it can open, and the two keys
    /// SPEC §2.2 requires are in it without this code having written either.
    ///
    /// **The defect this catches is a New container… that produces something
    /// nothing will open.** Every other path in this application starts from a
    /// container somebody else wrote, so nothing here had ever asserted that
    /// what `create` writes is a container at all — and the metadata it hands
    /// `pack_reader` is empty, which is only conformant because the library
    /// fills the two keys in. A change that stopped it doing so would leave a
    /// window happily writing files that fail their own read-back.
    #[test]
    fn a_container_made_here_reads_back_conformant() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let payload = dir.path().join("report.pdf");
        std::fs::write(&payload, b"the payload").expect("writes the payload");
        let into = dir.path().join("report.pdf.slpc");

        let watch = super::Watch::new();
        let made = super::create(&payload, &into, &watch).expect("makes a container");
        let super::Created::Written { path, .. } = made else {
            panic!("a container was not made");
        };
        assert_eq!(path, into);

        let opened = Opened::open(&into);
        assert!(
            matches!(opened.outcome, Outcome::Judged(slpc::Verdict::Conformant)),
            "{}",
            opened.verdict_line()
        );
        let payload_in_it = opened.payload.expect("a card");
        assert_eq!(payload_in_it.name, "report.pdf");
        assert_eq!(payload_in_it.size, "the payload".len() as u64);

        let tree = opened.metadata.expect("a document");
        let tree = tree.tree();
        assert!(tree.get(slpc::VERSION_KEY).is_some(), "no version key");
        assert_eq!(
            tree["payload"]["file"].as_str(),
            Some("report.pdf"),
            "payload.file is not the name the payload went in under"
        );
        // And the count reached the end, or the progress bar is decoration.
        assert_eq!(watch.done(), "the payload".len() as u64);
    }

    /// Stopping leaves nothing at the destination, and does not report failure.
    ///
    /// **The defect this catches is a stop reported as a failure, and a
    /// truncated container under the name somebody asked for.** Broken
    /// deliberately: with the `is_cancelled` line taken out of `create`, this
    /// fails on the error `Watched::read` raises rather than on the file, which
    /// is the half that bites.
    ///
    /// The other half is why that read raises an error rather than reporting
    /// the end of the payload, and no test can reach it: reporting the end
    /// would have `pack_reader` return `Ok` on a container holding part of a
    /// payload, and only that one line would stand between it and a commit.
    /// Belt and braces, and this test holds the braces.
    #[test]
    fn a_container_that_is_stopped_is_not_left_behind() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let payload = dir.path().join("report.pdf");
        std::fs::write(&payload, vec![0u8; 512 * 1024]).expect("writes the payload");
        let into = dir.path().join("report.pdf.slpc");

        let watch = super::Watch::new();
        watch.cancel();
        let made = super::create(&payload, &into, &watch).expect("stops rather than failing");

        assert!(
            matches!(made, super::Created::Cancelled),
            "a stop was reported as something else"
        );
        assert!(!into.exists(), "a container was left at the destination");
    }

    /// A file the specification will not let be a payload does not become one,
    /// and nothing is left where the container was going.
    ///
    /// **The defect this catches is a half-written destination.**
    /// `Destination` is reserved before the payload is read, so a name refused
    /// inside `pack_reader` is refused after there is a temporary file — and
    /// the guarantee that nothing appears at the destination rests on that
    /// temporary file being dropped rather than committed.
    #[test]
    fn a_file_that_cannot_be_a_payload_makes_no_container() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        // The one name SPEC §2.3 reserves: the metadata member's own.
        let payload = dir.path().join(slpc::METADATA_MEMBER);
        std::fs::write(&payload, b"title = \"not a payload\"\n").expect("writes the file");
        let into = dir.path().join("refused.slpc");

        let refused = super::create(&payload, &into, &super::Watch::new());
        assert!(refused.is_err(), "the reserved name was accepted");
        assert!(!into.exists(), "something was left at the destination");
    }
}
