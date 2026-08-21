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

/// Not yet asked on this platform, so the card says nothing.
///
/// macOS answers through Launch Services and Windows through
/// `AssocQueryString`, both from an extension alone and neither testable on the
/// machine this was written on. Slice 11 has the platforms to try them on.
#[cfg(not(target_os = "linux"))]
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
