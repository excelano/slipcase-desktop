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
//! and it could not be written to the copy. Everything else — no mark, no such
//! mark on this platform, a note nothing enforces — succeeds. So the rule for a
//! caller about to hand a payload to the system is the whole of the rule: an
//! error means do not open it.

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
    /// The source said nothing about where it came from, or this platform keeps
    /// nothing that would say.
    Silent,
}

/// Carry whatever the platform records about `from` onto `to`.
///
/// # Errors
///
/// Returns the write error when this platform gates opening on a mark, `from`
/// carries one, and it could not be put on `to`. A caller that is about to open
/// `to` must not, because the copy would be trusted where the original was not.
pub fn carry(from: &Path, to: &Path) -> io::Result<Mark> {
    platform::carry(from, to)
}

/// Whether the platform records this file as having arrived from elsewhere.
///
/// The card says so, because a person deciding whether to open a payload is
/// better served by knowing where the container came from than by being stopped
/// from opening it. What the platform will then do about the mark is the
/// platform's business — DESIGN.md §3's rule, applied to provenance rather than
/// to type — so this reports and does not gate.
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

    pub fn arrived_from_elsewhere(path: &Path) -> bool {
        matches!(xattr::get(path, QUARANTINE), Ok(Some(_)))
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
