//! Where a container came from, carried onto the payload taken out of it.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)
//
//! A container downloaded from the internet is marked as such by the platform
//! that downloaded it: `com.apple.quarantine` on macOS, a `Zone.Identifier`
//! stream on Windows. Both are consulted before a file is opened, and both are
//! properties of the file rather than of its contents — so a copy written by
//! this application carries neither unless this module puts them there.
//!
//! **Except under the macOS App Sandbox, where the platform marks the copy
//! first.** Measured 2026-08-25: a payload extracted by a sandboxed build came
//! out carrying `com.apple.quarantine` naming this application, from a
//! container carrying none, and the write this module then attempted was
//! refused — replacing one quarantine value with another is how forgery would
//! work. So the premise above is false in that one configuration, and the rule
//! below is written to survive it being false.
//!
//! Without it, extracting is laundering. A person downloads a container, opens
//! it here, and the payload reaches its handler as something this machine
//! created rather than as something that arrived from elsewhere; the warning
//! the platform would have shown never appears. That is the shape of defect
//! that made disk images and archives the delivery vehicle of choice, and it is
//! why `slpc` refusing a payload name with a separator is not the end of what
//! extraction owes.
//!
//! **The policy lives here rather than in the caller.** [`carry`] fails only
//! when the platform keeps a mark that gates opening, the source carries one,
//! and the copy ends up carrying none. Everything else — no mark, no such mark
//! on this platform, a note nothing enforces, a mark the platform put there
//! itself — succeeds. So the rule for a caller about to hand a payload to the
//! system is the whole of the rule: an error means do not open it.
//!
//! That is deliberately a test of the copy rather than of this module's own
//! success. What the paragraph above calls laundering is a payload reaching its
//! handler looking like something this machine made, and the warning that then
//! never appears; it is not the absence of one particular value. A copy the
//! platform marked is gated, so the harm does not arise, and the source's own
//! value — which agent, which download — is detail this module loses rather
//! than a control it gives up. Testing the file rather than the environment is
//! also why nothing here asks whether it is sandboxed.

use std::io;
use std::path::Path;

/// What was carried from a container onto the payload extracted from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// The source said where it came from, and the copy now says the same. The
    /// platform will consult it before opening the payload.
    Carried,
    /// The same, except that nothing on this platform consults it. Linux keeps
    /// provenance as a note rather than as a gate, so this is hygiene and not a
    /// control, and it is a separate answer so that nothing mistakes it for one.
    Noted,
    /// The copy already said it arrived from elsewhere, and this module is not
    /// what put it there. The platform marks what a sandboxed process writes
    /// and then refuses to have that mark replaced, so the source's own value
    /// is lost while the gate it exists for is in place. A separate answer from
    /// [`Carried`](Mark::Carried) because the copy does not say what the source
    /// said, only that it came from somewhere.
    AlreadyMarked,
    /// The source said nothing about where it came from, or this platform keeps
    /// nothing that would say.
    Silent,
}

/// Carry whatever the platform records about `from` onto `to`.
///
/// # Errors
///
/// Returns the write error when this platform gates opening on a mark, `from`
/// carries one, and `to` ends up carrying none. A caller that is about to open
/// `to` must not, because the copy would be trusted where the original was not.
pub fn carry(from: &Path, to: &Path) -> io::Result<Mark> {
    match platform::carry(from, to) {
        // A refused write is only a failure if the copy is unmarked after it.
        // Under the App Sandbox it is not: the platform marked the copy on
        // creation, which is both why the write was refused and why refusing it
        // costs nothing that matters. Asked of the file rather than of the
        // process, so this is one branch on all three platforms and not a
        // sandbox check.
        Err(_) if platform::carries_a_mark(to) => Ok(Mark::AlreadyMarked),
        other => other,
    }
}

/// Whether the platform records this file as having arrived from elsewhere.
///
/// The card says so, because a person deciding whether to open a payload is
/// better served by knowing where the container came from than by being stopped
/// from opening it. What the platform will then do about the mark is the
/// platform's business — DESIGN.md §3's rule, applied to provenance rather than
/// to type — so this reports and does not gate.
///
/// **Not the same question as whether the file is gated**, and the two were one
/// function until this application began writing marks of its own. Under the
/// App Sandbox the platform marks whatever this process writes, so a container
/// this application saved carries a mark that says only that it was saved here.
/// [`carry`] wants the gating question and asks `carries_a_mark`; this one is
/// about origin and disregards a mark whose agent is this application. Anything
/// it cannot read as ours it reports, because over-reporting provenance costs a
/// person one line of caution and under-reporting it is the defect this whole
/// module exists to prevent.
#[must_use]
pub fn arrived_from_elsewhere(path: &Path) -> bool {
    platform::arrived_from_elsewhere(path)
}

// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
mod platform {
    use super::{io, Mark, Path};

    /// The attribute Launch Services and Gatekeeper both read. Its value
    /// encodes the agent, a timestamp, and an event identifier, and none of
    /// that is this application's business: it is copied as opaque bytes,
    /// because rewriting it would be claiming the download was ours.
    const QUARANTINE: &str = "com.apple.quarantine";

    pub fn carry(from: &Path, to: &Path) -> io::Result<Mark> {
        match xattr::get(from, QUARANTINE)? {
            Some(value) => {
                xattr::set(to, QUARANTINE, &value)?;
                Ok(Mark::Carried)
            }
            None => Ok(Mark::Silent),
        }
    }

    /// Whether anything at all will consult a mark before opening this file.
    pub fn carries_a_mark(path: &Path) -> bool {
        value_of(path).is_some()
    }

    pub fn arrived_from_elsewhere(path: &Path) -> bool {
        match value_of(path) {
            Some(value) => !this_application_wrote(&value),
            None => false,
        }
    }

    fn value_of(path: &Path) -> Option<Vec<u8>> {
        xattr::get(path, QUARANTINE).ok().flatten()
    }

    /// Whether the mark records this application writing the file rather than
    /// the file arriving from anywhere.
    ///
    /// The value is `flags;timestamp;agent;event-uuid`, and the agent is the
    /// only field read here — the rest stays the opaque thing the constant
    /// above says it is. Measured under a sandbox on 2026-08-25, the agent of a
    /// mark the platform wrote on this application's behalf is the executable's
    /// own filename, so that is what it is compared against rather than a
    /// string spelled out here: a binary renamed keeps agreeing with itself.
    ///
    /// Every uncertainty answers false, which reports the file as having
    /// arrived from elsewhere. A value with no third field, an executable this
    /// process cannot name: none of those are evidence that the mark is ours,
    /// and the safe direction is to keep saying so.
    fn this_application_wrote(value: &[u8]) -> bool {
        use std::os::unix::ffi::OsStrExt;

        let Some(agent) = value.split(|b| *b == b';').nth(2) else {
            return false;
        };
        let Ok(us) = std::env::current_exe() else {
            return false;
        };
        us.file_name().is_some_and(|name| name.as_bytes() == agent)
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use super::{io, Mark, Path};
    use std::ffi::OsString;

    /// The alternate data stream every downloader writes and the shell reads.
    /// It needs no API of its own: a stream is addressed by appending `:name`
    /// to the path, so `std::fs` reaches it and nothing here is FFI.
    const ZONE: &str = ":Zone.Identifier";

    fn stream_of(path: &Path) -> OsString {
        let mut named = path.as_os_str().to_os_string();
        named.push(ZONE);
        named
    }

    pub fn carry(from: &Path, to: &Path) -> io::Result<Mark> {
        let zone = match std::fs::read(stream_of(from)) {
            Ok(bytes) => bytes,
            // No stream is the ordinary case for a container somebody made
            // here, and is not a failure to carry anything.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Mark::Silent),
            Err(e) => return Err(e),
        };
        std::fs::write(stream_of(to), zone)?;
        Ok(Mark::Carried)
    }

    /// The same question here. Nothing on Windows marks what this
    /// application writes, so a stream on a file means the file arrived
    /// carrying one.
    pub fn carries_a_mark(path: &Path) -> bool {
        arrived_from_elsewhere(path)
    }

    pub fn arrived_from_elsewhere(path: &Path) -> bool {
        std::fs::metadata(stream_of(path)).is_ok()
    }
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{io, Mark, Path};

    /// What browsers write on a downloaded file, by freedesktop convention.
    /// Nothing consults either one before opening it — Linux has no counterpart
    /// to quarantine or the zone stream — so carrying them preserves provenance
    /// and gates nothing, which is what `Mark::Noted` says.
    const ORIGIN: [&str; 2] = ["user.xdg.origin.url", "user.xdg.referrer.url"];

    // The `Result` is the shape the other platforms need, not this one: on
    // Linux nothing here can fail, because a note nothing reads is not worth
    // refusing a payload over. Narrowing the signature would make the arms
    // disagree and push the difference into every caller.
    #[allow(clippy::unnecessary_wraps)]
    pub fn carry(from: &Path, to: &Path) -> io::Result<Mark> {
        let mut carried = false;
        for name in ORIGIN {
            // Best effort in both directions. A filesystem that will not hold
            // a `user.` attribute is not an error here, because refusing to
            // open a payload over a note nothing reads would be theatre.
            if let Ok(Some(value)) = xattr::get(from, name) {
                if xattr::set(to, name, &value).is_ok() {
                    carried = true;
                }
            }
        }
        Ok(if carried { Mark::Noted } else { Mark::Silent })
    }

    /// The same question here, and neither answer gates anything: these
    /// attributes are a note, so nothing on this platform consults one before
    /// opening a file and nothing writes one on this application's behalf.
    pub fn carries_a_mark(path: &Path) -> bool {
        arrived_from_elsewhere(path)
    }

    pub fn arrived_from_elsewhere(path: &Path) -> bool {
        ORIGIN
            .iter()
            .any(|name| matches!(xattr::get(path, name), Ok(Some(_))))
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
mod platform {
    use super::{io, Mark, Path};

    pub fn carry(_from: &Path, _to: &Path) -> io::Result<Mark> {
        Ok(Mark::Silent)
    }

    pub fn carries_a_mark(_path: &Path) -> bool {
        false
    }

    pub fn arrived_from_elsewhere(_path: &Path) -> bool {
        false
    }
}

// ---------------------------------------------------------------------------

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{carry, Mark};

    /// A container carrying no provenance leaves the copy carrying none, rather
    /// than inventing one or reporting that something was carried.
    #[test]
    fn a_container_from_nowhere_marks_nothing() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("plain.slpc");
        let to = dir.path().join("payload.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");

        assert_eq!(carry(&from, &to).expect("carrying"), Mark::Silent);
        assert!(xattr::get(&to, "user.xdg.origin.url")
            .expect("reading")
            .is_none());
    }

    /// The defect this catches is the whole point of the module: a payload
    /// extracted from a downloaded container arriving with no record of where
    /// the container came from.
    #[test]
    fn a_downloaded_container_puts_its_origin_on_the_payload() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("downloaded.slpc");
        let to = dir.path().join("payload.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");
        xattr::set(&from, "user.xdg.origin.url", b"https://example.invalid/a.slpc")
            .expect("marking the source");

        assert_eq!(carry(&from, &to).expect("carrying"), Mark::Noted);
        assert_eq!(
            xattr::get(&to, "user.xdg.origin.url").expect("reading"),
            Some(b"https://example.invalid/a.slpc".to_vec()),
        );
    }

    /// Both attributes are carried, not just the first one found. Catches a
    /// loop that returns as soon as it has something.
    #[test]
    fn the_referrer_is_carried_as_well_as_the_origin() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("downloaded.slpc");
        let to = dir.path().join("payload.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");
        xattr::set(&from, "user.xdg.origin.url", b"https://example.invalid/a.slpc")
            .expect("marking the origin");
        xattr::set(&from, "user.xdg.referrer.url", b"https://example.invalid/page")
            .expect("marking the referrer");

        assert_eq!(carry(&from, &to).expect("carrying"), Mark::Noted);
        assert_eq!(
            xattr::get(&to, "user.xdg.referrer.url").expect("reading"),
            Some(b"https://example.invalid/page".to_vec()),
        );
    }

    /// Carrying replaces what the destination already said rather than leaving
    /// a stale origin from whatever wrote that file before.
    #[test]
    fn an_origin_already_on_the_copy_is_replaced() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("downloaded.slpc");
        let to = dir.path().join("payload.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");
        xattr::set(&from, "user.xdg.origin.url", b"https://example.invalid/new")
            .expect("marking the source");
        xattr::set(&to, "user.xdg.origin.url", b"https://example.invalid/stale")
            .expect("marking the destination");

        carry(&from, &to).expect("carrying");
        assert_eq!(
            xattr::get(&to, "user.xdg.origin.url").expect("reading"),
            Some(b"https://example.invalid/new".to_vec()),
        );
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tests {
    use super::{carry, Mark};

    const QUARANTINE: &str = "com.apple.quarantine";
    const FROM_SAFARI: &[u8] = b"0083;6a8dbb61;Safari;B8AC643B-5609-41D4-A666-ACC147704C79";
    const FROM_US: &[u8] = b"0082;6a8dc724;slipcase-desktop;";

    /// A file that will not accept an attribute, so that the write `carry`
    /// attempts fails the way the App Sandbox makes it fail. A test cannot
    /// enter a sandbox; what it can do is deny the same write for a reason of
    /// its own and hold `carry` to the same rule.
    fn unwritable(path: &std::path::Path) {
        let mut mode = std::fs::metadata(path).expect("the file").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o444);
        std::fs::set_permissions(path, mode).expect("making it unwritable");
    }

    /// The defect this catches is Extract and Open failing outright under the
    /// App Sandbox for every container that arrived from elsewhere — the
    /// containers the whole module exists for. Measured 2026-08-25: the
    /// platform marks what a sandboxed process writes and then refuses to have
    /// that mark replaced, so `carry` failed, and `copy_out` turns a failure
    /// here into a refusal to extract at all. A copy that is already marked is
    /// gated, so nothing was laundered and there is nothing to refuse.
    #[test]
    fn a_copy_the_platform_marked_first_is_not_a_failure() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("downloaded.slpc");
        let to = dir.path().join("report.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");
        xattr::set(&from, QUARANTINE, FROM_SAFARI).expect("marking the source");
        xattr::set(&to, QUARANTINE, FROM_US).expect("marking the copy");
        unwritable(&to);

        assert_eq!(
            carry(&from, &to).expect("a marked copy is not a failure"),
            Mark::AlreadyMarked
        );
    }

    /// The defect this catches is the fallback above swallowing a real one. A
    /// copy that carries no mark at all after the write was refused is exactly
    /// the laundering this module exists to prevent, and it must still be an
    /// error — otherwise the payload is handed to its handler looking like
    /// something this machine made.
    #[test]
    fn a_copy_with_no_mark_at_all_is_still_a_failure() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("downloaded.slpc");
        let to = dir.path().join("report.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");
        xattr::set(&from, QUARANTINE, FROM_SAFARI).expect("marking the source");
        unwritable(&to);

        assert!(
            carry(&from, &to).is_err(),
            "an unmarked copy was accepted, which is the laundering this module exists to prevent"
        );
    }

    /// The mark the platform writes on this application's behalf, whose agent
    /// is the running executable's own filename. Built rather than spelled out,
    /// because under `cargo test` the executable is the test binary.
    fn our_own_mark() -> Vec<u8> {
        use std::os::unix::ffi::OsStrExt;
        let us = std::env::current_exe().expect("this process has a path");
        let mut value = b"0082;6a8dc724;".to_vec();
        value.extend_from_slice(us.file_name().expect("and a filename").as_bytes());
        value.push(b';');
        value
    }

    /// The defect this catches is the card telling a person that a container
    /// they made here arrived from elsewhere. Under the App Sandbox the
    /// platform marks whatever this process writes, so saving an edit marks the
    /// container — measured 2026-08-25 — and a predicate that only asks whether
    /// a mark exists then reports a local file as downloaded.
    #[test]
    fn a_mark_this_application_wrote_is_not_provenance() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let saved = dir.path().join("saved-here.slpc");
        std::fs::write(&saved, b"container").expect("the container");
        xattr::set(&saved, QUARANTINE, &our_own_mark()).expect("marking it as we would");

        assert!(
            !super::arrived_from_elsewhere(&saved),
            "a container this application saved is being reported as downloaded"
        );
    }

    /// The defect this catches is the test above going too far and silencing
    /// real provenance. A mark naming any other agent is what the card exists
    /// to report, and disregarding one would be the module lying in the
    /// direction that costs something.
    #[test]
    fn a_mark_anything_else_wrote_still_is() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let downloaded = dir.path().join("downloaded.slpc");
        std::fs::write(&downloaded, b"container").expect("the container");
        xattr::set(&downloaded, QUARANTINE, FROM_SAFARI).expect("marking the source");

        assert!(super::arrived_from_elsewhere(&downloaded));
    }

    /// A value this module cannot read as its own is reported rather than
    /// disregarded. Catches a parser that treats a missing agent field, or any
    /// other shape it did not expect, as evidence the mark is ours — the safe
    /// direction is one line of unnecessary caution, and the other direction is
    /// the laundering this module exists to prevent.
    #[test]
    fn a_mark_that_cannot_be_read_is_reported() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let odd = dir.path().join("odd.slpc");
        std::fs::write(&odd, b"container").expect("the container");
        xattr::set(&odd, QUARANTINE, b"0082").expect("marking it oddly");

        assert!(super::arrived_from_elsewhere(&odd));
    }

    /// The defect this catches is the two questions being one function again.
    /// `carry` needs to know whether the copy is gated, and a copy the platform
    /// marked on this application's behalf is gated even though it did not
    /// arrive from anywhere. Making `carry` ask about origin instead breaks
    /// extraction under a sandbox, which is what the fallback was added to fix.
    #[test]
    fn a_copy_this_application_marked_still_counts_as_gated() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("downloaded.slpc");
        let to = dir.path().join("report.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");
        xattr::set(&from, QUARANTINE, FROM_SAFARI).expect("marking the source");
        xattr::set(&to, QUARANTINE, &our_own_mark()).expect("as the platform would");
        unwritable(&to);

        assert_eq!(
            carry(&from, &to).expect("a marked copy is not a failure"),
            Mark::AlreadyMarked
        );
        assert!(
            !super::arrived_from_elsewhere(&to),
            "and the same file does not claim to have come from anywhere"
        );
    }

    /// A container that arrived from nowhere leaves the copy alone, rather than
    /// inventing a mark or reporting one. The macOS counterpart of the Linux
    /// test of the same name, and it catches an arm that treats "no mark on the
    /// source" as something to carry.
    #[test]
    fn a_container_from_nowhere_marks_nothing() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("plain.slpc");
        let to = dir.path().join("report.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");

        assert_eq!(carry(&from, &to).expect("carrying"), Mark::Silent);
        assert!(xattr::get(&to, QUARANTINE).expect("reading").is_none());
    }

    /// The defect this catches is the whole point of the module on this
    /// platform: a payload extracted from a downloaded container arriving with
    /// no quarantine attribute, so that Gatekeeper is never consulted about it.
    #[test]
    fn a_downloaded_container_puts_its_quarantine_on_the_payload() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let from = dir.path().join("downloaded.slpc");
        let to = dir.path().join("report.pdf");
        std::fs::write(&from, b"container").expect("the container");
        std::fs::write(&to, b"payload").expect("the payload");
        xattr::set(&from, QUARANTINE, FROM_SAFARI).expect("marking the source");

        assert_eq!(carry(&from, &to).expect("carrying"), Mark::Carried);
        assert_eq!(
            xattr::get(&to, QUARANTINE).expect("reading"),
            Some(FROM_SAFARI.to_vec()),
            "the copy does not carry the value the container carried"
        );
    }
}
