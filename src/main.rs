//! Slipcase: a desktop application over the `slpc` library.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

// `deny` rather than `forbid`, and the difference is the whole of the
// exception. `forbid` cannot be lifted anywhere beneath it, and receiving a
// document from macOS needs one Objective-C method, which cannot be written
// without `unsafe`. Every module below is still denied; `opened_document` is
// the single `allow`, and `src/lib.rs` — which is where containers are
// actually read and written — keeps `forbid` untouched. DESIGN.md §2.
#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
// Windows creates a console for a console-subsystem process, and a file
// manager launching this one is not attached to a terminal, so double-clicking
// a container opened a black console window behind the application. Found by
// looking at the first frame Windows ever drew of this. The attribute is
// ignored everywhere else, and it is off in a debug build because that is
// where a panic message still has somewhere to go.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// The one exception to `deny(unsafe_code)` above, and the only module in this
// application that writes `unsafe`. macOS is the only platform of the three
// that does not deliver a double-clicked container as `argv[1]`.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
mod opened_document;

// Which way the desktop's light and dark setting points. A module rather than a
// few lines here because only one of the three platforms needs any of it, and
// the one value that is a judgement rather than a reading wants somewhere to be
// argued and tested.
mod system_theme;

use std::path::{Path, PathBuf};
use std::sync::mpsc;

use eframe::egui;

use slipcase_desktop::potext::{self, fill, t};
use slipcase_desktop::{
    create, extract, extract_at, why_not_a_payload, Created, Extracted, Opened, Payload,
    RequiredKeys, Saved, Watch,
};

/// The window's identity to the desktop environment.
///
/// A Wayland compositor matches this against the basename of the `.desktop`
/// entry to find the window's icon and its name, so the two have to agree.
/// DESIGN.md §8 installs that entry; this is the half that lives in the binary.
const APP_ID: &str = "slipcase-desktop";

/// The window's icon on Windows, which has no `.desktop` entry to find one in.
///
/// `APP_ID` above is how Linux answers this question and it does nothing here:
/// `with_app_id` is Wayland's `xdg_toplevel.set_app_id` and neither egui,
/// eframe, nor winit turns it into anything on Windows. Measured by reading
/// all three. Windows takes a window's icon from a resource compiled into the
/// executable, and compiling one needs `rc.exe` or `windres`, which DESIGN.md
/// §2 keeps out of the build. So the icon is carried as bytes and handed to
/// the window at run time, which needs no build step at all.
#[cfg(target_os = "windows")]
const WINDOW_ICON: &[u8] = include_bytes!("../packaging/windows/slipcase.ico");

/// The icon at the largest size the drawing carries without being upscaled.
///
/// A window gets one image and Windows scales it to 16 in the title bar and 32
/// in the task bar, doubling both at 200%. 64 is a whole multiple of those
/// four, so each is an integer downsample of the same drawing.
///
/// **It is not a whole multiple of what the intermediate scalings ask for**,
/// which this comment claimed until somebody looked: 125% wants 20 and 40,
/// 150% wants 24 and 48, and none of those divides 64. Those sizes are
/// resampled rather than downsampled evenly. Looked at on 2026-08-26 at 125%
/// and 200% — the title bar and the task bar read cleanly at both, so the
/// resampling costs nothing a person would notice, and 64 stays the choice
/// because it is the largest entry no scaling has to enlarge.
#[cfg(target_os = "windows")]
fn window_icon() -> Option<egui::IconData> {
    let directory = ico::IconDir::read(std::io::Cursor::new(WINDOW_ICON)).ok()?;
    let entry = directory.entries().iter().find(|e| e.width() == 64)?;
    let image = entry.decode().ok()?;
    Some(egui::IconData {
        rgba: image.rgba_data().to_vec(),
        width: image.width(),
        height: image.height(),
    })
}

/// Where the last container was chosen from, remembered between runs.
///
/// A convenience rather than a setting, so it goes in the state directory the
/// XDG base directory specification names for exactly that rather than beside
/// somebody's configuration. Every failure to read or write it is ignored: a
/// dialog that opens somewhere else is not worth a message, and a person who
/// cannot write to their own state directory has a larger problem than this.
mod last_folder {
    use std::path::{Path, PathBuf};

    /// The state directory, per the XDG base directory specification.
    fn base() -> Option<PathBuf> {
        if let Some(state) = std::env::var_os("XDG_STATE_HOME") {
            return Some(PathBuf::from(state));
        }
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
    }

    fn file_in(base: &Path) -> PathBuf {
        base.join("slipcase-desktop").join("last-folder")
    }

    /// The folder to start the dialog in, where there is one worth starting in.
    pub fn read() -> Option<PathBuf> {
        read_from(&base()?)
    }

    /// Remember where this container came from.
    pub fn write(container: &Path) {
        if let Some(base) = base() {
            write_to(&base, container);
        }
    }

    /// Split out from [`read`] so a test can say where to look without touching
    /// the environment every other test is sharing.
    fn read_from(base: &Path) -> Option<PathBuf> {
        let text = std::fs::read_to_string(file_in(base)).ok()?;
        let folder = PathBuf::from(text.trim_end());
        // Somewhere that has since been moved or removed is not somewhere to
        // open a dialog.
        folder.is_dir().then_some(folder)
    }

    fn write_to(base: &Path, container: &Path) {
        let Some(folder) = container.parent().and_then(Path::to_str) else {
            // A folder whose name is not UTF-8 is not remembered rather than
            // remembered wrongly.
            return;
        };
        let file = file_in(base);
        let Some(dir) = file.parent() else {
            return;
        };
        if std::fs::create_dir_all(dir).is_ok() {
            // Removed first, so a symbolic link left at this path is replaced
            // rather than followed. Inside the user's own state directory, so
            // it takes an attacker who can already write there — but the cost
            // of not doing it is that this application writes a path of its
            // choosing to a file of theirs, and the cost of doing it is a
            // system call.
            // `remove_file` then `create_new`, not then `write`. Removing
            // narrows the window and `fs::write` would still open
            // `O_CREAT|O_TRUNC` and follow a link replanted inside it;
            // `create_new` refuses a path that exists, so the file this writes
            // is one it created. Nothing is retried: not remembering a folder
            // costs a dialog opening somewhere less useful.
            let _ = std::fs::remove_file(&file);
            let _ = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&file)
                .and_then(|mut f| std::io::Write::write_all(&mut f, folder.as_bytes()));
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{read_from, write_to};

        /// Remembering a folder replaces a symbolic link rather than following
        /// it.
        ///
        /// Catches a plain `fs::write` at this path. It is inside the user's
        /// own state directory, so it takes somebody who can already write
        /// there — but what it buys them is this application writing a path of
        /// its choosing into a file of theirs, and the cost of not letting them
        /// is one system call.
        #[test]
        #[cfg(unix)]
        fn remembering_a_folder_does_not_follow_a_link() {
            use super::file_in;

            let base = tempfile::tempdir().expect("a temporary directory");
            let victim = base.path().join("victim");
            std::fs::write(&victim, b"NOT A PATH").expect("writes");

            let state = file_in(base.path());
            std::fs::create_dir_all(state.parent().expect("a parent")).expect("makes it");
            std::os::unix::fs::symlink(&victim, &state).expect("links");

            let container = base.path().join("somewhere").join("c.slpc");
            std::fs::create_dir_all(container.parent().expect("a parent")).expect("makes it");
            write_to(base.path(), &container);

            assert_eq!(
                std::fs::read(&victim).expect("the victim survives"),
                b"NOT A PATH",
                "the folder was written through the link"
            );
        }

        #[test]
        fn a_folder_survives_being_written_and_read() {
            let state = tempfile::tempdir().expect("a temporary directory");
            let containers = tempfile::tempdir().expect("a temporary directory");
            let container = containers.path().join("one.slpc");

            assert_eq!(read_from(state.path()), None, "nothing remembered yet");

            write_to(state.path(), &container);
            assert_eq!(
                read_from(state.path()).as_deref(),
                Some(containers.path()),
                "the folder it came from"
            );
        }

        /// A folder that has gone is not a folder to open a dialog in.
        #[test]
        fn a_folder_that_is_no_longer_there_is_not_offered() {
            let state = tempfile::tempdir().expect("a temporary directory");
            let gone = tempfile::tempdir().expect("a temporary directory");
            let container = gone.path().join("one.slpc");

            write_to(state.path(), &container);
            assert!(read_from(state.path()).is_some());

            drop(gone);
            assert_eq!(read_from(state.path()), None);
        }
    }
}

fn main() -> eframe::Result {
    // Before anything that could put a sentence in front of somebody, which on
    // this line is the container named on the command line: a path that is not
    // a container is judged here and its verdict is read off the card.
    //
    // This is the only call to `activate` in the application, and the rest of
    // the program never asks what language it is in — a lookup that finds
    // nothing hands back the English it was given. `potext` says why the test
    // suite is deliberately outside this.
    potext::activate(&[
        ("de", include_str!("../po/de.po")),
        // The pseudolocale, in a debug build and never in a release one: it
        // translates nothing, accents everything and runs 40% long, so a
        // string that never went through `t` and a label built to the width of
        // English both show themselves on sight. `po/pseudo.sh` writes it and
        // says what each of its three findings looks like.
        #[cfg(debug_assertions)]
        ("en-x-pseudo", include_str!("../po/en-x-pseudo.po")),
    ]);

    // One positional path, which is what a file manager hands an application it
    // was asked to open a document with. A dialog and a drop arrive in slice 5,
    // and nothing here is a command-line interface: `slipcase` is that.
    let opened = std::env::args_os().nth(1).map(Opened::open);

    let viewport = egui::ViewportBuilder::default()
        .with_app_id(APP_ID)
        .with_inner_size([900.0, 640.0]);

    // Shadowed rather than made mutable, so that no platform without an icon
    // to set carries an unused `mut`.
    #[cfg(target_os = "windows")]
    let viewport = match window_icon() {
        Some(icon) => viewport.with_icon(icon),
        None => viewport,
    };

    // **Saying nothing here is not neutral on macOS, and it cost the Dock its
    // icon.** The bundle carries `slipcase-desktop.icns` and `CFBundleIconFile`
    // points at it, which is where a macOS application's icon comes from — so
    // this arm looked like it had nothing to do. It does. `eframe`'s
    // `epi_integration.rs` substitutes its own `data/icon.png` — the egui logo,
    // a white hexagon on black — for any viewport that names no icon, and
    // `app_icon.rs` then hands that to `-[NSApplication
    // setApplicationIconImage:]`, which outranks the bundle. Measured
    // 2026-08-28 against the signed bundle: the Dock showed the egui logo while
    // `NSWorkspace` and `NSRunningApplication` both still resolved the correct
    // drawing, which is why nothing short of looking at the Dock found it.
    //
    // An empty `IconData` is how the icon is declined rather than replaced:
    // `AppTitleIconSetter::new` turns one into `None`, and the macOS arm only
    // calls `setApplicationIconImage:` where there is an image. Handing over
    // the drawing again would also work and would carry a second copy of it in
    // the binary to overwrite the bundle's with a worse-scaled equal.
    #[cfg(target_os = "macos")]
    let viewport = viewport.with_icon(egui::IconData::default());

    // Before `eframe`, because macOS dispatches the document that launched this
    // application before `eframe`'s creation closure is reached, and AppKit's
    // own handler refuses it there. Measured both ways: registering later
    // opened a container double-clicked into a running window and lost the one
    // that started it.
    #[cfg(target_os = "macos")]
    opened_document::watch();

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    // "Slipcase" rather than the crate name: DESIGN.md §8 puts the product name
    // in front of a person and keeps `slipcase-desktop` on disk and on PATH.
    eframe::run_native(
        "Slipcase",
        options,
        Box::new(move |cc| {
            // After AppKit has installed its own handler for this event, which
            // is the one that was refusing the document, and so late that the
            // window exists to be woken. macOS only: the other two platforms
            // read the path out of `argv` above and never reach this, which is
            // why the binding has to be spent explicitly on them.
            #[cfg(target_os = "macos")]
            opened_document::wake_with(&cc.egui_ctx);

            // Where the toolkit will not say whether the desktop is light or
            // dark, ask the desktop. Empty on the two platforms where `winit`
            // answers; on Linux it reads the portal and then follows it.
            // `DESIGN.md` §3.
            system_theme::follow(&cc.egui_ctx);

            // A container handed over on the command line, or double-clicked,
            // gets the same focus a container opened through the dialog does.
            // Read before the move, because the field initializers below run
            // in the order they are written.
            let focus_open = opened.is_some();

            Ok(Box::new(App {
                opened,
                scratch: None,
                extraction: Extraction::Idle,
                creating: Creating::Idle,
                replacing: None,
                said: None,
                picking: None,
                focus_open,
            }))
        }),
    )
}

struct App {
    /// The container named on the command line, if there was one.
    opened: Option<Opened>,
    /// Where extraction goes: a directory of this process's own, made on the
    /// first Open and removed with its contents when this drops. DESIGN.md §5
    /// keeps the application from ever writing beside the container it opened.
    scratch: Option<tempfile::TempDir>,
    /// What the last extraction did.
    extraction: Extraction,
    /// A file chosen to become the payload, waiting for a Save.
    ///
    /// DESIGN.md §5 makes replacing the payload an explicit action, and this is
    /// where explicit stops: choosing the file is not writing it. It waits here
    /// with the metadata edits so that one press of Save writes one container,
    /// rather than two writes with a window between them where a failure leaves
    /// half of what was asked for.
    replacing: Option<PathBuf>,
    /// A container being made out of a file somebody chose.
    creating: Creating,
    /// What the last write said, whether it saved a container or made one.
    said: Option<Said>,
    /// A file dialog open on another thread, and what its answer is for.
    picking: Option<Picking>,
    /// Whether the Open button still has to be given keyboard focus.
    ///
    /// Set when a container is shown and cleared the moment the focus is
    /// asked for, so that pressing Enter opens the payload and pressing Tab
    /// afterwards still moves away. Requesting it every frame would pin focus
    /// to the button and make the rest of the window unreachable from the
    /// keyboard, which is worse than the extra press this saves.
    focus_open: bool,
}

/// A dialog open on another thread.
struct Picking {
    /// Which question it is asking, since one channel serves all three.
    what: For,
    /// The one message the thread sends when the dialog closes.
    answer: mpsc::Receiver<Option<PathBuf>>,
}

/// What a dialog is being opened for.
///
/// One at a time, deliberately: three dialogs at once is three answers arriving
/// in an order nobody chose.
#[derive(Clone, Copy, PartialEq, Eq)]
enum For {
    /// A container to open.
    Container,
    /// Where to put the payload.
    ExtractTo,
    /// A file to become the payload.
    Replacement,
    /// A file to make a container out of.
    NewPayload,
    /// Where that container goes.
    NewContainer,
}

/// Where an extraction is going.
enum Target {
    /// The scratch directory, under the payload's own name. The platform is
    /// handed the file when it lands, which is what the Open button is.
    Handover(PathBuf),
    /// A path somebody named. Nothing is launched: they said where to put it,
    /// not what to do with it.
    Chosen(PathBuf),
}

/// What became of an extraction.
enum Extraction {
    /// Nothing has been asked for.
    Idle,
    /// A copy is under way on another thread.
    Running(Job<Extraction>),
    /// The payload is on disk, here.
    Done(PathBuf),
    /// The copy was stopped, and nothing of it was left behind.
    Cancelled,
    /// It could not be extracted, or could not be handed over.
    Failed(String),
}

/// A container being made out of a file, from the press to the answer.
///
/// Two dialogs and a copy, because neither half of it may be guessed. Which
/// file goes in is the whole of what the person is asking for, and where the
/// container lands is a location — DESIGN.md §5 has this application choosing
/// one only when nobody asked it to, and `slipcase pack`'s default of writing
/// beside the payload is a convention a command line can afford and a window
/// pressing a button cannot.
enum Creating {
    /// Nothing has been asked for.
    Idle,
    /// A file has been chosen, and the second dialog is asking where its
    /// container goes.
    Naming(PathBuf),
    /// The container is being written on another thread.
    Running(Job<Made>),
}

/// What became of making a container.
enum Made {
    /// It is at this path, and this is what could not be carried onto it.
    Done(PathBuf, Option<String>),
    /// Stopping was asked for, and nothing was left at the destination.
    Stopped,
    /// It could not be made, in the library's words. A container that was
    /// written and did not read back conformant arrives here too, for the
    /// reason [`Saved::Refused`] does at the bar: nothing is at the
    /// destination either way, and the sentence is what differs.
    Failed(String),
}

/// What pressing something on the card asks for.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Ask {
    /// Extract to the scratch directory and hand it to the platform.
    Open,
    /// Extract to somewhere a person names.
    Extract,
    /// Replace it with a file a person names.
    Replace,
    /// Forget a replacement already chosen.
    Undo,
}

impl Ask {
    /// Whether doing this has to decode the payload.
    ///
    /// Replacing does not. Nothing reads a member to write over it, so the one
    /// container the corpus holds that cannot be opened is still one whose
    /// payload can be swapped out. DESIGN.md §6.
    fn decodes(self) -> bool {
        matches!(self, Self::Open | Self::Extract)
    }

    /// Whether the card can offer it.
    ///
    /// Split out from the drawing so a test can ask. A button that is not
    /// offered says more than one that is offered and then fails: the refusal
    /// is a fact about this build, known before anything is pressed, and
    /// finding it out by pressing costs a dialog and a wait first.
    fn offered(self, payload: &Payload, busy: bool) -> bool {
        !busy && (!self.decodes() || payload.can_be_decoded())
    }
}

/// What the last Save did, in a sentence.
struct Said {
    text: String,
    /// Whether it is something that went wrong, which decides its colour. A
    /// save that did not happen has to look different from one that did.
    wrong: bool,
}

/// A copy under way, and what its thread will say when it stops.
///
/// Generic over that answer because both of this window's long copies want the
/// same three things — something to watch, something to measure it against,
/// and one message at the end — and they finish in different vocabularies.
struct Job<T> {
    /// Shared with the thread doing the copying.
    watch: Watch,
    /// How much there is to copy: what the central directory said the payload
    /// measures, or what the file being packed measures on disk.
    total: u64,
    /// The one message the thread sends when it is done.
    outcome: mpsc::Receiver<T>,
}

/// What to tell a person about a handover the platform refused.
///
/// **`opener::OpenError` prints `IO error` and nothing else** for the variant
/// carrying an `io::Error`, deliberately: the crate puts the detail in
/// `source()` and keeps `Display` to a category. Formatting the error itself
/// therefore produced *the system would not open it: IO error*, which tells a
/// person nothing about what to do next — and this is a sentence the
/// application wrote rather than one the platform handed over, so it is this
/// application's to get right. Found by pressing Open during the window
/// walkthrough on 2026-08-26, against a payload the shell will not open and a
/// security warning somebody cancelled; below the window both had said exactly
/// what was wrong.
///
/// The source is preferred over the error's own wording wherever there is one,
/// because that is where every variant this can produce keeps the platform's
/// sentence.
fn why(e: &opener::OpenError) -> String {
    match std::error::Error::source(e) {
        Some(platform) => platform.to_string(),
        None => e.to_string(),
    }
}

/// The colour a warning is said in, dark enough to read on a light background.
///
/// **egui's own `warn_fg_color` and `error_fg_color` are chosen against a dark
/// background and are not darkened for a light one.** Measured on 2026-08-26,
/// against the card the lines are drawn on: the provenance line came to 2.79:1
/// in light mode and the failure line to 3.76:1, where WCAG asks 4.5:1 for body
/// text and ordinary text on the same card measures 7.59:1. So the one line
/// the card colours on purpose — because a walkthrough found that nobody reads
/// a weak grey one — was the least readable thing on it, in the theme half the
/// machines in the world are set to.
///
/// Dark mode keeps egui's orange, which measures 7.53:1 and needs no help. Its
/// red does not: pure red on the dark card is 4.31:1, under the same bar, so
/// [`error_colour`] lightens it there rather than leaving the one theme that
/// was looked at failing too.
///
/// **The ground all of that is measured against is `Visuals::panel_fill`**, and
/// saying so is what the first pass at this got wrong. `Frame::group` — what
/// the card is drawn in — sets a margin, a corner radius and a stroke and no
/// fill at all, so the pixel behind these lines is the panel's: grey 27 in dark
/// mode and grey 248 in light. The dark figures above were first recorded
/// against grey 32, which is `faint_bg_color` composited over the panel and is
/// not drawn anywhere near the card, and every one of them came out low.
///
/// These are the card's colours and not a theme, which is why they live here
/// rather than in a `Visuals` this application would then have to maintain
/// whole.
fn warn_colour(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        visuals.warn_fg_color
    } else {
        egui::Color32::from_rgb(180, 70, 0)
    }
}

/// The colour a refusal is said in. [`warn_colour`] says why both exist.
fn error_colour(visuals: &egui::Visuals) -> egui::Color32 {
    if visuals.dark_mode {
        egui::Color32::from_rgb(255, 80, 80)
    } else {
        egui::Color32::from_rgb(180, 0, 0)
    }
}

impl App {
    /// Show a container, and forget what the last one's Open did.
    ///
    /// Without the second half, a message about the container just closed
    /// would sit under the card of the one just opened.
    fn show(&mut self, opened: Opened) {
        // A copy still running belongs to the container being closed. Left
        // alone it would finish and hand that payload to the platform, minutes
        // after the person moved on to another container.
        if let Extraction::Running(job) = &self.extraction {
            job.watch.cancel();
        }
        self.opened = Some(opened);
        self.extraction = Extraction::Idle;
        // The common thing to do with a container just opened is to open what
        // is in it, so Enter does that without a reach for the mouse. Only
        // asked for here: a container already on screen has had its chance and
        // the focus is now wherever the person put it.
        self.focus_open = true;
        // A file chosen to replace the payload of the container being closed
        // is not a file to replace the payload of the next one.
        self.replacing = None;
        self.said = None;
    }

    /// What the bar has to say about the container.
    ///
    /// The first is whether there is anything to write, which is what turns
    /// Save on: an edited document, a payload waiting to replace the one in
    /// there, or both.
    ///
    /// Both halves, and not one or the other. A save that failed changed
    /// nothing, so there is still something to write, and showing only the
    /// edited mark hides the reason behind the very state the failure caused.
    fn notes(&self) -> (bool, Option<&Said>) {
        let edited = self.opened.as_ref().is_some_and(Opened::metadata_edited)
            || self.replacing.is_some();
        (edited, self.said.as_ref())
    }

    /// Whether a press that would put another container on screen has to wait.
    ///
    /// A dialog is up, or a container is being made and will be shown the
    /// moment it is. Both end with something replacing what the window is
    /// holding, and two of them at once is two answers arriving in an order
    /// nobody chose — which is the reason `For` already allows one dialog at a
    /// time, applied to the other thing that finishes by calling `show`.
    fn busy(&self) -> bool {
        self.picking.is_some() || matches!(self.creating, Creating::Running(_))
    }

    /// The bar across the top: open, new, save, undo, redo, and what the last
    /// write said. Returns the button pressed, if one was.
    fn bar(&self, ui: &mut egui::Ui) -> Option<Pressed> {
        let (edited, said) = self.notes();
        let history = self.opened.as_ref().and_then(|o| o.metadata.as_ref());
        let (can_undo, can_redo) =
            history.map_or((false, false), |d| (d.can_undo(), d.can_redo()));
        let busy = self.busy();
        let mut pressed = None;
        // Not through `press` below, which holds `pressed` for the whole panel.
        let mut stop = false;
        let mut press = |ui: &mut egui::Ui, enabled: bool, label: &str, what: Pressed| {
            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                pressed = Some(what);
            }
        };
        // egui 0.36 folded `TopBottomPanel` and `SidePanel` into one `Panel`.
        egui::Panel::top("bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                press(ui, !busy, t("Open a container…"), Pressed::Open);
                press(ui, !busy, t("New container…"), Pressed::New);
                // Off until there is something to write, because DESIGN.md
                // §5 does not write a container nothing has changed in and
                // a button that does nothing should not invite a press.
                press(ui, edited, t("Save"), Pressed::Save);
                press(ui, can_undo, t("Undo"), Pressed::Undo);
                press(ui, can_redo, t("Redo"), Pressed::Redo);
                if edited {
                    ui.label(egui::RichText::new(t("edited")).italics().weak());
                }
                if let Some(said) = said {
                    let text = egui::RichText::new(&said.text);
                    ui.label(if said.wrong {
                        text.color(error_colour(ui.visuals()))
                    } else {
                        text.weak()
                    });
                }
                stop = making(ui, &self.creating);
            });
        });
        if stop {
            return Some(Pressed::StopMaking);
        }
        pressed
    }

    /// Write the edits back, and show what happened.
    fn save(&mut self) {
        let Some(opened) = &self.opened else {
            return;
        };
        let path = opened.path.clone();
        let outcome = opened.save(self.replacing.as_deref());

        let said = match &outcome {
            Ok(Saved::Written) => Said {
                text: t("Saved.").to_owned(),
                wrong: false,
            },
            Ok(Saved::Unchanged) => Said {
                text: t("Nothing had changed, so nothing was written.").to_owned(),
                wrong: false,
            },
            Ok(Saved::Refused(v)) => Said {
                text: fill(
                    t("Not saved. What was written did not read back conformant: {verdict}"),
                    &[("verdict", &v.to_string())],
                ),
                wrong: true,
            },
            Err(e) => Said {
                text: fill(t("Not saved: {reason}"), &[("reason", &e.to_string())]),
                wrong: true,
            },
        };

        if matches!(outcome, Ok(Saved::Written)) {
            // Read back what is now on disk, so the tree, the card, and the
            // edited mark all describe the container rather than the edit. This
            // is also what clears the replacement: it is in there now.
            self.show(Opened::open(&path));
        }
        self.said = Some(said);
    }

    /// Take a file chosen to become the payload, or say why it cannot be one.
    ///
    /// Refused here rather than at Save where it can be, so a name SPEC §2.3
    /// forbids is reported while the person still has the dialog in mind. The
    /// refusals this cannot see are the ones needing the container's member
    /// list, and those stay for Save to report.
    fn take_replacement(&mut self, file: PathBuf) {
        if let Some(why) = why_not_a_payload(&file) {
            self.replacing = None;
            self.said = Some(Said { text: why, wrong: true });
            return;
        }
        self.replacing = Some(file);
        self.said = None;
    }

    /// Take the thread's answer, once it has one.
    fn poll(&mut self) {
        let Extraction::Running(job) = &self.extraction else {
            return;
        };
        match job.outcome.try_recv() {
            Err(mpsc::TryRecvError::Empty) => {}
            Ok(finished) => self.extraction = finished,
            // The thread ended without sending, which it has no path to do.
            Err(mpsc::TryRecvError::Disconnected) => {
                self.extraction =
                    Extraction::Failed(t("the extraction stopped without saying why").to_owned());
            }
        }
    }

    /// Take the packing thread's answer, once it has one.
    fn poll_creating(&mut self) {
        let Creating::Running(job) = &self.creating else {
            return;
        };
        let made = match job.outcome.try_recv() {
            Err(mpsc::TryRecvError::Empty) => return,
            Ok(made) => made,
            // The thread ended without sending, which it has no path to do.
            Err(mpsc::TryRecvError::Disconnected) => {
                Made::Failed(t("the container was not made, and nothing said why").to_owned())
            }
        };
        self.creating = Creating::Idle;

        match made {
            Made::Done(path, provenance) => {
                // The answer to New container… is the container, so it takes
                // the window the way one opened through the dialog does. That
                // is also what puts the tree in front of somebody who has just
                // made a container carrying nothing but the two keys the
                // library wrote: the metadata is added here, in the editor
                // that already exists, rather than in a form this application
                // has no vocabulary to draw.
                last_folder::write(&path);
                self.show(Opened::open(&path));
                // After `show`, which clears what the last container said.
                self.said = Some(match provenance {
                    None => Said {
                        text: t("Made.").to_owned(),
                        wrong: false,
                    },
                    // A container that does not record where its payload came
                    // from is one the card will call local, and one whose
                    // payload leaves ungated when it is extracted. Said rather
                    // than logged: the person is holding the container it is
                    // true of.
                    Some(why) => Said {
                        text: fill(
                            t("Made. Where the payload came from could not be carried onto it: {reason}"),
                            &[("reason", &why)],
                        ),
                        wrong: true,
                    },
                });
            }
            Made::Stopped => {
                self.said = Some(Said {
                    text: t("Stopped. Nothing was left behind.").to_owned(),
                    wrong: false,
                });
            }
            Made::Failed(why) => self.said = Some(Said { text: why, wrong: true }),
        }
    }

    /// Start packing a container, on a thread of its own.
    ///
    /// On a thread for the reason extraction is: a payload is a file of
    /// arbitrary size and the window has to keep drawing while it is read, so
    /// that it can say how far along it is and offer to stop.
    fn start_creating(&mut self, payload: PathBuf, into: PathBuf, ctx: &egui::Context) {
        // What the payload measures on disk, which is what the count in
        // `create` advances against. Zero where it cannot be asked, which the
        // progress bar reads as done rather than as an error — the copy itself
        // is what will report a file it cannot read.
        let total = std::fs::metadata(&payload).map_or(0, |m| m.len());
        let watch = Watch::new();
        let (sender, outcome) = mpsc::channel();

        let theirs = watch.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let made = match create(&payload, &into, &theirs) {
                Ok(Created::Written { path, provenance }) => Made::Done(path, provenance),
                Ok(Created::Cancelled) => Made::Stopped,
                // The two halves of a container that is not there: one was
                // never written, and one was written and read back as
                // something this application will not put in front of anybody.
                // `save` splits the same pair into the same two sentences.
                Ok(Created::Refused(v)) => Made::Failed(fill(
                    t("Not made. What was written did not read back conformant: {verdict}"),
                    &[("verdict", &v.to_string())],
                )),
                Err(e) => Made::Failed(fill(
                    t("Not made: {reason}"),
                    &[("reason", &e.to_string())],
                )),
            };
            let _ = sender.send(made);
            // A pack that finishes while nothing is touching the window leaves
            // it asleep, and the frame that would have polled never comes.
            ctx.request_repaint();
        });

        self.creating = Creating::Running(Job {
            watch,
            total,
            outcome,
        });
    }

    /// Start extracting the payload, on a thread of its own.
    ///
    /// The window keeps drawing while it copies, which is what lets it show how
    /// far along it is and offer to stop. One thread serves both of DESIGN.md
    /// §5's destinations, because a payload of two gigabytes is a payload of two
    /// gigabytes whether it is going to a scratch directory or to a folder
    /// somebody chose. The handover happens over there too: `opener` starts a
    /// process, and starting it is not instant either.
    fn start_extraction(&mut self, target: Target) {
        let Some(opened) = &self.opened else {
            return;
        };

        let container = opened.path.clone();
        let total = opened.payload.as_ref().map_or(0, |p| p.size);
        let watch = Watch::new();
        let (sender, outcome) = mpsc::channel();

        let theirs = watch.clone();
        std::thread::spawn(move || {
            let copied = match &target {
                Target::Handover(dir) => extract(&container, dir, &theirs),
                Target::Chosen(path) => extract_at(&container, path, &theirs),
            };
            let finished = match copied {
                Ok(Extracted::Done(path)) => match target {
                    // Only the Open button launches anything. A person who said
                    // where to put the payload said where to put it.
                    Target::Chosen(_) => Extraction::Done(path),
                    Target::Handover(_) => match opener::open(&path) {
                        Ok(()) => Extraction::Done(path),
                        // Extraction worked and the handover did not, which is a
                        // different sentence: the payload is on disk either way.
                        Err(e) => Extraction::Failed(fill(
                            t("{file} was extracted, and the system would not open it: {reason}"),
                            &[("file", &slpc::display_path(&path)), ("reason", &why(&e))],
                        )),
                    },
                },
                Ok(Extracted::Cancelled) => Extraction::Cancelled,
                // The library's own wording. An encrypted payload and one
                // compressed by a method this build lacks both arrive here, and
                // both sit in a container that is conformant.
                Err(e) => Extraction::Failed(e.to_string()),
            };
            // Nobody is listening if the container was closed meanwhile, and
            // that is the cancel above having already done its work.
            let _ = sender.send(finished);
        });

        self.extraction = Extraction::Running(Job {
            watch,
            total,
            outcome,
        });
    }

    /// Start extracting to the scratch directory, to hand to the platform.
    fn start_handover(&mut self) {
        match self.scratch_dir() {
            Ok(dir) => self.start_extraction(Target::Handover(dir)),
            Err(why) => self.extraction = Extraction::Failed(why),
        }
    }

    /// Ask for a path, on a thread of its own.
    ///
    /// Not on this one. The portal backend puts the dialog in another process
    /// entirely, so blocking here does not make the dialog modal, it makes the
    /// window stop answering the compositor, and GNOME offers to force quit an
    /// application that is doing exactly what it was asked to. `rfd` supports
    /// being called from any thread in a windowed application, which this is.
    fn start_picking(&mut self, ctx: &egui::Context, what: For) {
        if self.picking.is_some() {
            return;
        }
        let (sender, answer) = mpsc::channel();
        let ctx = ctx.clone();

        // The file about to be packed, where there is one. Both of the two
        // questions a new container asks are about it.
        let packing = match &self.creating {
            Creating::Naming(payload) => Some(payload.clone()),
            Creating::Idle | Creating::Running(_) => None,
        };
        // Where the last container came from, so everything is found beside it
        // rather than from wherever the dialog would otherwise start. The one
        // exception is where a new container goes: the folder somebody just
        // chose a file out of is nearer to hand than the folder they last
        // opened a container from, and `slipcase pack` writes there by default.
        let start_in = match what {
            For::NewContainer => packing
                .as_deref()
                .and_then(Path::parent)
                .map(Path::to_owned)
                .or_else(last_folder::read),
            _ => last_folder::read(),
        };
        // The payload's own name, offered where the question is what to call
        // the file coming out. Somebody renaming it is choosing to.
        let suggested = match what {
            For::ExtractTo => self
                .opened
                .as_ref()
                .and_then(|o| o.payload.as_ref())
                .map(|p| p.name.clone()),
            // The naming convention SPEC §4 leaves as one: the payload's name
            // with `.slpc` after it. Offered rather than imposed — nothing
            // reads a container's name to find out what is inside it, so a
            // person who types something else has typed the name of their
            // container.
            For::NewContainer => packing
                .as_deref()
                .and_then(Path::file_name)
                .map(|name| format!("{}.slpc", name.to_string_lossy())),
            For::Container | For::Replacement | For::NewPayload => None,
        };

        std::thread::spawn(move || {
            let mut dialog = rfd::FileDialog::new();
            dialog = match what {
                For::Container => dialog
                    .set_title(t("Open a container"))
                    .add_filter(t("slipcases"), &["slpc"])
                    .add_filter(t("All files"), &["*"]),
                // No filter on either of these: a payload is any file at all,
                // which is what SPEC §2.3 leaves open.
                For::ExtractTo => dialog.set_title(t("Extract the payload to")),
                For::Replacement => dialog.set_title(t("Replace the payload with")),
                For::NewPayload => dialog.set_title(t("Make a container out of")),
                For::NewContainer => dialog
                    .set_title(t("Write the container to"))
                    .add_filter(t("slipcases"), &["slpc"])
                    .add_filter(t("All files"), &["*"]),
            };
            if let Some(folder) = start_in {
                dialog = dialog.set_directory(folder);
            }
            if let Some(name) = suggested {
                // **Not escaped**, and it was for a few hours on 2026-08-27,
                // which was wrong. This field's value becomes the name of a
                // file on disk: the comment above says the payload's own name
                // is offered and that renaming it is the person's choice, so
                // what goes in has to be a name. `display_name` renders a name
                // for reading — it turns U+202E into the eight characters
                // `\u{202E}` — and prefilling that meant Extract-to defaulted
                // to writing a file literally called `report\u{202E}fdp.exe`,
                // where `\` is a path separator on Windows. The handover path
                // writes the real name, so the two had come to disagree.
                //
                // The rule SPEC §3 states is about displaying a name. A text
                // field whose contents become a path is not a display, and the
                // card and the *Extracted to* line, which are, keep the escape.
                dialog = dialog.set_file_name(name);
            }
            // A save dialog for the one question that names a file that does
            // not exist yet, so the platform asks before overwriting.
            let chosen = match what {
                For::ExtractTo | For::NewContainer => dialog.save_file(),
                For::Container | For::Replacement | For::NewPayload => dialog.pick_file(),
            };
            let _ = sender.send(chosen);
            // Nothing has been touching the window while the dialog was up, so
            // it is asleep and has to be woken to notice the answer.
            ctx.request_repaint();
        });

        self.picking = Some(Picking { what, answer });
    }

    /// Take the dialog's answer, once it has one.
    ///
    /// Takes the context because one of the answers is a question: choosing a
    /// file to pack is what opens the dialog asking where its container goes.
    fn poll_picking(&mut self, ctx: &egui::Context) {
        let Some(picking) = &self.picking else {
            return;
        };
        let what = picking.what;
        let chosen = match picking.answer.try_recv() {
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => None,
            // A path, or a dialog somebody closed without choosing one.
            Ok(chosen) => chosen,
        };
        self.picking = None;

        let Some(path) = chosen else {
            // A dialog closed without an answer ends what it was part of. The
            // file already chosen for a container nobody named a place for is
            // not a file waiting for the next press.
            self.creating = Creating::Idle;
            return;
        };
        match what {
            For::Container => {
                last_folder::write(&path);
                self.show(Opened::open(path));
            }
            For::ExtractTo => self.start_extraction(Target::Chosen(path)),
            For::Replacement => self.take_replacement(path),
            For::NewPayload => self.take_new_payload(path, ctx),
            For::NewContainer => {
                let Creating::Naming(payload) = std::mem::replace(&mut self.creating, Creating::Idle)
                else {
                    return;
                };
                self.start_creating(payload, path, ctx);
            }
        }
    }

    /// Take a file chosen to be packed, or say why it cannot be one.
    ///
    /// Refused here rather than at the moment of packing, which is the same
    /// decision `take_replacement` records and the same call: a name SPEC §2.3
    /// forbids is a fact about the choice, and hearing it before the second
    /// dialog costs nobody a second question about a container that was never
    /// going to be written.
    fn take_new_payload(&mut self, payload: PathBuf, ctx: &egui::Context) {
        if let Some(why) = why_not_a_payload(&payload) {
            self.creating = Creating::Idle;
            self.said = Some(Said { text: why, wrong: true });
            return;
        }
        self.said = None;
        self.creating = Creating::Naming(payload);
        self.start_picking(ctx, For::NewContainer);
    }
}

/// What a button in the bar asked for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pressed {
    Open,
    New,
    Save,
    Undo,
    Redo,
    StopMaking,
}

/// Take the undo and redo chords before anything can answer them itself.
///
/// Before anything draws, so that a field with focus does not answer Ctrl+Z
/// with its own undo of its own text: the document's undo is the window's, the
/// way it is in Tommy Flyleaf. Redo is asked first, since its chord contains
/// undo's.
fn chords(ui: &egui::Ui) -> (bool, bool) {
    let redo = ui.input_mut(|i| {
        i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Z)
            || i.consume_key(egui::Modifiers::COMMAND, egui::Key::Y)
    });
    let undo = ui.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Z));
    (undo, redo)
}

/// The window before anything has been opened.
///
/// A function of its own for the reason `bar` and `card` are, and it earned
/// that when it stopped being a heading and a button: this is the one state
/// with no bar, and a container can be made from here, so what a press comes
/// to has to appear here or nowhere. Returns the button pressed, if one was.
/// The two ways into a container, in the order they are offered.
///
/// A function rather than the constant this was until German arrived: a
/// `const` cannot call a lookup, and these are the same two labels the toolbar
/// draws, so they have to come from the same catalogue entry or the window
/// says one thing in two places.
fn ways_in() -> [(&'static str, Pressed); 2] {
    [
        (t("Open a container…"), Pressed::Open),
        (t("New container…"), Pressed::New),
    ]
}

/// What a row of buttons will measure, so that something can be centred on it.
///
/// Asked of the style rather than written down, because the answer moves with
/// the font, the button padding and the scale: a constant here would be right
/// on this machine and wrong on a panel at 150%. `Button::ui` adds the same
/// padding to the same galley, which is what
/// `the_row_of_ways_in_measures_what_the_buttons_measure` holds this to.
fn row_width(ui: &egui::Ui, buttons: &[(&str, Pressed)]) -> f32 {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let padding = 2.0 * ui.spacing().button_padding.x;
    let labels: f32 = buttons
        .iter()
        .map(|(label, _)| {
            ui.painter()
                .layout_no_wrap((*label).to_owned(), font.clone(), egui::Color32::PLACEHOLDER)
                .size()
                .x
                + padding
        })
        .sum();
    #[allow(clippy::cast_precision_loss)]
    let gaps = buttons.len().saturating_sub(1) as f32;
    labels + gaps * ui.spacing().item_spacing.x
}

fn nothing_open(
    ui: &mut egui::Ui,
    creating: &Creating,
    said: Option<&Said>,
    busy: bool,
) -> Option<Pressed> {
    let mut pressed = None;
    ui.vertical_centered(|ui| {
        ui.add_space(72.0);
        ui.heading("Slipcase");
        ui.label(t("Open a container to see what is in it, or make one out of a file."));
        ui.add_space(12.0);
        // The width is measured and handed over rather than left to the
        // layout, and looking at the window is what settled that.
        // `vertical_centered` centres each child it is given by the size that
        // child asked for, and `ui.horizontal` asks for the whole width it is
        // offered — so a full-width row inside a centred column put both
        // buttons hard against the left edge under a centred heading, which is
        // how this drew until somebody looked. Every assertion in this file
        // passed against it.
        let row = egui::vec2(row_width(ui, &ways_in()), 0.0);
        ui.allocate_ui_with_layout(row, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            for (label, what) in ways_in() {
                if ui.add_enabled(!busy, egui::Button::new(label)).clicked() {
                    pressed = Some(what);
                }
            }
        });
        if making(ui, creating) {
            pressed = Some(Pressed::StopMaking);
        }
        if let Some(said) = said {
            let text = egui::RichText::new(&said.text);
            ui.label(if said.wrong {
                text.color(error_colour(ui.visuals()))
            } else {
                text.weak()
            });
        }
    });
    pressed
}

/// How far a container being made has got, and the way to stop it.
///
/// Drawn in two places and written once: the bar has it while a container is on
/// screen, and the empty state has it while one is not, because a person can
/// press New container… from either and the answer has to appear where they
/// are looking. Returns whether stopping was asked for.
fn making(ui: &mut egui::Ui, creating: &Creating) -> bool {
    let Creating::Running(job) = creating else {
        return false;
    };
    let done = job.watch.done();
    #[allow(clippy::cast_precision_loss)]
    let fraction = if job.total == 0 {
        1.0
    } else {
        done as f32 / job.total as f32
    };
    // A width of its own, because a `ProgressBar` in a row takes whatever is
    // left and the bar's row has a sentence after it.
    ui.add(
        egui::ProgressBar::new(fraction)
            .desired_width(160.0)
            .show_percentage(),
    );
    ui.button(t("Stop")).clicked()
}

/// The payload card: what it is, and what can be done with it.
///
/// DESIGN.md §3, and the two explicit actions §5 names. Not a method, because
/// the panel drawing it holds the document mutably for the tree and a `&self`
/// here would borrow the whole application alongside it. Returns what was
/// pressed, for the same reason.
fn card(
    ui: &mut egui::Ui,
    payload: &Payload,
    from_elsewhere: bool,
    extraction: &Extraction,
    replacing: Option<&std::path::Path>,
    busy: bool,
    focus_open: &mut bool,
) -> Option<Ask> {
    let mut asked = None;

    egui::Frame::group(ui.style()).show(ui, |ui| {
            // Through `slpc::display_name`, which SPEC §3 requires of anything
            // showing a member name. A payload called `report<U+202E>fdp.exe`
            // reads as `report.pdf` wherever the override is applied, and this
            // label sits beside a button that hands the file to whatever the
            // system registered for `.exe`.
            //
            // Measured 2026-08-27, and this is not the toolkit that applies it:
            // egui lays glyphs out in logical order and gives every one of these
            // characters zero advance width, so the override was never obeyed
            // here and the name simply rendered a character short. That is the
            // reason to escape rather than a reason not to. epaint carries a
            // to-do to heed them, so the immunity is somebody else's omission
            // and is scheduled to end; and a name that renders a character short
            // is already a name this card is not telling the truth about.
            ui.label(egui::RichText::new(slpc::display_name(&payload.name).into_owned()).strong());
            ui.label(payload.size_line());
            // Silent where the platform would not answer, rather than saying it
            // does not know.
            if let Some(application) = &payload.opens_with {
                ui.label(fill(t("Opens with {application}"), &[("application", application)]));
            }
            // After what the payload is, because both are true at once: the
            // platform would open a file of that name, and this build cannot
            // get the bytes out to give it one.
            if let Some(why) = &payload.unreadable {
                ui.label(
                    egui::RichText::new(fill(t("Cannot be opened here: {reason}"), &[("reason", why)]))
                        .color(error_colour(ui.visuals())),
                );
            }
            // A fact read out of the container rather than guessed from the
            // name, which is what DESIGN.md §3's rule against a filename table
            // forbids. Silent where the container records no mode, and on
            // Windows where a mode bit is not what makes a file executable.
            //
            // In the warning colour for the reason the provenance line below is:
            // it explains a refusal a person is about to meet — the platform
            // will decline to run the extracted copy — and weak grey is what the
            // walkthrough found nobody reads.
            if payload.executable {
                ui.label(
                    egui::RichText::new(t(
                        "The payload is an executable file; the extracted copy will not be executable.",
                    ))
                    .color(warn_colour(ui.visuals())),
                );
            }

            // Said rather than acted on. DESIGN.md §5's amendment: the payload
            // leaves carrying whatever the container carried, and what the
            // platform then does about it is the platform's business. In the
            // warning colour rather than the error one, because a container
            // arriving from elsewhere is a thing to know and not a thing that
            // went wrong — and not in weak grey, which the walkthrough already
            // found nobody reads.
            if from_elsewhere {
                ui.label(
                    egui::RichText::new(t(
                        "This container arrived from elsewhere, and the payload will carry that.",
                    ))
                    .color(warn_colour(ui.visuals())),
                );
            }

            match extraction {
                // A copy under way takes the row: there is one payload and one
                // thread, so nothing else on this card can be asked for yet.
                Extraction::Running(job) => {
                    let done = job.watch.done();
                    #[allow(clippy::cast_precision_loss)]
                    let fraction = if job.total == 0 {
                        1.0
                    } else {
                        done as f32 / job.total as f32
                    };
                    ui.add(egui::ProgressBar::new(fraction).show_percentage());
                    if ui.button(t("Cancel")).clicked() {
                        job.watch.cancel();
                    }
                }
                _ => {
                    ui.horizontal(|ui| {
                        for (label, ask) in [
                            (t("Open"), Ask::Open),
                            (t("Extract…"), Ask::Extract),
                            (t("Replace…"), Ask::Replace),
                        ] {
                            // Off while a dialog is up, because there is one
                            // dialog at a time and a second press would be
                            // silently dropped, and off where this build
                            // cannot decode what the button would decode.
                            let button =
                                ui.add_enabled(ask.offered(payload, busy), egui::Button::new(label));
                            // The line above says why, and a button explaining
                            // itself where the pointer already is saves
                            // looking for it.
                            let button = match &payload.unreadable {
                                Some(why) if ask.decodes() => button.on_disabled_hover_text(why),
                                _ => button,
                            };
                            // Once, and only where it would do something: a
                            // focus ring on a disabled button says press me
                            // about a button that cannot be pressed, and a
                            // payload this build cannot decode leaves Open
                            // disabled.
                            if *focus_open && ask == Ask::Open && button.enabled() {
                                button.request_focus();
                                *focus_open = false;
                            }
                            if button.clicked() {
                                asked = Some(ask);
                            }
                        }
                    });
                }
            }

            if let Some(file) = replacing {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(fill(
                            t("Will be replaced by {file} when this is saved."),
                            &[("file", &file.display().to_string())],
                        ))
                        .italics(),
                    );
                    // Somewhere to go after choosing the wrong file. Without
                    // this the only way out is closing the container.
                    if ui.button(t("Undo")).clicked() {
                        asked = Some(Ask::Undo);
                    }
                });
            }

            match extraction {
                Extraction::Idle | Extraction::Running(_) => {}
                Extraction::Done(path) => {
                    // `display_path` strips the `\\?\` prefix and does nothing
                    // about the name inside the path, which is the payload's
                    // and is attacker-controlled. Both, so the line says where
                    // the file is and what it is called.
                    ui.label(fill(
                        t("Extracted to {file}"),
                        &[("file", &slpc::display_name(&slpc::display_path(path)))],
                    ));
                }
                Extraction::Cancelled => {
                    // True now, and it was not until 2026-08-27: extraction
                    // used to truncate the destination before reading a byte,
                    // so stopping destroyed a file somebody had chosen to
                    // replace and then deleted it. Nothing is opened at the
                    // destination until the payload is whole.
                    ui.label(t("Stopped. Nothing was left behind."));
                }
                Extraction::Failed(why) => {
                    // The same red the bar gives a save that did not happen.
                    // An extraction that failed and one that landed are not
                    // two shades of the same thing, and italics alone left
                    // them looking like it.
                    ui.label(egui::RichText::new(why).color(error_colour(ui.visuals())));
                }
            }
        });

    asked
}

impl App {
    /// The scratch directory, made on first use.
    fn scratch_dir(&mut self) -> Result<PathBuf, String> {
        if self.scratch.is_none() {
            let mut builder = tempfile::Builder::new();
            builder.prefix("slipcase-");
            // 0700 asked for rather than assumed. `tempfile`'s directories go
            // through the umask — 0755 under the common one, 0775 under
            // Debian's — so every payload somebody pressed Open on sat in a
            // world-listable directory readable by any account on the machine,
            // for the life of the process. Measured 2026-08-27. `Cargo.toml`
            // said this crate was chosen *because* `TempDir` is 0700; it is
            // not, and that claim is corrected there. `NamedTempFile`, which
            // `slpc::Destination` uses, genuinely is 0600, which is why the two
            // looked alike.
            //
            // Unix only, because `Permissions` has no mode elsewhere. Windows
            // puts `%TEMP%` inside the user's profile with an inherited access
            // list, so the directory is already private to the account; there
            // is nothing to ask for and nothing to assume.
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt as _;
                builder.permissions(std::fs::Permissions::from_mode(0o700));
            }
            let made = builder
                .tempdir()
                .map_err(|e| {
                    fill(
                        t("no temporary directory to extract into: {reason}"),
                        &[("reason", &e.to_string())],
                    )
                })?;
            self.scratch = Some(made);
        }
        // Not `unwrap_or_default`: an empty path here would be a relative one,
        // and extraction would land beside the working directory, which is the
        // one thing DESIGN.md §5 says never to do.
        match &self.scratch {
            Some(dir) => Ok(dir.path().to_owned()),
            None => Err(t("no temporary directory to extract into").to_owned()),
        }
    }
}

impl eframe::App for App {
    // egui 0.36 hands the app a `Ui` rather than a `Context`, and that `Ui`
    // carries no margin or background of its own, so the panel is what gives
    // the window its own. `ui.ctx()` is where the context went. The `Frame` is
    // unused, and keeping the drawing out of this method is what lets a test
    // drive it: a `Frame` belongs to the runner and cannot be made in one.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render(ui);
    }
}

impl App {
    /// A container macOS asked this application to open.
    ///
    /// Polled rather than read once at startup, because unlike `argv[1]` on the
    /// other two platforms this arrives after the window is up, and arrives
    /// again every time somebody double-clicks a container while this is
    /// already running.
    ///
    /// Nothing is taken while a dialog is up, for the reason `poll_picking`
    /// exists: the container waits rather than replacing what a person is in
    /// the middle of choosing, and `taken` consumes, so asking early would lose
    /// it rather than defer it.
    #[cfg(target_os = "macos")]
    fn poll_opened_document(&mut self) {
        if self.busy() {
            return;
        }
        if let Some(path) = opened_document::taken() {
            // The same as choosing it in the dialog, because to a person it is:
            // the next Open should start in the folder they opened this from,
            // whether they reached it through Finder or through the dialog.
            last_folder::write(&path);
            self.show(Opened::open(path));
        }
    }

    fn render(&mut self, ui: &mut egui::Ui) {
        self.poll();
        self.poll_creating();
        self.poll_picking(ui.ctx());
        #[cfg(target_os = "macos")]
        self.poll_opened_document();
        // A copy under way is the one thing here that changes without anybody
        // touching the window, so it is the one thing that has to ask to be
        // drawn again. Both of them: a container being made is a copy too.
        if matches!(self.extraction, Extraction::Running(_))
            || matches!(self.creating, Creating::Running(_))
        {
            ui.ctx().request_repaint();
        }

        // Clicks are read inside the panels and acted on after them, because a
        // panel holds a borrow of the state a click changes.
        let mut asked = None;
        let mut pick_clicked = false;
        let mut new_clicked = false;
        let mut stop_making = false;

        let (undo, redo) = chords(ui);

        // Read before the panels, which borrow the state it asks about.
        let busy = self.busy();

        let pressed = if self.opened.is_some() {
            self.bar(ui)
        } else {
            None
        };
        if matches!(pressed, Some(Pressed::Open)) {
            pick_clicked = true;
        }
        if matches!(pressed, Some(Pressed::New)) {
            new_clicked = true;
        }
        if matches!(pressed, Some(Pressed::StopMaking)) {
            stop_making = true;
        }
        let save_clicked = matches!(pressed, Some(Pressed::Save));
        let undo_clicked = matches!(pressed, Some(Pressed::Undo));
        let redo_clicked = matches!(pressed, Some(Pressed::Redo));

        egui::CentralPanel::default().show(ui, |ui| match &mut self.opened {
            None => match nothing_open(ui, &self.creating, self.said.as_ref(), busy) {
                Some(Pressed::Open) => pick_clicked = true,
                Some(Pressed::New) => new_clicked = true,
                Some(Pressed::StopMaking) => stop_making = true,
                // Nothing else is drawn here: there is no container to save,
                // and no document to take an edit back out of.
                Some(Pressed::Save | Pressed::Undo | Pressed::Redo) | None => {}
            },
            Some(opened) => {
                ui.heading(opened.name());
                ui.label(opened.path.display().to_string());
                ui.separator();
                ui.label(opened.verdict_line());

                if let Some(payload) = &opened.payload {
                    ui.add_space(8.0);
                    asked = card(
                        ui,
                        payload,
                        opened.from_elsewhere,
                        &self.extraction,
                        self.replacing.as_deref(),
                        busy,
                        &mut self.focus_open,
                    );
                }

                // The metadata is the window: it gets the space rather than a
                // panel down one side. DESIGN.md §3.
                if let Some(doc) = &mut opened.metadata {
                    // Before the tree draws, and after the widget has been
                    // told to forget what is half typed: a key field commits
                    // its buffer when it loses focus, and would otherwise
                    // rename the row back after the undo.
                    if undo || undo_clicked {
                        flyleaf::forget_typing(ui.ctx());
                        doc.undo();
                    } else if redo || redo_clicked {
                        flyleaf::forget_typing(ui.ctx());
                        doc.redo();
                    }
                    ui.add_space(8.0);
                    let selected = egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| flyleaf::render(ui, doc.tree_mut(), &RequiredKeys))
                        .inner;
                    // Whatever this frame changed is a step, joined to the
                    // last one where the same row is still being worked in.
                    doc.record(selected.as_deref());
                }
            }
        });

        if pick_clicked {
            self.start_picking(ui.ctx(), For::Container);
        }

        if new_clicked {
            // The first of the two questions. `poll_picking` asks the second
            // once this one is answered, so that the pair stays one dialog at
            // a time like every other.
            self.start_picking(ui.ctx(), For::NewPayload);
        }

        if stop_making {
            if let Creating::Running(job) = &self.creating {
                job.watch.cancel();
            }
        }

        if save_clicked {
            self.save();
        }

        match asked {
            None => {}
            Some(Ask::Open) => self.start_handover(),
            Some(Ask::Extract) => self.start_picking(ui.ctx(), For::ExtractTo),
            Some(Ask::Replace) => self.start_picking(ui.ctx(), For::Replacement),
            Some(Ask::Undo) => {
                self.replacing = None;
                self.said = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{why, App, Ask, Creating, Extraction};
    use slipcase_desktop::{Opened, Payload};
    use slpc::toml_edit::DocumentMut;

    fn payload(unreadable: Option<&str>) -> Payload {
        Payload {
            name: "report.pdf".to_owned(),
            size: 7,
            opens_with: None,
            executable: false,
            unreadable: unreadable.map(ToOwned::to_owned),
        }
    }

    /// A payload this build can decode offers all three actions.
    #[test]
    fn everything_is_offered_for_a_payload_that_can_be_read() {
        let payload = payload(None);
        for ask in [Ask::Open, Ask::Extract, Ask::Replace] {
            assert!(ask.offered(&payload, false));
        }
    }

    /// One it cannot offers only the action that does not need a decoder.
    ///
    /// DESIGN.md §6: a conformant container whose payload is out of reach is
    /// still one whose payload can be replaced, because nothing has to read a
    /// member to write over it.
    #[test]
    fn only_replacing_is_offered_for_a_payload_that_cannot_be_read() {
        let payload = payload(Some("the member is encrypted (SPEC 2.5)"));

        assert!(!Ask::Open.offered(&payload, false), "nothing to hand over");
        assert!(!Ask::Extract.offered(&payload, false), "nothing to write out");
        assert!(
            Ask::Replace.offered(&payload, false),
            "writing over a member does not read it"
        );
    }

    /// A dialog already up takes precedence over all of it.
    #[test]
    fn nothing_is_offered_while_a_dialog_is_up() {
        for unreadable in [None, Some("the member is encrypted (SPEC 2.5)")] {
            let payload = payload(unreadable);
            for ask in [Ask::Open, Ask::Extract, Ask::Replace] {
                assert!(!ask.offered(&payload, true));
            }
        }
    }

    /// The defect this catches is the sentence a person reads after pressing
    /// Open saying *IO error* and nothing else. `opener::OpenError` keeps its
    /// `Display` to a category and puts the platform's own words in `source()`,
    /// so formatting the error threw away the only part worth reading. Found
    /// in the window on 2026-08-26, where a payload the shell will not open
    /// and a cancelled security warning produced the same empty sentence.
    #[test]
    fn a_refused_handover_repeats_what_the_platform_said() {
        let platform = std::io::Error::other("the specified device name is invalid");
        let refused = opener::OpenError::Io(platform);

        assert_eq!(why(&refused), "the specified device name is invalid");
        assert_ne!(
            why(&refused),
            refused.to_string(),
            "the error's own wording is the category, not the reason",
        );
    }

    /// WCAG relative luminance, which is what a contrast ratio is built out of.
    fn luminance(colour: eframe::egui::Color32) -> f32 {
        let channel = |v: u8| {
            let v = f32::from(v) / 255.0;
            if v <= 0.039_28 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * channel(colour.r()) + 0.7152 * channel(colour.g()) + 0.0722 * channel(colour.b())
    }

    fn contrast(text: eframe::egui::Color32, ground: eframe::egui::Color32) -> f32 {
        let (a, b) = (luminance(text), luminance(ground));
        let (lighter, darker) = if a > b { (a, b) } else { (b, a) };
        (lighter + 0.05) / (darker + 0.05)
    }

    /// The defect this catches is the card's coloured lines being unreadable in
    /// the theme nobody looked at. Measured in the window on 2026-08-26: the
    /// provenance line was 2.79:1 on the light card and the failure line
    /// 3.76:1, where ordinary text beside them is 7.59:1, and pure red was
    /// 4.31:1 even on the dark card. WCAG asks 4.5:1 for body text.
    ///
    /// It holds both themes to that bar, because the reason this line is
    /// coloured at all is that a walkthrough found nobody reads a weak grey
    /// one — a colour that cannot be read is the same defect wearing the other
    /// hat.
    ///
    /// The body-text figure is computed here rather than written beside the
    /// bar, because the first pass at this quoted 19.77:1 — pure black on the
    /// light panel, which egui does not draw body text in. Every number in
    /// these comments is now one this test produces, so a figure that drifts
    /// from the code is a failure rather than a sentence nobody re-runs.
    #[test]
    fn the_cards_coloured_lines_can_be_read_in_either_theme() {
        for visuals in [eframe::egui::Visuals::light(), eframe::egui::Visuals::dark()] {
            // `Frame::group`, which the card is drawn in, sets no fill, so the
            // pixel behind these lines is the panel's. Measuring against
            // anything else is how the dark figures came out low the first
            // time.
            let ground = visuals.panel_fill;
            let theme = if visuals.dark_mode { "dark" } else { "light" };

            let warn = contrast(super::warn_colour(&visuals), ground);
            assert!(
                warn >= 4.5,
                "the provenance line is {warn:.2}:1 on the {theme} card, under the 4.5:1 \
                 body-text bar; it is the one line the card colours on purpose"
            );

            let error = contrast(super::error_colour(&visuals), ground);
            assert!(
                error >= 4.5,
                "the failure line is {error:.2}:1 on the {theme} card, under the 4.5:1 \
                 body-text bar"
            );

            // What the card's uncoloured lines measure, which is the comparison
            // the doc comment makes. egui draws them in
            // `noninteractive.fg_stroke`, so that is what is asked.
            let ordinary = contrast(visuals.widgets.noninteractive.fg_stroke.color, ground);
            assert!(
                ordinary >= 4.5,
                "ordinary text on the {theme} card is {ordinary:.2}:1, so the bar the \
                 coloured lines are held to is one the card itself does not clear"
            );
        }
    }

    /// The directory a payload waits in is private, whatever the umask says.
    ///
    /// **The defect this catches published every payload somebody pressed Open
    /// on.** `tempfile` puts its directories through the umask — 0755 under the
    /// common one and 0775 under Debian's — so the handover directory, and the
    /// payload inside it, were readable by every account on the machine for the
    /// life of the process. `Cargo.toml` named this crate's 0700 as the reason
    /// for choosing it; `NamedTempFile` is 0600 and `TempDir` is not, and only
    /// the first had ever been measured.
    ///
    /// Drop the `permissions` call and this fails under any umask but 0077.
    #[test]
    #[cfg(unix)]
    fn the_handover_directory_is_private() {
        use std::os::unix::fs::PermissionsExt as _;

        let mut app = app(None);
        let dir = app.scratch_dir().expect("a scratch directory");
        let mode = std::fs::metadata(&dir).expect("stats").permissions().mode();
        assert_eq!(mode & 0o777, 0o700, "mode was {:o}", mode & 0o777);
    }

    fn app(opened: Option<Opened>) -> App {
        let focus_open = opened.is_some();
        App {
            opened,
            scratch: None,
            extraction: Extraction::Idle,
            creating: Creating::Idle,
            replacing: None,
            said: None,
            picking: None,
            focus_open,
        }
    }

    /// Ctrl+Z takes the last edit back and Ctrl+Shift+Z puts it again,
    /// through the window, with the chord taken before a field could. Found
    /// wanting by hand on 2026-09-07: a comment edited in the tree could not
    /// be taken back, and Ctrl+Z reached only egui's undo of a focused
    /// field's text.
    #[test]
    fn the_undo_and_redo_chords_reach_the_metadata() {
        let dir = tempfile::tempdir().expect("a scratch directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));
        let title = |app: &App| {
            app.opened
                .as_ref()
                .and_then(|o| o.metadata.as_ref())
                .and_then(|d| d.tree()["title"].as_str().map(str::to_owned))
        };
        let before = title(&app).expect("the container has a title");
        {
            let doc = app
                .opened
                .as_mut()
                .and_then(|o| o.metadata.as_mut())
                .expect("a document");
            doc.tree_mut()["title"] = slpc::toml_edit::value("after");
            doc.record(None);
        }
        assert_eq!(title(&app).as_deref(), Some("after"));

        let chord = |modifiers: eframe::egui::Modifiers| eframe::egui::RawInput {
            events: vec![eframe::egui::Event::Key {
                key: eframe::egui::Key::Z,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        };
        let ctx = eframe::egui::Context::default();
        ctx.run_ui(chord(eframe::egui::Modifiers::COMMAND), |ui| app.render(ui))
            .drop_without_applying_deltas();
        assert_eq!(title(&app).as_deref(), Some(before.as_str()), "undone");
        ctx.run_ui(
            chord(eframe::egui::Modifiers::COMMAND | eframe::egui::Modifiers::SHIFT),
            |ui| app.render(ui),
        )
        .drop_without_applying_deltas();
        assert_eq!(title(&app).as_deref(), Some("after"), "redone");
    }

    /// A container this test builds itself, so nothing here needs the
    /// conformance corpus checked out.
    fn a_container(dir: &std::path::Path) -> std::path::PathBuf {
        let path = dir.join("built-by-the-test.slpc");
        let metadata: DocumentMut = "title = \"built by the test\"\n# a comment\n"
            .parse()
            .expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &b"payload"[..], metadata, &mut bytes).expect("packs");
        std::fs::write(&path, &bytes).expect("writes the container");
        path
    }

    /// The state a person sees before they have opened anything. Slice 5 is
    /// where it stopped being a placeholder.
    #[test]
    fn the_empty_state_renders() {
        let mut app = app(None);
        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// The verdict, the card with its Open button, and the tree, all at once.
    #[test]
    fn an_open_container_renders() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));
        assert_eq!(
            app.opened.as_ref().map(Opened::verdict_word),
            Some("accept")
        );
        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// A path that is not a container renders too. DESIGN.md §6 wants a state
    /// rather than a dialog box for every one of these.
    #[test]
    fn a_path_that_is_not_there_renders() {
        let mut app = app(Some(Opened::open("/nonexistent/container.slpc")));
        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// Opening a container forgets what the last one's Open did. Without this,
    /// a message about the container just closed sits under the card of the one
    /// just opened.
    #[test]
    fn opening_a_container_forgets_the_last_one() {
        let mut app = app(None);
        app.extraction = Extraction::Failed("about the container just closed".to_owned());
        app.replacing = Some("/somewhere/for-the-last-one.pdf".into());

        app.show(Opened::open("/nonexistent/container.slpc"));

        assert!(matches!(app.extraction, Extraction::Idle));
        assert_eq!(app.replacing, None, "chosen for the container just closed");
        assert!(app.opened.is_some());
    }

    /// A payload waiting to replace the one in the container is something to
    /// write, so Save has to be on even though nobody typed anything.
    #[test]
    fn a_waiting_replacement_turns_save_on() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));

        let (nothing_to_write, _) = app.notes();
        assert!(!nothing_to_write, "nothing has been asked for yet");

        app.replacing = Some(dir.path().join("report-v2.pdf"));
        let (to_write, _) = app.notes();
        assert!(to_write, "a replacement is an edit to the container");

        // And the card says what will happen, rather than saying nothing until
        // the write has already happened.
        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// A file the specification will not let be a payload is refused where it
    /// was chosen, and no second dialog is opened for a container that was
    /// never going to be written.
    ///
    /// **The defect this catches is asking somebody where to put a container
    /// and then refusing to make it.** `take_new_payload` runs the same check
    /// `take_replacement` does, and it runs it before the second question
    /// rather than inside the packing thread. Break it by moving the check
    /// after `start_picking` and this fails on `creating`, which would be
    /// `Naming` with a file that cannot be a payload in it.
    #[test]
    fn a_file_that_cannot_be_packed_is_refused_before_the_second_dialog() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(None);

        eframe::egui::__run_test_ui(|ui| {
            app.take_new_payload(dir.path().join("slipcase.metadata.toml"), ui.ctx());
        });

        assert!(
            matches!(app.creating, Creating::Idle),
            "a container is still being made out of a file that cannot be a payload"
        );
        assert!(app.picking.is_none(), "a second dialog was opened anyway");
        let (_, said) = app.notes();
        let said = said.expect("it says why");
        assert!(said.wrong, "{}", said.text);
        assert!(said.text.contains("slipcase.metadata.toml"), "{}", said.text);
    }

    /// The row the empty state centres on measures what the buttons in it
    /// measure, under a style that is not the default one.
    ///
    /// **The defect this catches is two buttons drawn against the left edge
    /// under a centred heading**, which is how the empty state looked until
    /// somebody took a screenshot: `vertical_centered` centres a child by the
    /// size that child asked for, and `ui.horizontal` asks for the whole width
    /// it is offered. `row_width` is what the row asks for instead, and it is
    /// only worth having if it agrees with the buttons — a number too small
    /// clips the second button, and one too large puts the pair off centre by
    /// half the error.
    ///
    /// Asked under a changed style as well as the default, because the failure
    /// this guards against is somebody replacing the measurement with a
    /// constant that happens to be right on the machine they wrote it on.
    /// Broken deliberately by writing the default padding in as a number: the
    /// row then asks for 24 where the buttons measure 56.
    ///
    /// **It does not reach the text.** `__run_test_ui` carries no font data —
    /// measured, and a galley for any string comes back zero wide — so what
    /// this compares is the padding and the spacing on both sides while both
    /// labels measure nothing. The half about text was checked by looking at
    /// the window, which is the only place it can be, and `CHECKLIST.md` is
    /// where that run is recorded.
    #[test]
    fn the_row_of_ways_in_measures_what_the_buttons_measure() {
        for padding in [4.0_f32, 12.0] {
            eframe::egui::__run_test_ui(|ui| {
                ui.spacing_mut().button_padding.x = padding;
                let asked = super::row_width(ui, &super::ways_in());
                let drawn = ui
                    .horizontal(|ui| {
                        let mut union: Option<eframe::egui::Rect> = None;
                        for (label, _) in super::ways_in() {
                            let rect = ui.button(label).rect;
                            union = Some(union.map_or(rect, |a| a.union(rect)));
                        }
                        union.expect("a button was drawn")
                    })
                    .inner
                    .width();
                assert!(
                    (asked - drawn).abs() < 0.5,
                    "the row asks for {asked} and the buttons measure {drawn} at padding {padding}"
                );
            });
        }
    }

    /// A container that has been made takes the window, and says so.
    ///
    /// **The defect this catches is a press that answers with silence.**
    /// Making a container is the one long action that can be started with
    /// nothing on screen, so its answer has nowhere to go unless the new
    /// container becomes what the window is holding — which is also what puts
    /// the tree in front of somebody whose container carries nothing but the
    /// two keys the library wrote. Break `poll_creating` by dropping the
    /// `show` and this fails on `opened`.
    #[test]
    fn a_container_that_was_made_takes_the_window() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = a_container(dir.path());
        let mut app = app(None);

        let (sender, outcome) = std::sync::mpsc::channel();
        sender.send(super::Made::Done(path.clone(), None)).expect("sends");
        app.creating = Creating::Running(super::Job {
            watch: slipcase_desktop::Watch::new(),
            total: 0,
            outcome,
        });

        app.poll_creating();

        assert!(matches!(app.creating, Creating::Idle), "still making one");
        assert_eq!(
            app.opened.as_ref().map(|o| o.path.clone()),
            Some(path),
            "the container that was made is not the one on screen"
        );
        let (_, said) = app.notes();
        let said = said.expect("it says what happened");
        assert!(!said.wrong, "{}", said.text);
        // And the empty state is not what draws, so the answer is visible.
        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// Nothing that would replace what is on screen may be pressed while a
    /// container is being made.
    ///
    /// **The defect this catches is two answers arriving in an order nobody
    /// chose.** A container being made ends by calling `show`, exactly as
    /// opening one does, so a person who opens a container while one is being
    /// packed watches it replaced a moment later by a container they asked for
    /// earlier — and any unsaved edit to the one they opened goes with it. The
    /// window already allowed one dialog at a time for the same reason; this
    /// is that rule reaching the other thing that finishes by replacing the
    /// container.
    #[test]
    fn nothing_else_is_offered_while_a_container_is_being_made() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));
        assert!(!app.busy(), "nothing has been asked for yet");

        let (_sender, outcome) = std::sync::mpsc::channel();
        app.creating = Creating::Running(super::Job {
            watch: slipcase_desktop::Watch::new(),
            total: 1,
            outcome,
        });

        assert!(app.busy(), "a container being made is not a reason to wait");
        // The card's three buttons go with it, for the same reason.
        let payload = payload(None);
        for ask in [Ask::Open, Ask::Extract, Ask::Replace] {
            assert!(
                !ask.offered(&payload, app.busy()),
                "a button on the card was still offered"
            );
        }
        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// A file the specification will not let be a payload is refused where it
    /// was chosen, and nothing is left waiting.
    #[test]
    fn a_replacement_that_cannot_be_one_is_refused_at_the_choice() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));

        app.take_replacement(dir.path().join("slipcase.metadata.toml"));

        assert_eq!(app.replacing, None, "nothing is waiting to be written");
        let (to_write, said) = app.notes();
        assert!(!to_write, "and Save stays off");

        let said = said.expect("it says why");
        assert!(said.wrong, "{}", said.text);
        assert!(said.text.contains("slipcase.metadata.toml"), "{}", said.text);
    }

    /// The refusal has a line of its own on the card, drawn before anything is
    /// pressed rather than after something failed.
    #[test]
    fn a_payload_that_cannot_be_read_renders_its_refusal() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));

        let card = app
            .opened
            .as_mut()
            .expect("a container")
            .payload
            .as_mut()
            .expect("a conformant container has a card");
        assert!(card.can_be_decoded(), "the test built a plain container");
        card.unreadable = Some("the member is encrypted (SPEC 2.5)".to_owned());

        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// A container the platform marked as having arrived from elsewhere says so
    /// on the card, and one that was made here does not. The defect this
    /// catches is the line going missing, or worse, appearing on every
    /// container and so meaning nothing.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_container_from_elsewhere_says_so_and_a_local_one_does_not() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = a_container(dir.path());

        let local = Opened::open(&path);
        assert!(
            !local.from_elsewhere,
            "a container the test built is not from elsewhere",
        );

        xattr::set(&path, "user.xdg.origin.url", b"https://example.invalid/a.slpc")
            .expect("marking the container as downloaded");
        let downloaded = Opened::open(&path);
        assert!(
            downloaded.from_elsewhere,
            "a container carrying an origin was not reported as from elsewhere",
        );

        let mut app = app(Some(downloaded));
        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// The Windows counterpart of the test above, which is `cfg`-gated to Linux
    /// because it marks the container with an extended attribute. Until the
    /// provenance walkthrough was run on 2026-08-26 nothing on this platform
    /// covered the card's line at all — the arm compiled and its own tests
    /// never reached the window. The defect it catches is the same one: the
    /// line going missing, or appearing on every container and so meaning
    /// nothing.
    ///
    /// The stream is the shape a browser really writes, checked against 24
    /// downloads on the machine this was written on: `ZoneId=3` first, then the
    /// two URLs.
    #[cfg(target_os = "windows")]
    #[test]
    fn a_container_from_elsewhere_says_so_and_a_local_one_does_not() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = a_container(dir.path());

        let local = Opened::open(&path);
        assert!(
            !local.from_elsewhere,
            "a container the test built is not from elsewhere",
        );

        let mut stream = path.clone().into_os_string();
        stream.push(":Zone.Identifier");
        std::fs::write(
            &stream,
            b"[ZoneTransfer]\r\nZoneId=3\r\nReferrerUrl=https://example.invalid/\r\n\
              HostUrl=https://example.invalid/a.slpc\r\n",
        )
        .expect("marking the container as downloaded");

        let downloaded = Opened::open(&path);
        assert!(
            downloaded.from_elsewhere,
            "a container carrying a zone stream was not reported as from elsewhere",
        );

        let mut app = app(Some(downloaded));
        eframe::egui::__run_test_ui(|ui| app.render(ui));
    }

    /// Opening a container asks for focus on the Open button once, and stops
    /// asking. The defect this catches is the request being made every frame,
    /// which pins the focus to that button and leaves the tree, the Save, and
    /// every other control unreachable from the keyboard — worse than the one
    /// press it was meant to save.
    #[test]
    fn the_open_button_is_focused_once_and_not_every_frame() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));
        assert!(app.focus_open, "a container just shown wants the focus");

        eframe::egui::__run_test_ui(|ui| app.render(ui));
        assert!(
            !app.focus_open,
            "the focus was asked for again on the next frame",
        );
    }

    /// A payload this build cannot decode leaves Open disabled, and a focus
    /// ring on a disabled button says press me about a button that cannot be
    /// pressed. The flag stays up rather than being spent on it.
    #[test]
    fn a_payload_that_cannot_be_read_does_not_take_the_focus() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));

        let card = app
            .opened
            .as_mut()
            .expect("a container")
            .payload
            .as_mut()
            .expect("a conformant container has a card");
        card.unreadable = Some("the member is encrypted (SPEC 2.5)".to_owned());

        eframe::egui::__run_test_ui(|ui| app.render(ui));
        assert!(
            app.focus_open,
            "the focus was spent on a button that cannot be pressed",
        );
    }

    /// One that can be, is.
    #[test]
    fn a_replacement_that_can_be_one_waits_for_a_save() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut app = app(Some(Opened::open(a_container(dir.path()))));

        let chosen = dir.path().join("report-v2.pdf");
        app.take_replacement(chosen.clone());

        assert_eq!(app.replacing.as_deref(), Some(chosen.as_path()));
        let (to_write, said) = app.notes();
        assert!(to_write);
        assert!(said.is_none(), "choosing a file is not a message");
    }
}

#[cfg(all(test, unix))]
mod save_failure_tests {
    use super::{App, Creating, Extraction};
    use slipcase_desktop::{set_value, Opened};
    use slpc::toml_edit::{DocumentMut, Value};
    use std::os::unix::fs::PermissionsExt;

    /// A save that could not happen has to say so.
    ///
    /// It did not: the message was drawn only where the document was not
    /// edited, and a save that fails changes nothing, so the document is still
    /// edited and the mark saying so took the place of the reason. Pressing
    /// Save looked like pressing nothing.
    #[test]
    fn a_save_that_cannot_happen_says_why() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let locked = dir.path().join("locked");
        std::fs::create_dir(&locked).expect("makes a directory");

        let path = locked.join("built-by-the-test.slpc");
        let metadata: DocumentMut = "title = \"before\"\n".parse().expect("valid TOML");
        let mut bytes = Vec::new();
        slpc::pack_reader("report.pdf", &b"payload"[..], metadata, &mut bytes).expect("packs");
        std::fs::write(&path, &bytes).expect("writes");

        // Nothing can be created beside it now, which is what a `Destination`
        // has to do before it can replace anything.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555))
            .expect("locks the directory");

        let mut app = App {
            opened: Some(Opened::open(&path)),
            scratch: None,
            extraction: Extraction::Idle,
            creating: Creating::Idle,
            replacing: None,
            said: None,
            picking: None,
            focus_open: true,
        };

        let document = app
            .opened
            .as_mut()
            .expect("a container")
            .metadata
            .as_mut()
            .expect("a document");
        set_value(
            document.tree_mut()["title"].as_value_mut().expect("a value"),
            Value::from("after"),
        );

        app.save();

        // Both notes, and not one or the other: the document is still edited,
        // and that is the state the reason used to be hidden behind.
        let (edited, said) = app.notes();
        assert!(edited, "a save that failed leaves the document edited");

        let said = said.expect("a save says what it did");
        assert!(said.wrong, "{}", said.text);
        assert!(said.text.starts_with("Not saved"), "{}", said.text);

        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
            .expect("unlocks it so the directory can be removed");
    }
}
