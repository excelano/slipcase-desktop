// The path a payload takes on its way to the operating system.
//
// Nothing covered this until 2026-08-27, which was found by asking what tests
// the handover and getting no answer. `src/lib.rs`'s tests extract into a
// directory the test made; the window extracts into a scratch directory the
// application made, and hands the result to `opener`. The difference is where
// every property that matters lives: the mode of the directory, whether the
// file is reachable by the process that will open it, and whether the mark the
// container carried came with it.
//
// What is deliberately not here is `opener::open` itself. Calling it launches
// whatever the desktop has registered, which is somebody's PDF viewer appearing
// on their screen in the middle of a test run. The question that call answers
// on this platform — can the handler read the file — is asked below by reading
// it from another process, which is the handler's position exactly: a separate
// process running as the same user. On macOS under the App Sandbox it is not,
// and `CHECKLIST.md` item 6 is where that one lives.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

use std::path::Path;

/// A container holding `payload`, written to `dir`.
fn container(dir: &Path, payload: &[u8]) -> std::path::PathBuf {
    let path = dir.join("report.pdf.slpc");
    let mut bytes = Vec::new();
    slpc::pack_reader(
        "report.pdf",
        payload,
        slpc::toml_edit::DocumentMut::new(),
        &mut bytes,
    )
    .expect("packs");
    std::fs::write(&path, &bytes).expect("writes");
    path
}

/// Mark `path` the way this platform's downloaders mark a file.
///
/// **Written directly rather than through `slpc::provenance`**, deliberately: a
/// test that marks a file with the code under test asks the library whether it
/// agrees with itself, and passes just as happily if both halves are wrong
/// together. `slpc-rust` has `testsupport` for exactly this and it is a member
/// of that workspace, not reachable from here.
///
/// Returns false where the filesystem will not hold the mark, which is a fact
/// about the machine rather than something this code can answer for.
fn mark_as_downloaded(path: &Path) -> bool {
    #[cfg(target_os = "macos")]
    {
        // `flags;timestamp;agent;event-uuid`. 0083 is what Safari writes for a
        // download it has not yet had assessed.
        xattr::set(path, "com.apple.quarantine", b"0083;68ae0000;Safari;").is_ok()
    }
    #[cfg(target_os = "linux")]
    {
        xattr::set(
            path,
            "user.xdg.origin.url",
            b"https://example.invalid/a.slpc",
        )
        .is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        let mut stream = path.as_os_str().to_os_string();
        stream.push(":Zone.Identifier");
        std::fs::write(
            stream,
            b"[ZoneTransfer]\r\nZoneId=3\r\nHostUrl=https://example.invalid/a.slpc\r\n",
        )
        .is_ok()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = path;
        false
    }
}

/// The scratch directory the window makes, built the same way.
///
/// `App::scratch_dir` is private to the binary and this is an integration test,
/// so the construction is repeated here. `the_handover_directory_is_private` in
/// `src/main.rs` is what holds the real one to the same shape; if these two
/// drift, that test is the one that is right.
fn scratch() -> tempfile::TempDir {
    let mut builder = tempfile::Builder::new();
    builder.prefix("slipcase-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        builder.permissions(std::fs::Permissions::from_mode(0o700));
    }
    builder.tempdir().expect("a scratch directory")
}

/// A payload handed over is where it should be, intact, and readable by the
/// process that will open it.
///
/// **The defect this catches is a handover nobody can complete.** The scratch
/// directory became 0700 on 2026-08-27, to stop every payload somebody pressed
/// Open on being readable by every account on the machine. A directory mode is
/// exactly the kind of change that fixes one thing and breaks the thing it was
/// protecting, and on this platform the process that opens the payload is a
/// separate one — so it is asked from a separate one.
#[test]
fn a_payload_handed_over_is_readable_by_another_process() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = container(dir.path(), b"%PDF-1.7 the payload\n");
    let scratch = scratch();

    let landed = match slipcase_desktop::extract(source.as_path(), scratch.path(), &slipcase_desktop::Watch::new())
        .expect("extracts")
    {
        slipcase_desktop::Extracted::Done(p) => p,
        slipcase_desktop::Extracted::Cancelled => unreachable!("an unwatched copy"),
    };

    // Compared as resolved paths rather than as strings. On Windows
    // `slpc::payload_path` canonicalises the destination, so what comes back
    // carries the `\\?\` verbatim prefix and `starts_with` on the un-prefixed
    // scratch path is false. `src/lib.rs`'s extraction test learned this on
    // 2026-08-27 and this file learned it again the same day, which is what
    // `windows.yml` exists for.
    assert_eq!(
        std::fs::canonicalize(landed.parent().expect("a parent")).expect("resolves"),
        std::fs::canonicalize(scratch.path()).expect("resolves"),
        "the payload landed outside the scratch directory: {}",
        landed.display()
    );
    assert_eq!(landed.file_name().expect("a name"), "report.pdf");
    assert_eq!(
        std::fs::read(&landed).expect("this process can read it"),
        b"%PDF-1.7 the payload\n"
    );

    // The handler's position: another process, same user. `cat` rather than a
    // Rust child, because what is being asked is whether an ordinary program
    // launched by the desktop can get at the bytes.
    //
    // Unix only. There is no `cat` on Windows, and the question there is not
    // this one — `%TEMP%` is inside the user's profile and inherits its access
    // list, so nothing was asked for and nothing has to be checked back.
    #[cfg(unix)]
    {
        let out = std::process::Command::new("cat")
            .arg(&landed)
            .output()
            .expect("runs cat");
        assert!(
            out.status.success(),
            "another process could not read the payload: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(out.stdout, b"%PDF-1.7 the payload\n");
    }
}

/// A payload extracted into a private directory stays inside it.
///
/// **This does not test `App::scratch_dir`'s mode**, and its first draft said
/// it did. `scratch()` above is this file's own copy of that builder, because
/// the real one is private to the binary, so breaking the application's copy
/// changes nothing here. `the_handover_directory_is_private` in `src/main.rs`
/// is what holds the mode, and it bites.
///
/// What this holds is the half that copy cannot fake: that extraction puts the
/// payload inside the directory it was given, whatever that directory's mode.
#[test]
#[cfg(unix)]
fn the_payload_waits_somewhere_private() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().expect("a temporary directory");
    let source = container(dir.path(), b"private\n");
    let scratch = scratch();
    slipcase_desktop::extract(source.as_path(), scratch.path(), &slipcase_desktop::Watch::new())
        .expect("extracts");

    let mode = std::fs::metadata(scratch.path())
        .expect("stats")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o700, "this file's own builder: {:o}", mode & 0o777);

    let landed = scratch.path().join("report.pdf");
    assert!(landed.is_file(), "the payload is not in the directory it was given");
    assert_eq!(std::fs::read(&landed).expect("reads"), b"private\n");
}

/// Opening a second container reuses the directory rather than failing.
///
/// **The defect this catches was live for about an hour today.** Routing
/// extraction through `slpc::Destination` with `force` false made the handover
/// refuse to replace, and the scratch directory is one directory for a whole
/// session — so opening two containers whose payloads share a name failed on
/// the second. The conformance corpus caught it, twenty-five cases into a run;
/// nothing in `cargo test` did, because every test made its own directory.
#[test]
fn a_second_container_can_be_handed_over_into_the_same_directory() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let scratch = scratch();

    let first = dir.path().join("first");
    std::fs::create_dir(&first).expect("makes it");
    let a = container(&first, b"the first payload\n");

    let second = dir.path().join("second");
    std::fs::create_dir(&second).expect("makes it");
    let b = container(&second, b"the second payload\n");

    slipcase_desktop::extract(a.as_path(), scratch.path(), &slipcase_desktop::Watch::new())
        .expect("the first extracts");
    let landed = match slipcase_desktop::extract(b.as_path(), scratch.path(), &slipcase_desktop::Watch::new())
        .expect("the second extracts into the same directory")
    {
        slipcase_desktop::Extracted::Done(p) => p,
        slipcase_desktop::Extracted::Cancelled => unreachable!("an unwatched copy"),
    };

    assert_eq!(
        std::fs::read(&landed).expect("reads"),
        b"the second payload\n",
        "the second container's payload is what is handed over"
    );
}

/// Editing a container that arrived from elsewhere keeps where it came from.
///
/// **The defect this catches was found by reading a security document and
/// checking its claims.** Every provenance rule in `DESIGN.md` §5 is about
/// extraction — a payload leaving a container. Saving is the same question from
/// the other side and nobody had asked it: `Destination::in_place` replaces a
/// file by renaming a fresh one over it, and a fresh file carries no mark, so
/// changing one key and pressing Save stripped whatever the platform had
/// recorded. Every payload extracted afterwards was unmarked too, because
/// carrying copies from the container.
///
/// Fixed in `slpc` 0.3.7 and held there by a test of its own. This is the same
/// property asked of the application, through the save path the window uses,
/// because that is what a person meets. Skipped where the filesystem will not
/// hold a mark, announced rather than passed quietly.
#[test]
fn saving_an_edit_keeps_where_the_container_came_from() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = container(dir.path(), b"the payload\n");

    if !mark_as_downloaded(&path) {
        eprintln!("skipped: this filesystem will not hold a mark");
        return;
    }
    assert!(slpc::provenance::arrived_from_elsewhere(&path));

    let mut opened = slipcase_desktop::Opened::open(&path);
    opened
        .metadata
        .as_mut()
        .expect("a document")
        .insert("title", slpc::toml_edit::value("edited"));
    assert!(matches!(
        opened.save(None).expect("saves"),
        slipcase_desktop::Saved::Written
    ));

    assert!(
        slpc::provenance::arrived_from_elsewhere(&path),
        "the save laundered the container"
    );

    // And the edit actually landed, or the assertion above is about a file
    // nothing rewrote.
    let after = slipcase_desktop::Opened::open(&path);
    assert!(
        after
            .metadata
            .expect("a document")
            .get("title")
            .is_some_and(|v| v.as_str() == Some("edited")),
        "the edit did not land"
    );
}
