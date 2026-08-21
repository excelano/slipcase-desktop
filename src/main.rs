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
            }))
        }),
    )
}

struct App {
    /// The container named on the command line, if there was one. A dialog and
    /// a drop arrive in slice 5.
    opened: Option<Opened>,
    /// Where extraction goes: a directory of this process's own, made on the
    /// first Open and removed with its contents when this drops. DESIGN.md §5
    /// keeps the application from ever writing beside the container it opened.
    scratch: Option<tempfile::TempDir>,
    /// What the last Open did.
    extraction: Extraction,
    /// What the last Save did, in a sentence.
    save_said: Option<String>,
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

    /// Ask for a container.
    ///
    /// Blocks while the dialog is up, which is what a modal dialog does. The
    /// portal backend is asked through `rfd`'s own blocking call rather than an
    /// executor this application would otherwise not have.
    fn pick() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .set_title("Open a slipcase container")
            .add_filter("slipcase containers", &["slpc"])
            .add_filter("All files", &["*"])
            .pick_file()
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

        // Only the first of several. A single window shows one container, and
        // choosing among a handful dropped together would be a guess.
        let dropped = ui
            .ctx()
            .input(|i| i.raw.dropped_files.first().map(|f| f.path().to_owned()));
        let hovering = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());

        if self.opened.is_some() {
            // egui 0.36 folded `TopBottomPanel` and `SidePanel` into one `Panel`.
            let edited = self.opened.as_ref().is_some_and(Opened::metadata_edited);
            egui::Panel::top("bar").show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Open a container…").clicked() {
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
                    if ui.button("Open a container…").clicked() {
                        pick_clicked = true;
                    }
                    ui.add_space(6.0);
                    // A drop target that gives no sign it takes drops is one
                    // nobody tries.
                    let hint = if hovering {
                        "drop it to open"
                    } else {
                        "or drop one on this window"
                    };
                    ui.label(egui::RichText::new(hint).weak());
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

        // A drop beats the dialog: it names a container outright, and the
        // dialog would still be open in the same frame only by coincidence.
        if let Some(path) = dropped {
            self.show(Opened::open(path));
        } else if pick_clicked {
            if let Some(path) = Self::pick() {
                self.show(Opened::open(path));
            }
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
