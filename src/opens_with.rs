//! What the platform says would open a payload.
//!
//! One function per platform returning an optional string, and no table of
//! filenames mapped to types anywhere in this application. DESIGN.md §3.
//!
//! The question is asked from the payload's name alone, before anything has
//! been extracted. Where the platform will not answer from a name, the answer
//! is `None` and the card says nothing rather than guessing.

/// The name of the application the platform would open this payload with.
#[cfg(target_os = "linux")]
#[must_use]
pub fn opens_with(payload_name: &str) -> Option<String> {
    let mime = linux::mime_of(payload_name)?;
    let entry = linux::default_application(&mime)?;
    linux::display_name(&entry)
}

/// The name of the application the platform would open this payload with.
#[cfg(target_os = "macos")]
#[must_use]
pub fn opens_with(payload_name: &str) -> Option<String> {
    let extension = macos::extension_of(payload_name)?;
    let application = macos::application_for(extension)?;
    macos::display_name(&application)
}

/// Not yet asked on this platform, so the card says nothing.
///
/// Windows answers through `AssocQueryString` from an extension alone, and it
/// was not testable on the machine this was written on. `HANDOFF-windows.md`
/// is the brief for the platform that can.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[must_use]
pub fn opens_with(_payload_name: &str) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
mod linux {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    /// The media type `xdg-mime` gives a file of this name.
    ///
    /// `xdg-mime` needs a file that is there, and an empty one reports as
    /// `application/x-zerosize` whatever it is called, so the question is asked
    /// of two placeholders carrying the payload's name: one whose bytes sniff
    /// as text and one whose bytes sniff as binary. Where the name matched a
    /// glob, the glob wins over both and the answers agree. Where it matched
    /// nothing, each answer is the content of the placeholder rather than
    /// anything about the payload, the two differ, and there is nothing to say.
    pub fn mime_of(payload_name: &str) -> Option<String> {
        // A conformant container's payload name has been through
        // `slpc::check_payload_name`, which rejects every separator, so this
        // joins a plain filename. Checked again rather than assumed.
        if payload_name.is_empty() || payload_name.contains(['/', '\\']) {
            return None;
        }

        let dir = probe_dir()?;
        let text = write_probe(&dir, "text", payload_name, b" ");
        let binary = write_probe(&dir, "binary", payload_name, &[0u8; 4]);

        let answer = match (text, binary) {
            (Some(t), Some(b)) => {
                let (t, b) = (query_filetype(&t)?, query_filetype(&b)?);
                if t == b {
                    Some(t)
                } else {
                    None
                }
            }
            _ => None,
        };

        // The probes are this function's own litter and outlive nothing.
        let _ = std::fs::remove_dir_all(&dir);
        answer
    }

    /// A directory of this process's own to put the probes in.
    fn probe_dir() -> Option<PathBuf> {
        let dir = std::env::temp_dir().join(format!("slipcase-desktop-probe-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok()?;
        Some(dir)
    }

    /// One probe: the payload's name, under a directory of its own so that two
    /// probes can share the name that is the whole question.
    fn write_probe(dir: &Path, which: &str, name: &str, bytes: &[u8]) -> Option<PathBuf> {
        let sub = dir.join(which);
        std::fs::create_dir_all(&sub).ok()?;
        let path = sub.join(name);
        std::fs::write(&path, bytes).ok()?;
        Some(path)
    }

    /// `xdg-mime query filetype`, or nothing where it is not installed.
    fn query_filetype(path: &Path) -> Option<String> {
        run(&["query", "filetype", path.to_str()?])
    }

    /// The desktop entry `xdg-mime` names as the default for a media type.
    pub fn default_application(mime: &str) -> Option<String> {
        run(&["query", "default", mime])
    }

    /// `xdg-mime`, when it is there and has something to say.
    fn run(args: &[&str]) -> Option<String> {
        let out = Command::new("xdg-mime").args(args).output().ok()?;
        if !out.status.success() {
            return None;
        }
        let said = String::from_utf8(out.stdout).ok()?.trim().to_owned();
        if said.is_empty() {
            None
        } else {
            Some(said)
        }
    }

    /// The `Name` a desktop entry gives itself.
    ///
    /// A person recognises Document Viewer; nobody recognises
    /// `org.gnome.Evince.desktop`. Where the entry cannot be found or gives no
    /// name, the file name is not shown in its place: DESIGN.md §3 says nothing
    /// rather than something the platform did not say.
    pub fn display_name(entry: &str) -> Option<String> {
        if entry.contains(['/', '\\']) {
            return None;
        }
        for dir in data_dirs() {
            let path = dir.join("applications").join(entry);
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Some(name) = name_in(&text) {
                return Some(name);
            }
        }
        None
    }

    /// The unlocalised `Name` of the `[Desktop Entry]` group.
    fn name_in(text: &str) -> Option<String> {
        let mut in_entry = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_entry = line == "[Desktop Entry]";
                continue;
            }
            // `Name[de]` is somebody else's language. Only the bare key.
            if in_entry {
                if let Some(v) = line.strip_prefix("Name=") {
                    let v = v.trim();
                    if !v.is_empty() {
                        return Some(v.to_owned());
                    }
                }
            }
        }
        None
    }

    /// Where desktop entries live, in the order the XDG basedir specification
    /// says to search them.
    fn data_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(home) = std::env::var_os("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(home));
        } else if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share"));
        }
        let rest = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_owned());
        dirs.extend(rest.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
        dirs
    }

    #[cfg(test)]
    mod tests {
        use super::name_in;

        /// `Name[de]` is somebody else's language, and a `Name` under an action
        /// group is the action's rather than the application's.
        #[test]
        fn the_unlocalised_name_of_the_entry_group_wins() {
            let entry = "\
[Desktop Entry]
Type=Application
Name[de]=Dokumentenbetrachter
Name=Document Viewer
Exec=evince %U

[Desktop Action NewWindow]
Name=New Window
";
            assert_eq!(name_in(entry).as_deref(), Some("Document Viewer"));
        }

        /// An entry with no name of its own gives nothing, and the card then
        /// says nothing rather than showing the file name of the entry.
        #[test]
        fn an_entry_without_a_name_gives_nothing() {
            assert_eq!(name_in("[Desktop Entry]\nType=Application\n"), None);
        }
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::rc::Retained;
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSBundle, NSString, NSURL};
    use objc2_uniform_type_identifiers::UTType;

    /// The filename extension the question is asked of.
    ///
    /// Launch Services takes an extension rather than a whole name, so unlike
    /// Linux this needs no file on disk and no placeholder: the question can be
    /// asked of a payload that has never been extracted, which is what
    /// DESIGN.md §3 wants and what the Linux arm has to work around.
    ///
    /// A name with no extension, a name that is nothing but an extension, and a
    /// name ending in a bare dot all give nothing. A separator is refused for
    /// the same reason the Linux arm refuses one: `slpc::check_payload_name`
    /// rejects every separator, so a name carrying one is not a payload's, and
    /// answering for it would be answering about a path this application was
    /// never given.
    pub fn extension_of(payload_name: &str) -> Option<&str> {
        if payload_name.contains(['/', '\\']) {
            return None;
        }
        let extension = std::path::Path::new(payload_name).extension()?.to_str()?;
        if extension.is_empty() {
            None
        } else {
            Some(extension)
        }
    }

    /// The application Launch Services would open a file of this extension
    /// with, asked through the type the extension names.
    ///
    /// Where no type is declared for an extension, macOS synthesises a dynamic
    /// one — `dyn.ah62d4rv4ge81g5duqq` for `slpc` before this application is
    /// registered — rather than answering nothing. Nothing claims a synthesised
    /// type, so Launch Services then names no application and the answer is
    /// `None` without this needing to inspect the type: measured against
    /// `slpc`, `qqzz`, and the extensions in the example, a dynamic type never
    /// reached an application. A declared type reaching no application is the
    /// same `None` and just as honest — `xlsx` is declared on a machine with
    /// nothing installed that opens it.
    pub fn application_for(extension: &str) -> Option<Retained<NSURL>> {
        let content_type = UTType::typeWithFilenameExtension(&NSString::from_str(extension))?;
        NSWorkspace::sharedWorkspace().URLForApplicationToOpenContentType(&content_type)
    }

    /// The name the application bundle gives itself.
    ///
    /// A person recognises Preview; nobody recognises `com.apple.Preview` or
    /// `/System/Applications/Preview.app`. This reads the bundle rather than
    /// asking `NSFileManager` for a display name, because the display name
    /// follows the Finder preference for showing every filename extension and
    /// becomes `Preview.app` for a person who has turned it on. What the bundle
    /// calls itself does not move.
    ///
    /// `CFBundleDisplayName` first and `CFBundleName` after it: the first is
    /// what most applications carry, and `DiskImageMounter`, which opens a `dmg`,
    /// carries only the second. Where the bundle gives neither, nothing is
    /// shown rather than the path or the identifier — DESIGN.md §3 again.
    pub fn display_name(application: &NSURL) -> Option<String> {
        let bundle = NSBundle::bundleWithURL(application)?;
        for key in ["CFBundleDisplayName", "CFBundleName"] {
            let value = bundle.objectForInfoDictionaryKey(&NSString::from_str(key));
            let Some(name) = value.and_then(|v| v.downcast::<NSString>().ok()) else {
                continue;
            };
            let name = name.to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
        None
    }

    #[cfg(test)]
    mod tests {
        use super::extension_of;

        /// A name that is all name gives nothing, rather than the whole of it
        /// being handed to Launch Services as though it were an extension.
        #[test]
        fn a_name_without_an_extension_gives_nothing() {
            assert_eq!(extension_of("README"), None);
        }

        /// A leading dot makes a hidden file, not an extension, and asking
        /// about `bash_profile` would be asking about the wrong thing.
        #[test]
        fn a_leading_dot_is_not_an_extension() {
            assert_eq!(extension_of(".bash_profile"), None);
        }

        /// A name ending in a bare dot has an extension of no characters, and
        /// `UTType` answers nothing for it. Refused here so that the question
        /// is never asked rather than asked and discarded.
        #[test]
        fn a_trailing_dot_gives_nothing() {
            assert_eq!(extension_of("archive."), None);
        }

        /// The last extension is the one the platform answers for: macOS opens
        /// `a.tar.gz` with what claims `gz`, and answering for `tar` would name
        /// the wrong application.
        #[test]
        fn the_last_extension_of_a_compound_name_wins() {
            assert_eq!(extension_of("a.tar.gz"), Some("gz"));
        }

        /// A separator means this is not a payload's name — `check_payload_name`
        /// rejects every one — and taking the extension from it would answer
        /// about a path this application was never given.
        #[test]
        fn a_name_carrying_a_separator_is_refused() {
            assert_eq!(extension_of("/etc/passwd.pdf"), None);
            assert_eq!(extension_of("dir\\report.pdf"), None);
        }

        /// The extension is passed on as it was written. Launch Services is
        /// case-insensitive — `REPORT.PDF` and `report.pdf` both answer
        /// Preview — so folding the case here would be work that changes no
        /// answer.
        #[test]
        fn the_case_of_an_extension_is_left_alone() {
            assert_eq!(extension_of("REPORT.PDF"), Some("PDF"));
        }
    }
}
