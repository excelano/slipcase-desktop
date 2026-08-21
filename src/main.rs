//! Slipcase: a desktop application over the `slpc` library.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

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
                replacing: None,
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
                            path.display()
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
    extraction: &Extraction,
    replacing: Option<&std::path::Path>,
    busy: bool,
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
                            // silently dropped.
                            if ui.add_enabled(!busy, egui::Button::new(label)).clicked() {
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
                        &self.extraction,
                        self.replacing.as_deref(),
                        self.picking.is_some(),
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
    use super::{App, Extraction};
    use slipcase_desktop::Opened;
    use slpc::toml_edit::DocumentMut;

    fn app(opened: Option<Opened>) -> App {
        App {
            opened,
            scratch: None,
            extraction: Extraction::Idle,
            replacing: None,
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
