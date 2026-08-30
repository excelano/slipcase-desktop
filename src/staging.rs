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
/// Measured under the sandbox; `git log` holds the run.
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
            let landing = macos::Landing::reserved_for(path)?;
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

            // What the platform records about where the container came from,
            // put on the rewrite before it becomes the container. The other two
            // platforms get this from `Destination::in_place`, which carries it
            // across the rename it does; this arm does not use `in_place`, so
            // without the line below saving an edit to a downloaded container
            // would strip its `com.apple.quarantine` — the defect measured on
            // Linux on 2026-08-27 and fixed in `slpc` 0.3.7, arriving here by
            // the one door that fix does not reach.
            //
            // Done here rather than trusted to `replaceItemAtURL:`, which is
            // documented to preserve the original item's metadata and may well
            // preserve this too. May well is not measured, this is the
            // attribute measured as the difference between an unsigned
            // application from the internet being stopped and running,
            // and doing it twice costs a `Mark::AlreadyMarked` and nothing else.
            //
            // Not fatal. `carry` refuses when the copy would be ungated where
            // the original was gated, and that rule is written for a payload
            // about to be handed to the system. This is a container, and the
            // thing that opens a container is this application, which reports
            // provenance rather than acting on it — so a save that has already
            // been validated is not thrown away over it.
            if let Err(e) = slpc::provenance::carry(self.landing.original(), self.landing.staged())
            {
                eprintln!(
                    "slipcase-desktop: where this container came from could not be carried onto the rewrite: {e}"
                );
            }
            self.landing.replace_original()
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::path::{Path, PathBuf};

    use objc2_foundation::{
        NSFileManager, NSFileManagerItemReplacementOptions, NSSearchPathDirectory,
        NSSearchPathDomainMask, NSString, NSURL,
    };

    /// A scratch directory holding the rewrite, and the file it will become.
    pub(crate) struct Landing {
        /// Held so the directory outlives the file inside it, and is removed
        /// after the replacement has moved that file out.
        scratch: Scratch,
        staged: PathBuf,
        original: PathBuf,
    }

    impl Landing {
        /// Reserve a rewrite of `original`, somewhere the replacement can reach
        /// it from and nowhere near it.
        ///
        /// Those two are not the same requirement and the first was learned the
        /// expensive way. `tempfile::TempDir::new` used to choose this, which
        /// satisfies the second — the sandbox refuses a sibling of the
        /// container, which is the whole reason this module exists — and
        /// quietly fails the first. `TempDir` answers `TMPDIR`, which is on the
        /// boot volume, and measured 2026-08-25 a replacement whose two ends
        /// are on different volumes stops with `NSCocoaErrorDomain` 512 on APFS,
        /// FAT32 and exFAT alike. Save did not work for any container on an
        /// external drive, a mounted image, or a share. Measured on all three;
        /// `git log` holds the run.
        ///
        /// `NSItemReplacementDirectory` is what Apple provides for exactly this:
        /// asked with `appropriateForURL:`, it makes a fresh directory on the
        /// volume that URL is on. For a container on the boot volume it returns
        /// one under the same per-user temporary area `TempDir` was using, so
        /// the sandbox story is unchanged and the property
        /// `nothing_waits_beside_the_container` asserts still holds; for one on
        /// a second volume it returns a directory there and the replacement
        /// succeeds.
        ///
        /// There is deliberately no fallback to `TempDir` if this fails. The
        /// only things that would fail it are a volume that cannot be written,
        /// which the replacement could not have finished on either, and the
        /// path that fallback would take is the one now known not to work.
        ///
        /// The path is resolved first, for the reason `Destination::in_place`
        /// resolves it: a container reached through a symbolic link should have
        /// the container replaced and not the link. It also has to be resolved
        /// before it is asked about, since the volume that matters is the one
        /// the container is on rather than the one the link is on.
        pub(crate) fn reserved_for(original: &Path) -> std::io::Result<Self> {
            let original = std::fs::canonicalize(original)?;
            let scratch = Scratch::on_the_volume_holding(&original)?;
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

        /// The container being replaced, for reading what the platform records
        /// about where it came from.
        pub(crate) fn original(&self) -> &Path {
            &self.original
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
                        "cannot replace {}: {}{}",
                        self.original.display(),
                        e.localizedDescription(),
                        because_of(&e)
                    ))
                })?;

            // After the replacement rather than before, which is the whole
            // reason this is a line and not left to fall off the end of the
            // function: the staged file lives in there until the call above has
            // moved it out. It used to be `TempDir::close`, reported rather than
            // swallowed so that a failure to clean up could not hide behind a
            // save that worked. That stopped being worth doing when the
            // directory stopped being ours — macOS made it, macOS empties
            // `.TemporaryItems`, and a replacement that succeeded is not a save
            // to fail over a directory that did not go.
            drop(self.scratch);
            Ok(())
        }
    }

    /// The directory macOS made for one replacement, removed when it is over.
    ///
    /// `tempfile::TempDir` was what held this and cannot be any more, because
    /// where the rewrite waits is no longer this application's choice — see
    /// `Landing::reserved_for`. What `TempDir` was doing for free is the drop,
    /// so that is what this is.
    struct Scratch(PathBuf);

    impl Scratch {
        /// Ask macOS for a directory a file can be replaced *from*.
        ///
        /// `NSUserDomainMask` is not a choice: `NSItemReplacementDirectory` is
        /// documented to take that one and `appropriateForURL:` is what
        /// actually decides where the directory lands.
        fn on_the_volume_holding(original: &Path) -> std::io::Result<Self> {
            let url = NSFileManager::defaultManager()
                .URLForDirectory_inDomain_appropriateForURL_create_error(
                    NSSearchPathDirectory::ItemReplacementDirectory,
                    NSSearchPathDomainMask::UserDomainMask,
                    Some(&url_for(original)),
                    true,
                )
                .map_err(|e| {
                    std::io::Error::other(format!(
                        "nowhere to rewrite {}: {}",
                        original.display(),
                        e.localizedDescription()
                    ))
                })?;
            // A file URL always has a path; the `Option` is for the ones that
            // do not, and this call cannot return one of those.
            let path = url.path().ok_or_else(|| {
                std::io::Error::other("macOS named a replacement directory with no path")
            })?;
            Ok(Self(PathBuf::from(path.to_string())))
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            // The whole tree rather than the directory alone: a successful
            // replacement has moved the staged file out and leaves this empty,
            // and a failed one leaves the file sitting in it.
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Whatever the failure was underneath Cocoa's sentence, if it said.
    ///
    /// This exists because `localizedDescription` is written for a person
    /// looking at a dialog and hides the only fact worth having. The
    /// cross-volume failure this module was rewritten to fix reports *The file
    /// “x.slpc” couldn’t be saved in the folder “y”* and nothing else, and the
    /// `EXDEV` under it — which names the defect outright — took a probe to
    /// reach. Appended rather than substituted: the sentence is still the one a
    /// person can act on, and the errno is for whoever reads the message after
    /// them.
    fn because_of(error: &objc2_foundation::NSError) -> String {
        use std::fmt::Write;
        error.underlyingErrors().iter().fold(String::new(), |mut so_far, under| {
            let _ = write!(so_far, " ({} {})", under.domain(), under.localizedDescription());
            so_far
        })
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
    /// back. That had been read out of Apple's documentation rather than
    /// measured: the extended attributes were measured on 2026-08-25 and the
    /// mode bits were not. This test is
    /// where they are, and the Apple silicon workflow is where it runs — a mode
    /// bit needs no display, no session and no person, so the one platform
    /// nobody here can execute answers it anyway. It passes: the replacement
    /// keeps the original's permissions, as the documentation said it would.
    ///
    /// It bites on both routes. Swapping the non-macOS arm to
    /// `Destination::new` leaves a container the person made `0600` sitting at
    /// `0664`, the umask's answer rather than the container's, which is how this
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

    /// The defect this catches is Save doing nothing for a container that is
    /// not on the boot volume.
    ///
    /// `replaceItemAtURL:` is documented in terms of the item it replaces and
    /// says nothing about where the replacement may come from. It wants both
    /// ends on one volume: measured 2026-08-25, staging on the boot volume and
    /// replacing onto a mounted image fails with `NSCocoaErrorDomain` 512 over
    /// `NSPOSIXErrorDomain` 18, `EXDEV`, on APFS, HFS+, FAT32 and exFAT alike,
    /// leaving the original untouched. Nothing was corrupted and the error did
    /// reach the person, so this is the quiet kind: an external drive, a
    /// mounted image or a share, and every save refuses. It was found by review
    /// from another machine rather than by anything here, which is why it is
    /// worth a test that owns a second volume rather than a note.
    ///
    /// It bites on the change that fixed it. Putting `tempfile::TempDir::new`
    /// back in `Scratch::on_the_volume_holding` stages on the boot volume again
    /// and this fails at the commit, which is how it was checked.
    ///
    /// The two assertions before the work are not ceremony. The first draft of
    /// this test read the mount point out of the wrong field of `hdiutil`'s
    /// output — its first line names the device and leaves the mount point
    /// empty — so every run wrote into the working directory, crossed nothing,
    /// and passed against the defect it was written for. `same_file_system`
    /// is what makes that failure loud rather than green.
    ///
    /// Every macOS machine has `hdiutil`, so there is nothing here to skip
    /// quietly over: a failure is a real one.
    #[cfg(target_os = "macos")]
    #[test]
    fn a_container_on_another_volume_can_be_rewritten() {
        use std::io::Write;
        use std::os::unix::fs::MetadataExt;

        /// Detaches on the way out, including out of a panic, so that a failure
        /// in the middle of this test does not leave a volume mounted.
        ///
        /// By device rather than by mount point, because the mount point is the
        /// thing that can be misread and a `detach ""` is a silent no-op that
        /// leaves the volume attached — measured, eleven of them, while getting
        /// this test wrong.
        struct Mounted(String);
        impl Drop for Mounted {
            fn drop(&mut self) {
                let _ = std::process::Command::new("hdiutil")
                    .args(["detach", "-quiet", &self.0])
                    .status();
            }
        }

        let dir = tempfile::TempDir::new().expect("a directory for the image");
        let image = dir.path().join("second-volume.dmg");
        let made = std::process::Command::new("hdiutil")
            .args(["create", "-size", "20m", "-fs", "APFS", "-volname", "SlipcaseTest", "-quiet"])
            .arg(&image)
            .status()
            .expect("hdiutil create runs");
        assert!(made.success(), "could not make a disk image to test against");

        let attached = std::process::Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-noverify"])
            .arg(&image)
            .output()
            .expect("hdiutil attach runs");
        assert!(
            attached.status.success(),
            "could not mount the disk image: {}",
            String::from_utf8_lossy(&attached.stderr)
        );
        // Both fields off the one line that has a mount point. The volume name
        // is read back rather than assumed, since a volume of this name already
        // mounted makes macOS pick another.
        let listing = String::from_utf8_lossy(&attached.stdout);
        let (device, mount) = listing
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let device = fields.next()?.trim();
                let mount = fields.nth(1)?.trim();
                (!mount.is_empty()).then(|| (device.to_owned(), mount.to_owned()))
            })
            .next_back()
            .expect("hdiutil said where it mounted the image");
        let _mounted = Mounted(device);

        let container = std::path::Path::new(&mount).join("elsewhere.slpc");
        std::fs::write(&container, b"not a container, and nothing reads it here")
            .expect("writes the container onto the second volume");

        let same_file_system = |a: &std::path::Path, b: &std::path::Path| {
            std::fs::metadata(a).expect("stats").dev() == std::fs::metadata(b).expect("stats").dev()
        };
        assert!(
            !same_file_system(&container, std::path::Path::new(".")),
            "{} is on the same volume as the working directory, so this test crosses nothing",
            container.display()
        );

        let mut staged = Staged::over(&container).expect("reserves a rewrite");
        assert!(
            // The directory, not the reserved name: `Destination::new` writes
            // through a temporary file and only takes that name at commit, so
            // there is nothing to stat there yet.
            !same_file_system(
                staged.landing.staged().parent().expect("the rewrite waits in a directory"),
                &std::env::temp_dir(),
            ),
            "the rewrite is waiting in the boot volume's temporary directory, \
             which is the arrangement this test exists to refuse"
        );

        staged.writer().write_all(b"rewritten").expect("writes");
        staged.commit().expect("the replacement lands on the other volume");

        assert_eq!(
            std::fs::read(&container).expect("reads the container back"),
            b"rewritten",
            "the container on the second volume still holds what it did"
        );
    }
}
