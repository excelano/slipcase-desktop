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

use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui;

use slipcase_desktop::{
    extract, extract_at, tree, why_not_a_payload, Extracted, Opened, Payload, Saved, Watch,
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

/// The icon at the one size that divides evenly into every size Windows will
/// draw it at.
///
/// A window gets one image and Windows scales it to 16 in the title bar and 32
/// in the task bar, doubling both on a high-density display. 64 is a whole
/// multiple of all four, so every one of them is an integer downsample of the
/// same drawing rather than a resample of a resample.
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
            let _ = std::fs::write(file, folder);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{read_from, write_to};

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
            #[cfg(not(target_os = "macos"))]
            let _ = cc;

            // A container handed over on the command line, or double-clicked,
            // gets the same focus a container opened through the dialog does.
            // Read before the move, because the field initializers below run
            // in the order they are written.
            let focus_open = opened.is_some();

            Ok(Box::new(App {
                opened,
                scratch: None,
                extraction: Extraction::Idle,
                replacing: None,
                save_said: None,
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
    /// What the last Save did.
    save_said: Option<Said>,
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
    Running(Job),
    /// The payload is on disk, here.
    Done(PathBuf),
    /// The copy was stopped, and nothing of it was left behind.
    Cancelled,
    /// It could not be extracted, or could not be handed over.
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

/// A copy under way.
struct Job {
    /// Shared with the thread doing the copying.
    watch: Watch,
    /// What the central directory said the payload measures.
    total: u64,
    /// The one message the thread sends when it is done.
    outcome: mpsc::Receiver<Extraction>,
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
        self.save_said = None;
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
        (edited, self.save_said.as_ref())
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
                text: "Saved.".to_owned(),
                wrong: false,
            },
            Ok(Saved::Unchanged) => Said {
                text: "Nothing had changed, so nothing was written.".to_owned(),
                wrong: false,
            },
            Ok(Saved::Refused(v)) => Said {
                text: format!("Not saved. What was written did not read back conformant: {v}"),
                wrong: true,
            },
            Err(e) => Said {
                text: format!("Not saved: {e}"),
                wrong: true,
            },
        };

        if matches!(outcome, Ok(Saved::Written)) {
            // Read back what is now on disk, so the tree, the card, and the
            // edited mark all describe the container rather than the edit. This
            // is also what clears the replacement: it is in there now.
            self.show(Opened::open(&path));
        }
        self.save_said = Some(said);
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
            self.save_said = Some(Said { text: why, wrong: true });
            return;
        }
        self.replacing = Some(file);
        self.save_said = None;
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
                    Extraction::Failed("the extraction stopped without saying why".to_owned());
            }
        }
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
                        Err(e) => Extraction::Failed(format!(
                            "{} was extracted, and the system would not open it: {e}",
                            slipcase_desktop::shown(&path)
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

        // Where the last container came from, so everything is found beside it
        // rather than from wherever the dialog would otherwise start.
        let start_in = last_folder::read();
        // The payload's own name, offered where the question is what to call
        // the file coming out. Somebody renaming it is choosing to.
        let suggested = match what {
            For::ExtractTo => self
                .opened
                .as_ref()
                .and_then(|o| o.payload.as_ref())
                .map(|p| p.name.clone()),
            For::Container | For::Replacement => None,
        };

        std::thread::spawn(move || {
            let mut dialog = rfd::FileDialog::new();
            dialog = match what {
                For::Container => dialog
                    .set_title("Open a slipcase")
                    .add_filter("slipcases", &["slpc"])
                    .add_filter("All files", &["*"]),
                // No filter on either of these: a payload is any file at all,
                // which is what SPEC §2.3 leaves open.
                For::ExtractTo => dialog.set_title("Extract the payload to"),
                For::Replacement => dialog.set_title("Replace the payload with"),
            };
            if let Some(folder) = start_in {
                dialog = dialog.set_directory(folder);
            }
            if let Some(name) = suggested {
                dialog = dialog.set_file_name(name);
            }
            // A save dialog for the one question that names a file that does
            // not exist yet, so the platform asks before overwriting.
            let chosen = match what {
                For::ExtractTo => dialog.save_file(),
                For::Container | For::Replacement => dialog.pick_file(),
            };
            let _ = sender.send(chosen);
            // Nothing has been touching the window while the dialog was up, so
            // it is asleep and has to be woken to notice the answer.
            ctx.request_repaint();
        });

        self.picking = Some(Picking { what, answer });
    }

    /// Take the dialog's answer, once it has one.
    fn poll_picking(&mut self) {
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
            return;
        };
        match what {
            For::Container => {
                last_folder::write(&path);
                self.show(Opened::open(path));
            }
            For::ExtractTo => self.start_extraction(Target::Chosen(path)),
            For::Replacement => self.take_replacement(path),
        }
    }
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
            ui.label(egui::RichText::new(&payload.name).strong());
            ui.label(payload.size_line());
            // Silent where the platform would not answer, rather than saying it
            // does not know.
            if let Some(application) = &payload.opens_with {
                ui.label(format!("Opens with {application}"));
            }
            // After what the payload is, because both are true at once: the
            // platform would open a file of that name, and this build cannot
            // get the bytes out to give it one.
            if let Some(why) = &payload.unreadable {
                ui.label(
                    egui::RichText::new(format!("Cannot be opened here: {why}"))
                        .color(ui.visuals().error_fg_color),
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
                    egui::RichText::new(
                        "This container arrived from elsewhere, and the payload will carry that.",
                    )
                    .color(ui.visuals().warn_fg_color),
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
                    if ui.button("Cancel").clicked() {
                        job.watch.cancel();
                    }
                }
                _ => {
                    ui.horizontal(|ui| {
                        for (label, ask) in [
                            ("Open", Ask::Open),
                            ("Extract…", Ask::Extract),
                            ("Replace…", Ask::Replace),
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
                        egui::RichText::new(format!(
                            "Will be replaced by {} when this is saved.",
                            file.display()
                        ))
                        .italics(),
                    );
                    // Somewhere to go after choosing the wrong file. Without
                    // this the only way out is closing the container.
                    if ui.button("Undo").clicked() {
                        asked = Some(Ask::Undo);
                    }
                });
            }

            match extraction {
                Extraction::Idle | Extraction::Running(_) => {}
                Extraction::Done(path) => {
                    ui.label(format!("Extracted to {}", slipcase_desktop::shown(path)));
                }
                Extraction::Cancelled => {
                    ui.label("Stopped. Nothing was left behind.");
                }
                Extraction::Failed(why) => {
                    // The same red the bar gives a save that did not happen.
                    // An extraction that failed and one that landed are not
                    // two shades of the same thing, and italics alone left
                    // them looking like it.
                    ui.label(egui::RichText::new(why).color(ui.visuals().error_fg_color));
                }
            }
        });

    asked
}

impl App {
    /// The scratch directory, made on first use.
    fn scratch_dir(&mut self) -> Result<PathBuf, String> {
        if self.scratch.is_none() {
            let made = tempfile::Builder::new()
                .prefix("slipcase-")
                .tempdir()
                .map_err(|e| format!("no temporary directory to extract into: {e}"))?;
            self.scratch = Some(made);
        }
        // Not `unwrap_or_default`: an empty path here would be a relative one,
        // and extraction would land beside the working directory, which is the
        // one thing DESIGN.md §5 says never to do.
        match &self.scratch {
            Some(dir) => Ok(dir.path().to_owned()),
            None => Err("no temporary directory to extract into".to_owned()),
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
        if self.picking.is_some() {
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
        self.poll_picking();
        #[cfg(target_os = "macos")]
        self.poll_opened_document();
        // A copy under way is the one thing here that changes without anybody
        // touching the window, so it is the one thing that has to ask to be
        // drawn again.
        if matches!(self.extraction, Extraction::Running(_)) {
            ui.ctx().request_repaint();
        }

        // Clicks are read inside the panels and acted on after them, because a
        // panel holds a borrow of the state a click changes.
        let mut asked = None;
        let mut pick_clicked = false;
        let mut save_clicked = false;

        if self.opened.is_some() {
            // egui 0.36 folded `TopBottomPanel` and `SidePanel` into one `Panel`.
            let (edited, said) = self.notes();
            egui::Panel::top("bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    let picking = self.picking.is_some();
                    if ui
                        .add_enabled(!picking, egui::Button::new("Open a slipcase…"))
                        .clicked()
                    {
                        pick_clicked = true;
                    }
                    // Off until there is something to write, because DESIGN.md
                    // §5 does not write a container nothing has changed in and
                    // a button that does nothing should not invite a press.
                    if ui
                        .add_enabled(edited, egui::Button::new("Save"))
                        .clicked()
                    {
                        save_clicked = true;
                    }
                    if edited {
                        ui.label(egui::RichText::new("edited").italics().weak());
                    }
                    if let Some(said) = said {
                        let text = egui::RichText::new(&said.text);
                        ui.label(if said.wrong {
                            text.color(ui.visuals().error_fg_color)
                        } else {
                            text.weak()
                        });
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ui, |ui| match &mut self.opened {
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(72.0);
                    ui.heading("Slipcase");
                    ui.label("Open a slipcase to see what is in it.");
                    ui.add_space(12.0);
                    if ui
                        .add_enabled(
                            self.picking.is_none(),
                            egui::Button::new("Open a slipcase…"),
                        )
                        .clicked()
                    {
                        pick_clicked = true;
                    }
                });
            }
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
                        self.picking.is_some(),
                        &mut self.focus_open,
                    );
                }

                // The metadata is the window: it gets the space rather than a
                // panel down one side. DESIGN.md §3.
                if let Some(doc) = &mut opened.metadata {
                    ui.add_space(8.0);
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| tree::render(ui, doc));
                }
            }
        });

        if pick_clicked {
            self.start_picking(ui.ctx(), For::Container);
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
                self.save_said = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Ask, Extraction};
    use slipcase_desktop::{Opened, Payload};
    use slpc::toml_edit::DocumentMut;

    fn payload(unreadable: Option<&str>) -> Payload {
        Payload {
            name: "report.pdf".to_owned(),
            size: 7,
            opens_with: None,
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

    fn app(opened: Option<Opened>) -> App {
        let focus_open = opened.is_some();
        App {
            opened,
            scratch: None,
            extraction: Extraction::Idle,
            replacing: None,
            save_said: None,
            picking: None,
            focus_open,
        }
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
    use super::{App, Extraction};
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
            replacing: None,
            save_said: None,
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
            document["title"].as_value_mut().expect("a value"),
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
