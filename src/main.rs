//! Slipcase: a desktop application over the `slpc` library.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui;

use slipcase_desktop::{extract, tree, Extracted, Opened, Payload, Saved, Watch};

/// The window's identity to the desktop environment.
///
/// A Wayland compositor matches this against the basename of the `.desktop`
/// entry to find the window's icon and its name, so the two have to agree.
/// DESIGN.md §8 installs that entry; this is the half that lives in the binary.
const APP_ID: &str = "slipcase-desktop";

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

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id(APP_ID)
            .with_inner_size([900.0, 640.0]),
        ..Default::default()
    };

    // "Slipcase" rather than the crate name: DESIGN.md §8 puts the product name
    // in front of a person and keeps `slipcase-desktop` on disk and on PATH.
    eframe::run_native(
        "Slipcase",
        options,
        Box::new(move |_cc| {
            Ok(Box::new(App {
                opened,
                scratch: None,
                extraction: Extraction::Idle,
                save_said: None,
                picking: None,
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
    /// What the last Open did.
    extraction: Extraction,
    /// What the last Save did, in a sentence.
    save_said: Option<String>,
    /// A file dialog open on another thread.
    picking: Option<mpsc::Receiver<Option<PathBuf>>>,
}

/// What became of an Open.
enum Extraction {
    /// Nothing has been asked for.
    Idle,
    /// A copy is under way on another thread.
    Running(Job),
    /// The payload is on disk and the platform was handed it.
    Handed(PathBuf),
    /// The copy was stopped, and nothing of it was left behind.
    Cancelled,
    /// It could not be extracted, or could not be handed over.
    Failed(String),
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
        self.save_said = None;
    }

    /// Write the edited metadata back, and show what happened.
    fn save(&mut self) {
        let Some(opened) = &self.opened else {
            return;
        };
        let path = opened.path.clone();
        let outcome = opened.save();

        let said = match &outcome {
            Ok(Saved::Written) => "Saved.".to_owned(),
            Ok(Saved::Unchanged) => "Nothing had changed, so nothing was written.".to_owned(),
            Ok(Saved::Refused(v)) => {
                format!("Not saved. What was written did not read back conformant: {v}")
            }
            Err(e) => format!("Not saved: {e}"),
        };

        if matches!(outcome, Ok(Saved::Written)) {
            // Read back what is now on disk, so the tree, the card, and the
            // edited mark all describe the container rather than the edit.
            self.show(Opened::open(&path));
        }
        self.save_said = Some(said);
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
    /// far along it is and offer to stop. Both the copy and the handover to the
    /// platform happen over there: `opener` starts a process, and starting it
    /// is not instant either.
    fn open_payload(&mut self) -> Extraction {
        let dir = match self.scratch_dir() {
            Ok(dir) => dir,
            Err(why) => return Extraction::Failed(why),
        };
        let Some(opened) = &self.opened else {
            return Extraction::Idle;
        };

        let container = opened.path.clone();
        let total = opened.payload.as_ref().map_or(0, |p| p.size);
        let watch = Watch::new();
        let (sender, outcome) = mpsc::channel();

        let theirs = watch.clone();
        std::thread::spawn(move || {
            let finished = match extract(&container, &dir, &theirs) {
                Ok(Extracted::Done(path)) => match opener::open(&path) {
                    Ok(()) => Extraction::Handed(path),
                    // Extraction worked and the handover did not, which is a
                    // different sentence: the payload is on disk either way.
                    Err(e) => Extraction::Failed(format!(
                        "{} was extracted, and the system would not open it: {e}",
                        path.display()
                    )),
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

        Extraction::Running(Job {
            watch,
            total,
            outcome,
        })
    }

    /// Ask for a container, on a thread of its own.
    ///
    /// Not on this one. The portal backend puts the dialog in another process
    /// entirely, so blocking here does not make the dialog modal, it makes the
    /// window stop answering the compositor, and GNOME offers to force quit an
    /// application that is doing exactly what it was asked to. `rfd` supports
    /// being called from any thread in a windowed application, which this is.
    fn start_picking(&mut self, ctx: &egui::Context) {
        if self.picking.is_some() {
            return;
        }
        let (sender, outcome) = mpsc::channel();
        let ctx = ctx.clone();

        // Where the last one came from, so a second container is found beside
        // the first rather than from wherever the dialog would otherwise start.
        let start_in = last_folder::read();

        std::thread::spawn(move || {
            let mut dialog = rfd::FileDialog::new()
                .set_title("Open a slipcase container")
                .add_filter("slipcase containers", &["slpc"])
                .add_filter("All files", &["*"]);
            if let Some(folder) = start_in {
                dialog = dialog.set_directory(folder);
            }
            let chosen = dialog.pick_file();
            let _ = sender.send(chosen);
            // Nothing has been touching the window while the dialog was up, so
            // it is asleep and has to be woken to notice the answer.
            ctx.request_repaint();
        });

        self.picking = Some(outcome);
    }

    /// Take the dialog's answer, once it has one.
    fn poll_picking(&mut self) {
        let Some(outcome) = &self.picking else {
            return;
        };
        match outcome.try_recv() {
            Err(mpsc::TryRecvError::Empty) => {}
            // A path, or a dialog somebody closed without choosing one.
            Ok(chosen) => {
                self.picking = None;
                if let Some(path) = chosen {
                    last_folder::write(&path);
                    self.show(Opened::open(path));
                }
            }
            Err(mpsc::TryRecvError::Disconnected) => self.picking = None,
        }
    }
}

/// The payload card: what it is, and what can be done with it.
///
/// DESIGN.md §3. Not a method, because the panel drawing it holds the document
/// mutably for the tree and a `&self` here would borrow the whole application
/// alongside it. Returns whether Open was pressed, for the same reason.
fn card(ui: &mut egui::Ui, payload: &Payload, extraction: &Extraction) -> bool {
    let mut open_clicked = false;

    egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.label(egui::RichText::new(&payload.name).strong());
            ui.label(payload.size_line());
            // Silent where the platform would not answer, rather than saying it
            // does not know.
            if let Some(application) = &payload.opens_with {
                ui.label(format!("Opens with {application}"));
            }

            match extraction {
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
                    if ui.button("Open").clicked() {
                        open_clicked = true;
                    }
                }
            }

            match extraction {
                Extraction::Idle | Extraction::Running(_) => {}
                Extraction::Handed(path) => {
                    ui.label(format!("Extracted to {}", path.display()));
                }
                Extraction::Cancelled => {
                    ui.label("Stopped. Nothing was left behind.");
                }
                Extraction::Failed(why) => {
                    ui.label(egui::RichText::new(why).italics());
                }
            }
        });

    open_clicked
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
    fn render(&mut self, ui: &mut egui::Ui) {
        self.poll();
        self.poll_picking();
        // A copy under way is the one thing here that changes without anybody
        // touching the window, so it is the one thing that has to ask to be
        // drawn again.
        if matches!(self.extraction, Extraction::Running(_)) {
            ui.ctx().request_repaint();
        }

        // Clicks are read inside the panels and acted on after them, because a
        // panel holds a borrow of the state a click changes.
        let mut open_clicked = false;
        let mut pick_clicked = false;
        let mut save_clicked = false;

        if self.opened.is_some() {
            // egui 0.36 folded `TopBottomPanel` and `SidePanel` into one `Panel`.
            let edited = self.opened.as_ref().is_some_and(Opened::metadata_edited);
            egui::Panel::top("bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    let picking = self.picking.is_some();
                    if ui
                        .add_enabled(!picking, egui::Button::new("Open a container…"))
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
                    } else if let Some(said) = &self.save_said {
                        ui.label(egui::RichText::new(said).weak());
                    }
                });
            });
        }

        egui::CentralPanel::default().show(ui, |ui| match &mut self.opened {
            None => {
                ui.vertical_centered(|ui| {
                    ui.add_space(72.0);
                    ui.heading("Slipcase");
                    ui.label("Open a .slpc container to see what is in it.");
                    ui.add_space(12.0);
                    if ui
                        .add_enabled(
                            self.picking.is_none(),
                            egui::Button::new("Open a container…"),
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
                    if card(ui, payload, &self.extraction) {
                        open_clicked = true;
                    }
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
            self.start_picking(ui.ctx());
        }

        if save_clicked {
            self.save();
        }

        if open_clicked {
            self.extraction = self.open_payload();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{App, Extraction};
    use slipcase_desktop::Opened;
    use slpc::toml_edit::DocumentMut;

    fn app(opened: Option<Opened>) -> App {
        App {
            opened,
            scratch: None,
            extraction: Extraction::Idle,
            save_said: None,
            picking: None,
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

        app.show(Opened::open("/nonexistent/container.slpc"));

        assert!(matches!(app.extraction, Extraction::Idle));
        assert!(app.opened.is_some());
    }
}
