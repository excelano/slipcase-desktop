//! Slipcase: a desktop application over the `slpc` library.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]

use std::path::PathBuf;

use eframe::egui;

use slipcase_desktop::{tree, Opened};

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
}

/// What became of an Open.
enum Extraction {
    /// Nothing has been asked for.
    Idle,
    /// The payload is on disk and the platform was handed it.
    Handed(PathBuf),
    /// It could not be extracted, or could not be handed over.
    Failed(String),
}

impl App {
    /// Extract the payload and give it to whatever opens it.
    ///
    /// Synchronous, so a large payload holds the window still while it copies.
    /// Slice 6 moves it off this thread and gives it progress and a cancel.
    fn open_payload(&mut self) -> Extraction {
        let dir = match self.scratch_dir() {
            Ok(dir) => dir,
            Err(why) => return Extraction::Failed(why),
        };
        let Some(opened) = &self.opened else {
            return Extraction::Idle;
        };

        let path = match opened.extract_to(&dir) {
            Ok(path) => path,
            // The library's own wording. An encrypted payload and one
            // compressed by a method this build lacks both arrive here, and
            // both sit in a container that is conformant.
            Err(e) => return Extraction::Failed(e.to_string()),
        };

        match opener::open(&path) {
            Ok(()) => Extraction::Handed(path),
            // Extraction worked and the handover did not, which is a different
            // sentence: the payload is on disk either way.
            Err(e) => Extraction::Failed(format!(
                "{} was extracted, and the system would not open it: {e}",
                path.display()
            )),
        }
    }

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
    // the window its own. `ui.ctx()` is where the context went.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Read inside the panel and acted on after it, because the panel holds
        // a borrow of the state the Open changes.
        let mut open_clicked = false;

        egui::CentralPanel::default().show(ui, |ui| match &self.opened {
            None => {
                ui.heading("Slipcase");
                ui.label("No container open.");
            }
            Some(opened) => {
                ui.heading(opened.name());
                ui.label(opened.path.display().to_string());
                ui.separator();
                ui.label(opened.verdict_line());

                // The payload is a card: its name, its size, and what the
                // platform says would open it. DESIGN.md §3. The Open button
                // arrives in slice 4.
                if let Some(payload) = &opened.payload {
                    ui.add_space(8.0);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new(&payload.name).strong());
                        ui.label(payload.size_line());
                        // Silent where the platform would not answer, rather
                        // than saying it does not know.
                        if let Some(application) = &payload.opens_with {
                            ui.label(format!("Opens with {application}"));
                        }
                        if ui.button("Open").clicked() {
                            open_clicked = true;
                        }
                        match &self.extraction {
                            Extraction::Idle => {}
                            Extraction::Handed(path) => {
                                ui.label(format!("Extracted to {}", path.display()));
                            }
                            Extraction::Failed(why) => {
                                ui.label(egui::RichText::new(why).italics());
                            }
                        }
                    });
                }

                // The metadata is the window: it gets the space rather than a
                // panel down one side. DESIGN.md §3.
                if let Some(doc) = &opened.metadata {
                    ui.add_space(8.0);
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| tree::render(ui, doc));
                }
            }
        });

        if open_clicked {
            self.extraction = self.open_payload();
        }
    }
}
