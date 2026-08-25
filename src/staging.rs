//! Where a rewritten container waits, and how it lands on the original.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

use std::fs::File;
use std::path::Path;

/// A container being rewritten, and the original it will replace.
///
/// Two platforms let `slpc` decide where the rewrite waits, which is beside the
/// container. macOS cannot, and the reason is measured rather than reasoned
/// about: under the App Sandbox a person's grant covers the file they chose
/// through the open panel and not the directory holding it, so
/// `Destination::in_place` — which asks `NamedTempFile` for a randomly-named
/// sibling — stops with *Operation not permitted* before a byte is written.
/// `CHECKLIST.md` holds the run.
///
/// This is not a way around the library. Both arms below use `slpc` for the
/// whole of what it does: reserving a file, handing back a writer, flushing and
/// rewinding it to be read, and putting it where it belongs. They differ only
/// in which public constructor they ask for and, on macOS, in one further move
/// the library has no business knowing about.
pub(crate) struct Staged {
    destination: slpc::Destination,
    #[cfg(target_os = "macos")]
    landing: macos::Landing,
}

impl Staged {
    /// Reserve somewhere to rewrite `path`, which the result will replace.
    ///
    /// # Errors
    ///
    /// Whatever the library says about reserving a file, and on macOS whatever
    /// the filesystem says about a scratch directory.
    pub(crate) fn over(path: &Path) -> slpc::Result<Self> {
        #[cfg(not(target_os = "macos"))]
        {
            Ok(Self {
                destination: slpc::Destination::in_place(path)?,
            })
        }
        #[cfg(target_os = "macos")]
        {
            let landing = macos::Landing::beside_nothing(path)?;
            Ok(Self {
                destination: slpc::Destination::new(landing.staged(), false)?,
                landing,
            })
        }
    }

    /// Where to write.
    pub(crate) fn writer(&mut self) -> &mut File {
        self.destination.writer()
    }

    /// What has been written so far, flushed and rewound to be read back.
    ///
    /// # Errors
    ///
    /// Whatever the library says about flushing and rewinding.
    pub(crate) fn written(&mut self) -> slpc::Result<&mut File> {
        self.destination.written()
    }

    /// Put the rewritten container where the original is.
    ///
    /// # Errors
    ///
    /// Whatever the library says about committing, and on macOS whatever the
    /// platform says about the replacement.
    pub(crate) fn commit(self) -> slpc::Result<()> {
        #[cfg(not(target_os = "macos"))]
        {
            self.destination.commit()
        }
        #[cfg(target_os = "macos")]
        {
            // The library's own commit first, which renames the temporary file
            // to its reserved name inside the scratch directory. Both are ours,
            // so nothing about that rename is a sandbox's business.
            self.destination.commit()?;
            self.landing.replace_original()
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::{Path, PathBuf};

    use objc2_foundation::{
        NSFileManager, NSFileManagerItemReplacementOptions, NSString, NSURL,
    };

    /// A scratch directory holding the rewrite, and the file it will become.
    pub(crate) struct Landing {
        /// Held so the directory outlives the file inside it, and is removed
        /// after the replacement has moved that file out.
        scratch: tempfile::TempDir,
        staged: PathBuf,
        original: PathBuf,
    }

    impl Landing {
        /// Reserve a rewrite of `original` that is nowhere near it.
        ///
        /// The path is resolved first, for the reason `Destination::in_place`
        /// resolves it: a container reached through a symbolic link should have
        /// the container replaced and not the link.
        pub(crate) fn beside_nothing(original: &Path) -> std::io::Result<Self> {
            let original = std::fs::canonicalize(original)?;
            let scratch = tempfile::TempDir::new()?;
            // The same name it will have again, so that anything reading the
            // staged file — the validation this exists to allow — sees a
            // container named the way containers are named.
            let name = original
                .file_name()
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("{} names no file", original.display()),
                    )
                })?
                .to_owned();
            let staged = scratch.path().join(name);
            Ok(Self {
                scratch,
                staged,
                original,
            })
        }

        pub(crate) fn staged(&self) -> &Path {
            &self.staged
        }

        /// Move the rewrite onto the original.
        ///
        /// `-[NSFileManager replaceItemAtURL:…]` rather than a rename, and the
        /// difference is the whole point of this module. A rename into the
        /// container's directory is a new name in a directory a sandboxed
        /// application has no grant on; this call is the one Apple sanctions
        /// for replacing a file a person chose, and it needs no unsafe — the
        /// binding is a safe function and `objc2-foundation` was already here
        /// for `NSBundle` and the Apple Event.
        ///
        /// The original's metadata is what survives, which is what the
        /// `in_place` this replaced also promised: permissions come from the
        /// file being replaced rather than from the umask. Passing
        /// `NSFileManagerItemReplacementUsingNewMetadataOnly` would be how to
        /// ask for the other behaviour, and it is not passed.
        pub(crate) fn replace_original(self) -> slpc::Result<()> {
            let original = url_for(&self.original);
            let staged = url_for(&self.staged);

            NSFileManager::defaultManager()
                .replaceItemAtURL_withItemAtURL_backupItemName_options_resultingItemURL_error(
                    &original,
                    &staged,
                    None,
                    NSFileManagerItemReplacementOptions::empty(),
                    None,
                )
                .map_err(|e| {
                    std::io::Error::other(format!(
                        "cannot replace {}: {}",
                        self.original.display(),
                        e.localizedDescription()
                    ))
                })?;

            // Explicit rather than left to the drop, so that a failure to clean
            // up is not silently swallowed while the replacement is reported as
            // having succeeded. The staged file has been moved out by now, so
            // this is an empty directory.
            self.scratch.close()?;
            Ok(())
        }
    }

    fn url_for(path: &Path) -> objc2::rc::Retained<NSURL> {
        NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()))
    }
}

#[cfg(test)]
mod tests {
    use super::Staged;

    /// The defect this catches is the rewrite waiting beside the container.
    ///
    /// On macOS that is what makes Save fail under the App Sandbox: the grant a
    /// person gives through the open panel covers the file they chose and not
    /// the directory holding it, so creating a randomly-named sibling stops
    /// with *Operation not permitted*, measured 2026-08-25 and recorded in
    /// `CHECKLIST.md`. A test cannot enter a sandbox, so it asserts the
    /// property that made the sandbox refuse: after reserving a rewrite, the
    /// container's own directory holds nothing but the container. Reverting
    /// this arm to `Destination::in_place` puts a second entry there and fails
    /// this test, which is how it was checked.
    #[cfg(target_os = "macos")]
    #[test]
    fn nothing_waits_beside_the_container() {
        let dir = tempfile::TempDir::new().expect("a directory to work in");
        let container = dir.path().join("some.slpc");
        std::fs::write(&container, b"not a container, and nothing reads it here")
            .expect("writes the container");

        let staged = Staged::over(&container).expect("reserves a rewrite");

        let beside: Vec<_> = std::fs::read_dir(dir.path())
            .expect("reads the directory")
            .map(|e| e.expect("an entry").file_name())
            .collect();
        assert_eq!(
            beside,
            vec![std::ffi::OsString::from("some.slpc")],
            "the rewrite is waiting beside the container, where a sandbox cannot create it"
        );
        drop(staged);
    }

    /// The defect this catches is a rewrite that never lands.
    ///
    /// Every platform's arm has to end with the original holding what was
    /// written, and macOS reaches that through a different call than the other
    /// two — `replaceItemAtURL:` rather than a rename. Breaking that call to
    /// return without moving anything leaves the original untouched and fails
    /// here.
    #[test]
    fn what_was_written_ends_up_in_the_original() {
        use std::io::Write;

        let dir = tempfile::TempDir::new().expect("a directory to work in");
        let container = dir.path().join("some.slpc");
        std::fs::write(&container, b"before").expect("writes the container");

        let mut staged = Staged::over(&container).expect("reserves a rewrite");
        staged.writer().write_all(b"after").expect("writes");
        staged.commit().expect("commits");

        assert_eq!(
            std::fs::read(&container).expect("reads the container back"),
            b"after",
            "the original does not hold what was written"
        );
    }

    /// The defect this catches is a container coming back readable by people it
    /// was not readable by before.
    ///
    /// The two arms reach this property by different routes, which is why it is
    /// worth asserting rather than assuming. `Destination::in_place` carries the
    /// original's permissions onto the replacement and documents that it does;
    /// `Destination::new`, which the macOS arm asks for instead, deliberately
    /// does not and hands back whatever the umask would have given a new file.
    /// So macOS depends on `replaceItemAtURL:` putting the original's metadata
    /// back, and that was read out of Apple's documentation rather than
    /// measured — the extended attributes were measured on 2026-08-25 and
    /// recorded in `CHECKLIST.md`, and the mode bits were not.
    ///
    /// It bites on both routes. Swapping the non-macOS arm to
    /// `Destination::new` leaves a `0600` container `0644` here, which is how it
    /// was checked; on macOS the same failure is what a replacement taking its
    /// metadata from the staged file would produce.
    #[cfg(unix)]
    #[test]
    fn the_container_keeps_the_permissions_it_had() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().expect("a directory to work in");
        let container = dir.path().join("private.slpc");
        std::fs::write(&container, b"before").expect("writes the container");
        // No umask hands out 0600 for a new file, so a replacement that took
        // the umask's answer cannot pass this by coincidence.
        std::fs::set_permissions(&container, std::fs::Permissions::from_mode(0o600))
            .expect("makes the container private");

        let mut staged = Staged::over(&container).expect("reserves a rewrite");
        staged.writer().write_all(b"after").expect("writes");
        staged.commit().expect("commits");

        assert_eq!(
            std::fs::metadata(&container)
                .expect("reads the container back")
                .permissions()
                .mode()
                & 0o777,
            0o600,
            "the rewrite did not keep the container's own permissions"
        );
    }
}
