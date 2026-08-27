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

/// The name of the application the platform would open this payload with.
#[cfg(target_os = "windows")]
#[must_use]
pub fn opens_with(payload_name: &str) -> Option<String> {
    let extension = windows::extension_of(payload_name)?;
    let progid = windows::progid_for(&extension)?;
    windows::display_name(&progid)
}

/// Not asked on this platform, so the card says nothing.
///
/// The three platforms this application is built for have an arm each above.
/// Anything else that compiles reaches this one, and DESIGN.md §3 permits the
/// card to say nothing where the platform will not answer.
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
#[must_use]
pub fn opens_with(_payload_name: &str) -> Option<String> {
    None
}

#[cfg(target_os = "linux")]
mod linux {
    use std::os::unix::fs::PermissionsExt as _;
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
        let text = write_probe(dir.path(), "text", payload_name, b" ");
        let binary = write_probe(dir.path(), "binary", payload_name, &[0u8; 4]);

        match (text, binary) {
            (Some(t), Some(b)) => {
                let (t, b) = (query_filetype(&t)?, query_filetype(&b)?);
                if t == b {
                    Some(t)
                } else {
                    None
                }
            }
            _ => None,
        }
        // `dir` drops here and takes the probes with it. Returning early above
        // is why it has to be a `TempDir` rather than a path removed at the
        // end: the previous version's `remove_dir_all` was skipped by every `?`
        // in this function, so a probe carrying the payload's name outlived the
        // question on any container whose type the platform would not name.
    }

    /// A directory of this process's own to put the probes in.
    ///
    /// **A private directory, not a predictable one, and this is the second
    /// time that distinction has cost something here.** It used to be
    /// `$TMPDIR/slipcase-desktop-probe-<pid>`, and every part of that was
    /// wrong. `/tmp` is world-writable, a process id is guessable and reused,
    /// `create_dir_all` succeeds on a path that already exists — including a
    /// symbolic link to somewhere else — and `fs::write` follows one. So
    /// anybody with an account on the machine could plant that path, and
    /// opening a container would then create or truncate a file of the
    /// container's choosing in a directory of theirs: measured 2026-08-27, a 35
    /// byte `authorized_keys` became four NUL bytes, with no user action beyond
    /// opening the container, because this runs from `Opened::open` at startup.
    ///
    /// The mode matters as much as the name. `tempfile`'s directories go
    /// through the umask — 0755 under the common one — so the probes, which
    /// carry the payload's filename, were readable by every account on the
    /// machine. `Cargo.toml` already argued all of this about the *other*
    /// scratch directory and this code did not get the message.
    fn probe_dir() -> Option<tempfile::TempDir> {
        tempfile::Builder::new()
            .prefix("slipcase-probe-")
            .permissions(std::fs::Permissions::from_mode(0o700))
            .tempdir()
            .ok()
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

#[cfg(target_os = "windows")]
mod windows {
    use windows_registry::{Key, CLASSES_ROOT, CURRENT_USER};

    /// Where a per-user choice of application is recorded, one subkey per
    /// extension.
    ///
    /// It beats the machine-wide association under `HKCR`, and reading only
    /// the machine-wide key is the defect that hides here: on the machine this
    /// was written on `.txt` is `txtfile` machine-wide and something else
    /// entirely per user.
    const USER_CHOICE: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\FileExts";

    /// The shell's cache of friendly names it has already resolved.
    ///
    /// Measured as the only place a plain name for Notepad or for Explorer
    /// appears on this machine: neither carries a `FriendlyAppName` of its own
    /// and both spell theirs as a resource reference this crate cannot follow.
    const MUI_CACHE: &str =
        "Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\Shell\\MuiCache";

    /// The payload's extension, dot included, as Windows counts it.
    ///
    /// Everything from the last dot, so `report.tar.gz` is `.gz` and
    /// `.gitignore` is all extension — which is the shape the registry keys a
    /// name on. A name with no dot, or with nothing after it, has no extension
    /// and there is nothing to ask about.
    pub fn extension_of(payload_name: &str) -> Option<String> {
        // A conformant container's payload name has been through
        // `slpc::check_payload_name`, which rejects every separator, so this is
        // a plain filename. Checked again rather than assumed, because a
        // separator here would turn an extension into a registry path.
        if payload_name.is_empty() || payload_name.contains(['/', '\\']) {
            return None;
        }
        let dot = payload_name.rfind('.')?;
        let extension = &payload_name[dot..];
        if extension.len() == 1 {
            return None;
        }
        Some(extension.to_owned())
    }

    /// The `ProgID` the platform would open this extension with.
    ///
    /// A per-user choice wins outright, including where it names a `ProgID` that
    /// is not registered. That is measured rather than assumed: `.txt` here
    /// chooses a packaged application that is no longer installed, and
    /// `AssocQueryString` answers nothing for it rather than falling back to
    /// the machine-wide `txtfile`. Falling back would name an application the
    /// platform would not actually use.
    pub fn progid_for(extension: &str) -> Option<String> {
        if let Some(chosen) = value(
            CURRENT_USER,
            &format!("{USER_CHOICE}\\{extension}\\UserChoice"),
            "ProgId",
        ) {
            return Some(chosen);
        }
        if let Some(machine_wide) = value(CLASSES_ROOT, extension, "") {
            return Some(machine_wide);
        }
        sole_candidate(extension)
    }

    /// The one `ProgID` under `OpenWithProgids`, where there is exactly one.
    ///
    /// `.webp` and `.shtml` are registered this way on this machine: no chosen
    /// application, no machine-wide default, one candidate, and
    /// `AssocQueryString` names it. Where there is more than one candidate
    /// there is no default, and picking among them would be the guess
    /// DESIGN.md §3 forbids.
    fn sole_candidate(extension: &str) -> Option<String> {
        let key = CLASSES_ROOT
            .open(format!("{extension}\\OpenWithProgids"))
            .ok()?;
        let mut names = key.values().ok()?.map(|(name, _)| name);
        let only = names.next()?;
        if only.is_empty() || names.next().is_some() {
            return None;
        }
        Some(only)
    }

    /// A name for the application behind a `ProgID`.
    ///
    /// A person recognises Microsoft Edge; nobody recognises `MSEdgePDF`.
    /// Three places hold a plain name, in the order the shell itself prefers
    /// them, and where none of them does the answer is nothing rather than the
    /// `ProgID` or the name of an executable.
    pub fn display_name(progid: &str) -> Option<String> {
        if let Some(named) = value(
            CLASSES_ROOT,
            &format!("{progid}\\Application"),
            "ApplicationName",
        ) {
            return Some(named);
        }

        let command = value(CLASSES_ROOT, &format!("{progid}\\shell\\open\\command"), "")?;
        let executable = expand_in(exe_path_in(&command)?, |name| std::env::var(name).ok());
        let file_name = executable.rsplit('\\').next()?;
        if file_name.is_empty() {
            return None;
        }

        if let Some(named) = value(
            CLASSES_ROOT,
            &format!("Applications\\{file_name}"),
            "FriendlyAppName",
        ) {
            return Some(named);
        }
        value(
            CURRENT_USER,
            MUI_CACHE,
            &format!("{executable}.FriendlyAppName"),
        )
    }

    /// One registry string, where the key is there, the value is there, and
    /// what it holds can be shown to a person.
    ///
    /// The indirect check sits here rather than at each caller so that no path
    /// through this module can put a resource reference on the card. A `ProgID`
    /// and a shell command never begin with `@`, so nothing else is lost.
    fn value(root: &Key, path: &str, name: &str) -> Option<String> {
        let read = root.open(path).ok()?.get_string(name).ok()?;
        let read = read.trim();
        if read.is_empty() || is_indirect(read) {
            return None;
        }
        Some(read.to_owned())
    }

    /// Whether a registry string points into a resource table instead of
    /// holding a name.
    ///
    /// Two shapes measured: `@C:\Windows\system32\notepad.exe,-469` for a
    /// classic binary, and `@{Package?ms-resource://...}` for a packaged
    /// application. Following either is `SHLoadIndirectString`, a raw call this
    /// crate cannot make, so both are refused.
    fn is_indirect(value: &str) -> bool {
        value.starts_with('@')
    }

    /// The executable in a shell open command.
    ///
    /// Quoted where the path holds spaces and bare otherwise, and the
    /// arguments do not always begin with a space: `Explorer.exe
    /// /idlist,%I,%L` and `"msedge.exe" --single-argument %1` were both
    /// measured on this machine.
    fn exe_path_in(command: &str) -> Option<&str> {
        let command = command.trim_start();
        let path = if let Some(rest) = command.strip_prefix('"') {
            rest.split_once('"').map(|(path, _)| path)?
        } else {
            command.split_whitespace().next()?
        };
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    }

    /// `%NAME%` replaced with what the lookup gives, and left standing where it
    /// gives nothing.
    ///
    /// No open command measured here used one — every one held an absolute
    /// path — but `%SystemRoot%\system32\...` is a documented shape. A path
    /// left holding a literal `%SystemRoot%` matches no key and no cache entry,
    /// which is the honest failure; substituting an empty string would build a
    /// path that could match something else.
    fn expand_in(text: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(start) = rest.find('%') {
            let (before, from_percent) = rest.split_at(start);
            out.push_str(before);
            let Some((name, tail)) = from_percent[1..].split_once('%') else {
                out.push_str(from_percent);
                return out;
            };
            if let Some(expanded) = lookup(name) {
                out.push_str(&expanded);
            } else {
                out.push('%');
                out.push_str(name);
                out.push('%');
            }
            rest = tail;
        }
        out.push_str(rest);
        out
    }

    #[cfg(test)]
    mod tests {
        use super::{exe_path_in, expand_in, extension_of, is_indirect};

        /// Windows keys an association on everything after the last dot, which
        /// is not what `Path::extension` means: it calls `.gitignore` no
        /// extension at all, and the registry has a key for exactly that name.
        /// Taking the first dot instead of the last would ask about `.tar`.
        #[test]
        fn the_extension_is_everything_after_the_last_dot() {
            assert_eq!(extension_of("report.tar.gz").as_deref(), Some(".gz"));
            assert_eq!(extension_of(".gitignore").as_deref(), Some(".gitignore"));
            assert_eq!(extension_of("report.pdf").as_deref(), Some(".pdf"));
        }

        /// A name with no extension has nothing to ask about, and a trailing
        /// dot leaves an extension of just the dot: asking for the key `.`
        /// opens the class root itself, whose default value is not an
        /// association.
        #[test]
        fn a_name_without_an_extension_asks_nothing() {
            assert_eq!(extension_of("README"), None);
            assert_eq!(extension_of("report."), None);
            assert_eq!(extension_of(""), None);
            assert_eq!(extension_of("sub\\report.pdf"), None);
        }

        /// Splitting a command on whitespace alone loses the quoted path that
        /// every application under Program Files has, and would answer
        /// `C:\Program`. Splitting only on quotes loses the bare paths, of
        /// which Explorer's is one and does not put a space before its
        /// arguments.
        #[test]
        fn the_executable_comes_out_of_either_shape_of_command() {
            assert_eq!(
                exe_path_in(
                    r#""C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe" --single-argument %1"#
                ),
                Some(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe")
            );
            assert_eq!(
                exe_path_in(r"C:\Windows\system32\notepad.exe %1"),
                Some(r"C:\Windows\system32\notepad.exe")
            );
            assert_eq!(
                exe_path_in(r"C:\Windows\Explorer.exe /idlist,%I,%L"),
                Some(r"C:\Windows\Explorer.exe")
            );
            assert_eq!(exe_path_in(""), None);
            assert_eq!(exe_path_in(r#""C:\unterminated.exe %1"#), None);
        }

        /// A variable the environment does not hold has to stay literal.
        /// Dropping it would turn `%SystemRoot%\system32\notepad.exe` into
        /// `\system32\notepad.exe`, which is a real file on the system drive
        /// and would then be looked up as one.
        #[test]
        fn an_unknown_variable_is_left_standing() {
            let known = |name: &str| (name == "SystemRoot").then(|| r"C:\Windows".to_owned());
            assert_eq!(
                expand_in(r"%SystemRoot%\system32\notepad.exe", known),
                r"C:\Windows\system32\notepad.exe"
            );
            assert_eq!(
                expand_in(r"%NotSet%\system32\notepad.exe", known),
                r"%NotSet%\system32\notepad.exe"
            );
            assert_eq!(expand_in(r"C:\bare\path.exe", known), r"C:\bare\path.exe");
            assert_eq!(expand_in("%unclosed", known), "%unclosed");
        }

        /// Both shapes of resource reference have to be refused, or the card
        /// shows a person `@C:\Windows\system32\notepad.exe,-469` where it
        /// meant to show them Notepad. The packaged shape is the one that is
        /// easy to miss: it is the value of `ApplicationName`, which is
        /// otherwise the best answer there is.
        #[test]
        fn a_resource_reference_is_not_a_name() {
            assert!(is_indirect(r"@C:\Windows\system32\notepad.exe,-469"));
            assert!(is_indirect(
                "@{Windows.PrintDialog_6.2.1.0_neutral_neutral_cw5n1h2txyewy?ms-resource://Windows.PrintDialog/resources/DisplayName}"
            ));
            assert!(!is_indirect("Microsoft Edge"));
            assert!(!is_indirect("Notepad"));
        }
    }
}
