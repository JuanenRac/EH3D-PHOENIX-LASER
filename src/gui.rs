//! The window itself (built with the `gui` feature): it draws the state of `window::Window` and passes on what the person does. Everything that can be decided without a screen is
//! decided there, in code that is tested.

use crate::browse::{self, Browser};
use crate::commonmat;
use crate::lang::{fill, Lang, Msg};
use crate::materials;
use crate::panels::{self, id as pid, Cmd, LayoutWatch, MenuFacts};
use crate::shot::{ShotRequest, ShotTheme, ShotWindow, PATIENCE_FRAMES};
use crate::suitecore;
use crate::suitetext::S;
use crate::suitewin::Step;
use crate::vconfig::{KerfSide, LayerMode, QrEcc, TextAlign};
use crate::window::{fire, Job, Key, Kind, LayerField, Mode, Section, Window, MAX_SEGMENTS};
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Pos2, Rect, Stroke, Vec2,
};
use eh3d_attachpanel::{AttachPanel, Program};
use eh3d_dock::core::AboutInfo;
use eh3d_dock::gui::Dock;
use eh3d_dock::menu::{Chosen, DockAction, MenuBar, StdAction};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

// the colours of the theme Forge, the one the other programs of the suite start with: amber on charcoal
const BACKGROUND: Color32 = Color32::from_rgb(0x0f, 0x0d, 0x0b);
const PANEL: Color32 = Color32::from_rgb(0x17, 0x13, 0x10);
const RAISED: Color32 = Color32::from_rgb(0x22, 0x1c, 0x17);
const TEXT: Color32 = Color32::from_rgb(0xf4, 0xec, 0xe0);
const DIM: Color32 = Color32::from_rgb(0x9c, 0x8d, 0x7a);
const ACCENT: Color32 = Color32::from_rgb(0xff, 0xb0, 0x00);
// amber as text is only for what a person should be careful about; titles and the other words that stand out take the colour of the program (`program_text`)
const CAUTION: Color32 = ACCENT;
const OK: Color32 = Color32::from_rgb(0x9b, 0xe3, 0x4a);
const DANGER: Color32 = Color32::from_rgb(0xff, 0x54, 0x70);
/// The head moving with the laser off, and the edge of the work.
const TRAVEL: Color32 = Color32::from_rgb(0x4a, 0x55, 0x66);
const EDGE: Color32 = Color32::from_rgb(0x4d, 0xd0, 0xe1);
/// A line that is cut or engraved in a drawing (one colour: the power of a drawing is set by layer).
const CUT: Color32 = Color32::from_rgb(0xff, 0x6a, 0x3d);

enum Outcome {
    Cancel,
    Pick(PathBuf),
}

/// The window that opens a file or asks for the name of the file to write.
struct Dialog {
    browser: Browser,
    /// What the file is for.
    job: Job,
    /// The name being typed when the window asks for a file to write.
    export: Option<String>,
}

/// The list of the project that is open on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SuiteList {
    /// The branches and their state.
    Branches,
    /// The files of the other branches, to take one.
    Take,
    /// The branches, to extract one.
    Extract,
}

struct App {
    /// The list of the project that is shown.
    suite_list: Option<SuiteList>,
    /// The window had the keyboard in the last frame (the project is read again when it comes back).
    was_focused: bool,
    win: Window,
    /// The picture mode, while the window is there only to be photographed.
    shot: Option<ShotState>,
    dialog: Option<Dialog>,
    /// The window that asks whether the laser may be turned on (a frame with a pilot power); `Some(true)` when the file is to be written once the answer is yes.
    confirm: Option<bool>,
    fit_pending: bool,
    /// The place of the work under the pointer.
    cursor: Option<(f64, f64)>,
    /// The panel of the settings of the server (the boxes, the state of the link, the login), made the first time the section is drawn.
    server_pane: Option<crate::linkpane::ServerPane>,
    /// The section for the server is held open (the picture of that window, which keeps no settings).
    server_open: bool,
    /// What the program asks of the frame of panels (show a panel, change workspace); the shell does it after the frame.
    pending_dock: Vec<DockAction>,
    /// The files that were opened, for the list of recent files of the frame.
    recent_added: Vec<String>,
    /// The panel of the attachments of the project of the suite.
    attachments: AttachPanel,
}

impl App {
    fn new(lang: Lang, initial: Option<PathBuf>) -> App {
        App::build(lang, initial, true)
    }

    /// The window with its picture; `defaults` is false in the picture mode, which reads no settings file of the folder or of the environment.
    fn build(lang: Lang, initial: Option<PathBuf>, defaults: bool) -> App {
        let mut win = Window::new(lang);
        if defaults {
            win.load_default_config();
        }
        let mut fit_pending = false;
        if let Some(p) = initial {
            if win.open_dropped(&p) {
                if win.can_compute() {
                    win.compute();
                }
                fit_pending = true;
            }
        }
        App {
            suite_list: None,
            was_focused: true,
            win,
            shot: None,
            dialog: None,
            confirm: None,
            fit_pending,
            cursor: None,
            server_pane: None,
            server_open: false,
            pending_dock: Vec::new(),
            recent_added: Vec::new(),
            attachments: AttachPanel::new(Program::new(
                crate::eh3dfile::BRANCH,
                env!("CARGO_PKG_VERSION"),
            )),
        }
    }

    /// The attachments of the project of the window; what it writes is read again by the window.
    fn panel_attachments(&mut self, ui: &mut egui::Ui) {
        let file = self.win.suite.file.clone();
        if self
            .attachments
            .show(ui, file.as_deref(), self.win.lang.code())
        {
            self.win.suite_refresh();
        }
    }

    fn folder(&self) -> PathBuf {
        self.win.work_folder().unwrap_or_else(browse::start_dir)
    }

    /// Opens the window of files for a job: the list of pictures to open, or the name of the file to write.
    fn ask(&mut self, job: Job) {
        let export = job.is_export().then(|| match job {
            Job::SuiteSaveAs => self.win.suite_suggested_name(),
            Job::SuiteExtract => self.win.suite_extract_name(),
            _ => self
                .win
                .suggested_output_for(job)
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "engraving.nc".to_string()),
        });
        self.dialog = Some(Dialog {
            browser: Browser::new(self.folder(), job.extensions()),
            job,
            export,
        });
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                let own = program_text(ui.visuals());
                ui.label(
                    egui::RichText::new("EH3D-PHOENIX-LASER")
                        .strong()
                        .size(18.0)
                        .color(own),
                );
                ui.separator();
                if ui.button(Msg::OpenImage.tr(lang)).clicked() {
                    self.ask(Job::OpenImage);
                }
                if ui.button(Msg::OpenSettings.tr(lang)).clicked() {
                    self.ask(Job::OpenConfig);
                }
                if ui.button(Msg::OpenGcode.tr(lang)).clicked() {
                    self.ask(Job::OpenGcode);
                }
                let can = self.win.can_compute();
                if ui
                    .add_enabled(can, egui::Button::new(Msg::Compute.tr(lang)))
                    .clicked()
                {
                    self.do_compute();
                }
                if ui
                    .add_enabled(
                        self.win.can_export(),
                        egui::Button::new(Msg::ExportGcode.tr(lang)),
                    )
                    .clicked()
                {
                    self.do_export();
                }
                if ui
                    .add_enabled(self.win.has_work(), egui::Button::new(Msg::Fit.tr(lang)))
                    .clicked()
                {
                    self.fit_pending = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("language")
                        .selected_text(self.win.lang.name())
                        .show_ui(ui, |ui| {
                            for l in Lang::ALL {
                                ui.selectable_value(&mut self.win.lang, l, l.name());
                            }
                        });
                    ui.label(egui::RichText::new(Msg::Language.tr(lang)).color(DIM));
                });
            });
        });
    }

    // ------------------------------------------------------------ the panels of the window: each draws what is left of its group

    /// The mode of the work and the settings of the work that mode makes: the picture, the laser, the grid, the framing, the text, the codes, the outline.
    fn panel_cut_settings(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            self.mode_group(ui);
            match self.win.mode {
                Mode::Engrave => {
                    self.dots_group(ui);
                    self.laser_group(ui);
                }
                Mode::Calibrate => self.calibration_group(ui),
                Mode::Focus => self.focus_group(ui),
                Mode::Frame => self.frame_group(ui),
                Mode::Preview => self.program_group(ui),
                Mode::Cut => self.vector_group(ui),
                Mode::Text => self.text_group(ui),
                Mode::Qr => self.qr_group(ui),
                Mode::Marks => self.marks_group(ui),
                Mode::Barcode => self.barcode_group(ui),
                Mode::Trace => {
                    self.trace_group(ui);
                    self.laser_group(ui);
                }
            }
            self.layers_group(ui);
        });
    }

    /// The file that is open and the machine profile.
    fn panel_properties(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            self.picture_group(ui);
            self.machine_group(ui);
        });
    }

    /// The library of materials of the person.
    fn panel_library(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            self.material_group(ui);
        });
    }

    /// The starting points of the common database of materials: a material, a laser, what it does and the thickness give the power, the speed and the passes the database has, with the note that
    /// they were not tried on a machine (always in sight) and the reasons when the material must not meet a laser.
    fn panel_materials(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            if !commonmat::available() {
                ui.label(egui::RichText::new(commonmat::UNAVAILABLE).color(DIM));
                return;
            }
            ui.label(
                egui::RichText::new(commonmat::not_verified_note(lang))
                    .color(CAUTION)
                    .small(),
            );
            let entries = self.win.common_entries();
            let shown = self
                .win
                .common_id
                .as_ref()
                .and_then(|id| entries.iter().find(|e| e.id == *id))
                .map_or_else(|| Msg::NoMaterial.tr(lang).to_string(), |e| e.name.clone());
            egui::ComboBox::from_id_salt("common-material")
                .width(220.0)
                .selected_text(shown)
                .show_ui(ui, |ui| {
                    for e in &entries {
                        let label = if e.laser_unsafe {
                            format!("{} ({})", e.name, Msg::CommonMatNoLaser.tr(lang))
                        } else {
                            e.name.clone()
                        };
                        let current = self.win.common_id.as_deref() == Some(e.id.as_str());
                        if ui.selectable_label(current, label).clicked() {
                            self.win.common_id = Some(e.id.clone());
                        }
                    }
                });
            let mut kind = self.win.common_kind;
            egui::ComboBox::from_id_salt("common-kind")
                .selected_text(match kind {
                    commonmat::Kind::Diode => Msg::CommonMatDiode.tr(lang),
                    commonmat::Kind::Co2 => Msg::CommonMatCo2.tr(lang),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut kind,
                        commonmat::Kind::Diode,
                        Msg::CommonMatDiode.tr(lang),
                    );
                    ui.selectable_value(
                        &mut kind,
                        commonmat::Kind::Co2,
                        Msg::CommonMatCo2.tr(lang),
                    );
                });
            self.win.common_kind = kind;
            let mut what = self.win.common_what;
            egui::ComboBox::from_id_salt("common-what")
                .selected_text(match what {
                    commonmat::What::Cut => Msg::ModeCut.tr(lang),
                    commonmat::What::Engrave => Msg::ModeEngrave.tr(lang),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut what, commonmat::What::Cut, Msg::ModeCut.tr(lang));
                    ui.selectable_value(
                        &mut what,
                        commonmat::What::Engrave,
                        Msg::ModeEngrave.tr(lang),
                    );
                });
            self.win.common_what = what;
            if what == commonmat::What::Cut {
                ui.label(Msg::CommonMatThickness.tr(lang));
                ui.add(
                    egui::TextEdit::singleline(&mut self.win.common_thickness)
                        .desired_width(f32::INFINITY),
                );
            }
            self.field_for(ui, Key::LaserWatts);
            match self.win.common_answer() {
                None => {}
                Some(Err(_)) => {
                    ui.label(
                        egui::RichText::new(Msg::CommonMatNeedWatts.tr(lang))
                            .color(DANGER)
                            .small(),
                    );
                }
                Some(Ok(answer)) => {
                    if let Some(f) = answer.found {
                        ui.label(fill(
                            Msg::CommonMatResult.tr(lang),
                            &[
                                &format!("{}", f.power_percent),
                                &format!("{}", f.speed_mm_min),
                                &f.passes.to_string(),
                            ],
                        ));
                        if let Some(air) = f.air_assist {
                            let on = if air { Msg::StateOn } else { Msg::StateOff };
                            ui.label(fill(Msg::CommonMatAir.tr(lang), &[on.tr(lang)]));
                        }
                        for note in &answer.notes {
                            ui.label(egui::RichText::new(note.as_str()).color(CAUTION).small());
                        }
                        if ui.button(Msg::CommonMatUse.tr(lang)).clicked() {
                            self.win.use_common_start();
                        }
                    }
                    if let Some(w) = &answer.warning {
                        ui.label(egui::RichText::new(w.as_str()).color(DANGER));
                    }
                    for r in &answer.reasons {
                        ui.label(egui::RichText::new(r.as_str()).color(DANGER).small());
                    }
                }
            }
        });
    }

    /// Framing the work with the laser off or with the pilot of low power, and the limits of the machine.
    fn panel_jog(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            if self.win.mode != Mode::Frame && ui.button(Msg::ModeFrame.tr(lang)).clicked() {
                self.win.set_mode(Mode::Frame);
            }
            self.frame_group(ui);
        });
    }

    /// The server: the settings of the link, the job that was sent, pause, resume and stop.
    fn panel_machine(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            self.server_group(ui);
        });
    }

    /// What the program does and the check of the program.
    fn panel_simulation(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            if self.win.preview_lines().is_empty() {
                ui.label(egui::RichText::new(Msg::NoFile.tr(lang)).color(DIM));
            }
            self.preview_group(ui);
        });
    }

    /// The program: what it is made from, working it out and writing it.
    fn panel_job(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        self.win.can_compute(),
                        egui::Button::new(Msg::Compute.tr(lang)),
                    )
                    .clicked()
                {
                    self.do_compute();
                }
                if ui
                    .add_enabled(
                        self.win.can_export(),
                        egui::Button::new(Msg::ExportGcode.tr(lang)),
                    )
                    .clicked()
                {
                    self.do_export();
                }
                if ui
                    .add_enabled(self.win.has_work(), egui::Button::new(Msg::Fit.tr(lang)))
                    .clicked()
                {
                    self.fit_pending = true;
                }
            });
            if let Some(done) = &self.win.done {
                if let Some((name, hash)) = &done.source {
                    ui.label(egui::RichText::new(name.as_str()).strong());
                    ui.label(
                        egui::RichText::new(format!("SHA-256 {hash}"))
                            .color(DIM)
                            .small(),
                    );
                }
                ui.label(egui::RichText::new(done.tools.as_str()).color(DIM).small());
            }
            let (text, problem) = if self.win.stale {
                (Msg::Stale.tr(lang).to_string(), true)
            } else {
                (self.win.status.text(lang), self.win.status.is_problem())
            };
            ui.label(egui::RichText::new(text).color(if problem { DANGER } else { OK }));
            if let Some(info) = self.win.info_line() {
                ui.label(info);
            }
        });
    }

    /// The report written to be read before a job.
    fn panel_report(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            if ui
                .add_enabled(
                    self.win.done.is_some(),
                    egui::Button::new(Msg::ReportMake.tr(lang)),
                )
                .clicked()
            {
                self.win.make_report();
            }
            match &self.win.report_text {
                Some(text) => {
                    let mut shown: &str = text.as_str();
                    ui.add(
                        egui::TextEdit::multiline(&mut shown)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY),
                    );
                }
                None => {
                    ui.label(egui::RichText::new(Msg::PanelNoReport.tr(lang)).color(DIM));
                }
            }
        });
    }

    /// What the program has to say: the state, the notes of the reading and the lines of the server.
    fn panel_console(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::ScrollArea::vertical().show(ui, |ui| {
            let (text, problem) = if self.win.stale {
                (Msg::Stale.tr(lang).to_string(), true)
            } else {
                (self.win.status.text(lang), self.win.status.is_problem())
            };
            ui.label(egui::RichText::new(text).color(if problem { DANGER } else { OK }));
            for line in self.win.check_lines() {
                ui.label(egui::RichText::new(line).small());
            }
            for line in self.win.server_lines() {
                ui.label(egui::RichText::new(line).small());
            }
            self.notes_group(ui);
        });
    }

    /// Works the program out; the laser is not lit until the person has said so.
    fn do_compute(&mut self) {
        if self.win.needs_confirmation() {
            self.confirm = Some(false);
        } else if self.win.compute() {
            self.fit_pending = true;
        }
    }

    /// Asks for the file to write the program to; the laser is not lit until the person has said so.
    fn do_export(&mut self) {
        if self.win.needs_confirmation() {
            self.confirm = Some(true);
        } else {
            self.ask(Job::ExportGcode);
        }
    }

    /// The things that are always in sight, in every workspace and outside the panels: the stop of the job and the state of the interlocks.
    fn safety_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::bottom("safety").show(ui, |ui| {
            ui.horizontal(|ui| {
                let stop = egui::Button::new(
                    egui::RichText::new(Msg::ServerStop.tr(lang))
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(DANGER);
                if ui.add_enabled(self.win.server_active(), stop).clicked() {
                    self.win.server_stop();
                }
                let text = egui::RichText::new(self.win.safety_line());
                ui.label(if self.win.safety_problem() {
                    text.color(DANGER)
                } else {
                    text.color(DIM)
                });
            });
        });
    }

    /// What the menus need to know to turn entries off.
    fn menu_facts(&self) -> MenuFacts {
        MenuFacts {
            has_work: self.win.has_work(),
            can_compute: self.win.can_compute(),
            can_export: self.win.can_export(),
            has_suite: self.win.suite.file.is_some(),
            has_done: self.win.done.is_some(),
            can_send: self.win.can_send(),
            server_active: self.win.server_active(),
        }
    }

    /// Opens a file by what it is (a file of settings, a picture, a drawing, a program) and works the result out.
    fn open_path(&mut self, path: &Path) {
        let is_settings = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("cfg"));
        let ok = if is_settings {
            let ok = self.win.run_job(Job::OpenConfig, path);
            if ok && self.win.vector.is_some() {
                self.win.compute();
            }
            ok
        } else if self.win.open_dropped(path) {
            if self.win.can_compute() {
                self.win.compute();
            }
            self.fit_pending = true;
            true
        } else {
            false
        };
        if ok {
            self.recent_added.push(path.display().to_string());
        }
    }

    /// Does what a menu entry says.
    fn run_cmd(&mut self, cmd: Cmd, ctx: &egui::Context) {
        match cmd {
            Cmd::Std(StdAction::Open) | Cmd::OpenImage => self.ask(Job::OpenImage),
            Cmd::Std(StdAction::Export) | Cmd::ExportGcode => {
                if self.win.can_export() {
                    self.do_export();
                }
            }
            Cmd::Std(StdAction::Quit) => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Cmd::Std(StdAction::Preferences) => self
                .pending_dock
                .push(DockAction::ShowPanel(pid::CUT_SETTINGS.to_string())),
            // the rest of the standard entries are for things this program does not have, and they are turned off
            Cmd::Std(_) => {}
            Cmd::OpenSettings => self.ask(Job::OpenConfig),
            Cmd::OpenGcode => self.ask(Job::OpenGcode),
            Cmd::OpenMaterials => self.ask(Job::OpenMaterials),
            Cmd::NewMaterials => self.ask(Job::NewMaterials),
            Cmd::SuiteOpen => self.ask(Job::SuiteOpen),
            Cmd::SuiteSave => {
                if self.win.suite_save() == Step::NeedsName {
                    self.ask(Job::SuiteSaveAs);
                }
            }
            Cmd::SuiteSaveAs => self.ask(Job::SuiteSaveAs),
            Cmd::SuiteImport => self.ask(Job::SuiteImport),
            Cmd::SuiteTake => self.show_suite_list(SuiteList::Take),
            Cmd::SuiteExtract => self.show_suite_list(SuiteList::Extract),
            Cmd::SuiteBranches => self.show_suite_list(SuiteList::Branches),
            Cmd::Compute => {
                if self.win.can_compute() {
                    self.do_compute();
                }
            }
            Cmd::Fit => self.fit_pending = true,
            Cmd::MakeReport => {
                self.win.make_report();
            }
            Cmd::SendToServer => {
                if self.win.can_send() && !self.win.server_active() {
                    self.win.send_to_server();
                }
            }
            Cmd::EmergencyStop => self.win.server_stop(),
        }
    }

    fn show_suite_list(&mut self, list: SuiteList) {
        self.win.suite_refresh();
        self.suite_list = Some(list);
    }

    /// The project of the suite stays in sight: its name, what is out of date, what a write would unsign and what was last done.
    fn suite_bar(&mut self, ui: &mut egui::Ui) {
        let lines = self.win.suite_banner();
        if lines.is_empty() {
            return;
        }
        egui::Panel::top("suite").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (text, attention) in lines {
                    let text = egui::RichText::new(text);
                    ui.label(if attention {
                        text.strong().color(DANGER)
                    } else {
                        text.color(DIM)
                    });
                    ui.separator();
                }
            });
        });
    }

    /// The list of the branches, or of the files to take, of the project.
    fn suite_list_window(&mut self, ctx: &egui::Context) {
        let Some(list) = self.suite_list else {
            return;
        };
        let (Some(view), code) = (self.win.suite.view.clone(), self.win.lang.code()) else {
            self.suite_list = None;
            return;
        };
        let title = match list {
            SuiteList::Branches => S::MenuBranches,
            SuiteList::Take => S::MenuTake,
            SuiteList::Extract => S::MenuExtract,
        };
        let mut open = true;
        let mut take = None;
        let mut extract = None;
        let mut again = false;
        egui::Window::new(title.tr(code))
            .id(egui::Id::unique("suite-list"))
            .open(&mut open)
            .collapsible(false)
            .default_size([620.0, 360.0])
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| match list {
                        SuiteList::Branches => {
                            egui::Grid::new("suite-branches")
                                .num_columns(5)
                                .striped(true)
                                .show(ui, |ui| {
                                    for word in [
                                        S::ColBranch,
                                        S::ColApp,
                                        S::ColFormat,
                                        S::ColFiles,
                                        S::ColState,
                                    ] {
                                        let own = program_text(ui.visuals());
                                        ui.label(egui::RichText::new(word.tr(code)).color(own));
                                    }
                                    ui.end_row();
                                    for b in &view.branches {
                                        ui.label(b.id.as_str());
                                        ui.label(b.app.as_str());
                                        ui.label(b.format.as_str());
                                        ui.label(b.files.to_string());
                                        let state = suitecore::state_text(b, code);
                                        let state = egui::RichText::new(state);
                                        ui.label(if b.stale.is_empty() {
                                            state
                                        } else {
                                            state.color(DANGER)
                                        });
                                        ui.end_row();
                                    }
                                });
                        }
                        SuiteList::Take => {
                            if view.takeable.is_empty() {
                                ui.label(S::NothingToTake.tr(code));
                            } else {
                                ui.label(S::ChooseFile.tr(code));
                            }
                            for (branch, name) in &view.takeable {
                                if ui
                                    .selectable_label(false, format!("{branch}/{name}"))
                                    .clicked()
                                {
                                    take = Some((branch.clone(), name.clone()));
                                }
                            }
                        }
                        SuiteList::Extract => {
                            ui.label(S::ChooseBranch.tr(code));
                            for b in &view.branches {
                                if ui.selectable_label(false, b.id.as_str()).clicked() {
                                    extract = Some(b.id.clone());
                                }
                            }
                        }
                    });
                ui.separator();
                if ui.button(S::ReadAgain.tr(code)).clicked() {
                    again = true;
                }
            });
        if again {
            self.win.suite_refresh();
        }
        if !open {
            self.suite_list = None;
        }
        if let Some((branch, name)) = take {
            self.suite_list = None;
            if self.win.suite_take(&branch, &name) == Step::Done {
                self.after_suite();
            }
        }
        if let Some(branch) = extract {
            self.suite_list = None;
            self.win.suite.extract_branch = branch;
            self.ask(Job::SuiteExtract);
        }
    }

    /// The question before a project is written when there is something to agree to.
    fn suite_confirm_window(&mut self, ctx: &egui::Context) {
        let Some(pending) = &self.win.suite.pending else {
            return;
        };
        let code = self.win.lang.code();
        let lines: Vec<String> = pending.warnings.iter().map(|w| w.text(code)).collect();
        let cancel = Msg::Cancel.tr(self.win.lang);
        let mut answer = None;
        egui::Window::new(S::ConfirmTitle.tr(code))
            .id(egui::Id::unique("suite-confirm"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                for line in &lines {
                    ui.label(egui::RichText::new(line.as_str()).color(DANGER));
                }
                ui.horizontal(|ui| {
                    if ui.button(S::Write.tr(code)).clicked() {
                        answer = Some(true);
                    }
                    if ui.button(cancel).clicked() {
                        answer = Some(false);
                    }
                });
            });
        match answer {
            Some(true) => {
                if self.win.suite_confirm() == Step::Done {
                    self.after_suite();
                }
            }
            Some(false) => self.win.suite_cancel(),
            None => {}
        }
    }

    /// The work that came from a project is worked out and looked at.
    fn after_suite(&mut self) {
        if let Some(file) = &self.win.suite.file {
            self.recent_added.push(file.display().to_string());
        }
        if self.win.done.is_none() && self.win.can_compute() && !self.win.needs_confirmation() {
            // the line that says what was done with the project stays
            let said = self.win.status.clone();
            if self.win.compute() {
                self.win.status = said;
                self.fit_pending = true;
            }
        }
    }

    /// What is done at the start of a frame: a file dropped on the window is opened, and a project is read again when the window
    /// gets the keyboard back, so what another program changed is told.
    fn frame_start(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|i| i.focused);
        if focused && !self.was_focused {
            self.win.suite_refresh();
        }
        self.was_focused = focused;
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if let Some(p) = dropped.first() {
            self.open_path(p);
        }
    }

    /// The title of a group of a panel.
    fn heading(&self, ui: &egui::Ui, msg: Msg) -> egui::RichText {
        egui::RichText::new(msg.tr(self.win.lang))
            .strong()
            .color(program_text(ui.visuals()))
    }

    /// The settings of one group, each with its name above a box to type in. A setting that does not matter with the choices made is left out.
    fn fields_of(&mut self, ui: &mut egui::Ui, section: Section) {
        let lang = self.win.lang;
        for i in 0..self.win.fields.len() {
            if self.win.fields[i].section != section {
                continue;
            }
            match self.win.fields[i].key {
                Key::Overscan if self.win.overscan_auto => continue,
                // the single power is not used while the grid of powers is
                Key::FocusPower if self.win.focus_powers_given() => continue,
                Key::Accel if !self.win.overscan_auto => continue,
                Key::Threshold if self.win.kind != Kind::Threshold => continue,
                // the grid is made from the two lists, or from a material and its ladder
                Key::Powers | Key::Feeds if self.win.cal_material.is_some() => continue,
                Key::CalSteps | Key::CalSpread | Key::CalMinPower | Key::CalMinFeed
                    if self.win.cal_material.is_none() =>
                {
                    continue
                }
                Key::CalLabelMm if self.win.cal_material.is_none() || !self.win.cal_labels => {
                    continue
                }
                _ => {}
            }
            let label = self.win.fields[i].label.tr(lang);
            let mut text = self.win.fields[i].text.clone();
            ui.label(label);
            let edit = egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY);
            if ui.add(edit).changed() {
                self.win.set_field(i, text);
            }
        }
    }

    /// One setting by its key, with its name above a box to type in.
    fn field_for(&mut self, ui: &mut egui::Ui, key: Key) {
        let lang = self.win.lang;
        if let Some(i) = self.win.fields.iter().position(|f| f.key == key) {
            let label = self.win.fields[i].label.tr(lang);
            let mut text = self.win.fields[i].text.clone();
            ui.label(label);
            let edit = egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY);
            if ui.add(edit).changed() {
                self.win.set_field(i, text);
            }
        }
    }

    fn picture_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionFile))
            .default_open(true)
            .show(ui, |ui| {
                if ui.button(Msg::OpenImage.tr(lang)).clicked() {
                    self.ask(Job::OpenImage);
                }
                let shown = self
                    .win
                    .current_name()
                    .unwrap_or_else(|| Msg::NoFile.tr(lang).to_string());
                ui.label(egui::RichText::new(shown).color(DIM));
                if let Some(info) = self.win.info_line() {
                    ui.label(info);
                }
                ui.label(
                    egui::RichText::new(Msg::Formats.tr(lang))
                        .color(DIM)
                        .small(),
                );
            });
    }

    fn mode_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionMode))
            .default_open(true)
            .show(ui, |ui| {
                let mut mode = self.win.mode;
                let shown = match mode {
                    Mode::Engrave => Msg::ModeEngrave,
                    Mode::Calibrate => Msg::ModeCalibrate,
                    Mode::Cut => Msg::ModeCut,
                    Mode::Focus => Msg::ModeFocus,
                    Mode::Frame => Msg::ModeFrame,
                    Mode::Preview => Msg::ModePreview,
                    Mode::Trace => Msg::ModeTrace,
                    Mode::Text => Msg::ModeText,
                    Mode::Qr => Msg::ModeQr,
                    Mode::Marks => Msg::ModeMarks,
                    Mode::Barcode => Msg::ModeBarcode,
                };
                egui::ComboBox::from_id_salt("mode")
                    .selected_text(shown.tr(lang))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut mode, Mode::Engrave, Msg::ModeEngrave.tr(lang));
                        ui.selectable_value(
                            &mut mode,
                            Mode::Calibrate,
                            Msg::ModeCalibrate.tr(lang),
                        );
                        ui.selectable_value(&mut mode, Mode::Cut, Msg::ModeCut.tr(lang));
                        ui.selectable_value(&mut mode, Mode::Focus, Msg::ModeFocus.tr(lang));
                        ui.selectable_value(&mut mode, Mode::Frame, Msg::ModeFrame.tr(lang));
                        ui.selectable_value(&mut mode, Mode::Preview, Msg::ModePreview.tr(lang));
                        ui.selectable_value(&mut mode, Mode::Trace, Msg::ModeTrace.tr(lang));
                        ui.selectable_value(&mut mode, Mode::Text, Msg::ModeText.tr(lang));
                        ui.selectable_value(&mut mode, Mode::Qr, Msg::ModeQr.tr(lang));
                        ui.selectable_value(&mut mode, Mode::Marks, Msg::ModeMarks.tr(lang));
                        ui.selectable_value(&mut mode, Mode::Barcode, Msg::ModeBarcode.tr(lang));
                    });
                if mode != self.win.mode {
                    self.win.set_mode(mode);
                }
            });
    }

    /// The machine profiles of the settings file: the list to take one from, which sets the full-power value and the limits of the boxes. Nothing is shown when the file has none.
    fn machine_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        let names = self.win.profile_names();
        if names.is_empty() {
            return;
        }
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionMachine))
            .default_open(true)
            .show(ui, |ui| {
                let shown = self
                    .win
                    .settings
                    .profile
                    .clone()
                    .unwrap_or_else(|| Msg::NoProfile.tr(lang).to_string());
                let mut chosen: Option<String> = None;
                egui::ComboBox::from_id_salt("profile")
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for n in &names {
                            let current = self.win.settings.profile.as_deref() == Some(n.as_str());
                            if ui.selectable_label(current, n.as_str()).clicked() {
                                chosen = Some(n.clone());
                            }
                        }
                    });
                if let Some(n) = chosen {
                    self.win.apply_profile(&n);
                }
            });
    }

    fn dots_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionImage))
            .default_open(true)
            .show(ui, |ui| {
                self.fields_of(ui, Section::Image);
                ui.label(Msg::Dither.tr(lang));
                let mut kind = self.win.kind;
                egui::ComboBox::from_id_salt("dither")
                    .selected_text(kind.label().tr(lang))
                    .show_ui(ui, |ui| {
                        for k in Kind::ALL {
                            ui.selectable_value(&mut kind, k, k.label().tr(lang));
                        }
                    });
                if kind != self.win.kind {
                    self.win.set_kind(kind);
                }
                let mut invert = self.win.tone_invert;
                if ui.checkbox(&mut invert, Msg::Invert.tr(lang)).changed() {
                    self.win.set_tone_invert(invert);
                }
                let mut level = self.win.tone_auto_level;
                if ui.checkbox(&mut level, Msg::AutoLevel.tr(lang)).changed() {
                    self.win.set_tone_auto_level(level);
                }
            });
    }

    /// The library of materials: the file to load it from and the list to take a material from, which fills the boxes of the window.
    fn material_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionMaterial))
            .default_open(true)
            .show(ui, |ui| {
                if ui.button(Msg::OpenMaterials.tr(lang)).clicked() {
                    self.ask(Job::OpenMaterials);
                }
                if ui.button(Msg::MaterialNewFile.tr(lang)).clicked() {
                    self.ask(Job::NewMaterials);
                }
                let names = self.win.material_names();
                if !names.is_empty() {
                    let shown = self
                        .win
                        .material
                        .clone()
                        .unwrap_or_else(|| Msg::NoMaterial.tr(lang).to_string());
                    let mut chosen: Option<String> = None;
                    egui::ComboBox::from_id_salt("material")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for n in &names {
                                let current = self.win.material.as_deref() == Some(n.as_str());
                                if ui.selectable_label(current, n.as_str()).clicked() {
                                    chosen = Some(n.clone());
                                }
                            }
                        });
                    if let Some(n) = chosen {
                        self.win.apply_material(&n);
                    }
                    // what is done with the material that is picked: edit it, copy it or delete it from the file (the second press confirms)
                    if let Some(current) = self.win.material.clone() {
                        ui.horizontal(|ui| {
                            if ui.button(Msg::MaterialEdit.tr(lang)).clicked() {
                                self.win.material_edit(&current);
                            }
                            if ui.button(Msg::MaterialCopy.tr(lang)).clicked() {
                                self.win.material_copy(&current);
                            }
                            if ui.button(Msg::MaterialDelete.tr(lang)).clicked() {
                                self.win.material_delete(&current);
                            }
                        });
                    }
                }
                if ui.button(Msg::MaterialNewFromBoxes.tr(lang)).clicked() {
                    self.win.material_new_from_boxes();
                }
                self.material_editor(ui);
            });
    }

    /// The material that is being edited: its name, its note and one box for each value (the keys of the file; an empty box leaves the value out), and the buttons that save it in the file
    /// or let it go.
    fn material_editor(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        let Some(draft) = self.win.material_draft.clone() else {
            return;
        };
        ui.separator();
        ui.label(egui::RichText::new(Msg::MaterialEditor.tr(lang)).strong());
        ui.label(Msg::MaterialName.tr(lang));
        let mut name = draft.name.clone();
        if ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(f32::INFINITY))
            .changed()
        {
            self.win.set_draft_name(name);
        }
        ui.label(Msg::MaterialNote.tr(lang));
        let mut note = draft.note.clone();
        if ui
            .add(egui::TextEdit::singleline(&mut note).desired_width(f32::INFINITY))
            .changed()
        {
            self.win.set_draft_note(note);
        }
        let labels = crate::lang::material_labels();
        for (i, key) in materials::KEYS.iter().enumerate() {
            ui.label(format!("{} ({key})", labels[i].tr(lang)));
            let mut text = draft.values.get(i).cloned().unwrap_or_default();
            if ui
                .add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY))
                .changed()
            {
                self.win.set_draft_value(i, text);
            }
        }
        ui.horizontal(|ui| {
            if ui.button(Msg::MaterialSave.tr(lang)).clicked() {
                self.win.material_save();
            }
            if ui.button(Msg::Cancel.tr(lang)).clicked() {
                self.win.material_cancel();
            }
        });
    }

    /// The outline of the picture: the width of the work, how the ink is told from the paper, how much the line is simplified and whether the shapes are cut or filled with lines.
    fn trace_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionTrace))
            .default_open(true)
            .show(ui, |ui| {
                self.field_for(ui, Key::WidthMm);
                self.fields_of(ui, Section::Trace);
                let mut ink = self.win.trace_invert;
                if ui.checkbox(&mut ink, Msg::TraceInk.tr(lang)).changed() {
                    self.win.set_trace_invert(ink);
                }
                let mut fill = self.win.trace_engrave;
                if ui.checkbox(&mut fill, Msg::TraceEngrave.tr(lang)).changed() {
                    self.win.set_trace_engrave(fill);
                }
            });
    }

    fn laser_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionLaser))
            .default_open(true)
            .show(ui, |ui| {
                let mut auto = self.win.overscan_auto;
                if ui.checkbox(&mut auto, Msg::OverscanAuto.tr(lang)).changed() {
                    self.win.set_overscan_auto(auto);
                }
                self.fields_of(ui, Section::Laser);
            });
    }

    fn calibration_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionCalibration))
            .default_open(true)
            .show(ui, |ui| {
                // the grid from a material of the library: its power and its speed are the middle of the ladder
                let names = self.win.material_names();
                if !names.is_empty() {
                    ui.label(Msg::CalMaterial.tr(lang));
                    let shown = self
                        .win
                        .cal_material
                        .clone()
                        .unwrap_or_else(|| Msg::NoMaterial.tr(lang).to_string());
                    let mut chosen: Option<Option<String>> = None;
                    egui::ComboBox::from_id_salt("cal-material")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            let none = self.win.cal_material.is_none();
                            if ui
                                .selectable_label(none, Msg::NoMaterial.tr(lang))
                                .clicked()
                            {
                                chosen = Some(None);
                            }
                            for n in &names {
                                let current = self.win.cal_material.as_deref() == Some(n.as_str());
                                if ui.selectable_label(current, n.as_str()).clicked() {
                                    chosen = Some(Some(n.clone()));
                                }
                            }
                        });
                    if let Some(c) = chosen {
                        self.win.set_cal_material(c.as_deref());
                    }
                    if self.win.cal_material.is_some() {
                        let mut on = self.win.cal_labels;
                        if ui.checkbox(&mut on, Msg::CalLabels.tr(lang)).changed() {
                            self.win.set_cal_labels(on);
                        }
                    }
                }
                self.fields_of(ui, Section::Calibration);
                // the full power value and the limits are the same settings as the ones of the laser
                for key in [Key::SMax, Key::MaxPower, Key::MaxSpeed, Key::LaserWatts] {
                    self.field_for(ui, key);
                }
            });
    }

    /// The frame of the work: the margin, the laps, the speed and the pilot power. The laser stays off with a pilot power of 0; with more, the window asks first.
    fn frame_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionFrame))
            .default_open(true)
            .show(ui, |ui| {
                self.fields_of(ui, Section::Frame);
                for key in [Key::MaxPower, Key::MaxSpeed] {
                    self.field_for(ui, key);
                }
                if self.win.pilot_percent().is_some_and(|p| p > 0.0) {
                    ui.label(
                        egui::RichText::new(Msg::NeedConfirm.tr(lang))
                            .color(DANGER)
                            .small(),
                    );
                }
            });
    }

    /// The program that was opened to be seen: the file, and the full power and the acceleration its time and its warnings are worked out with.
    fn program_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::ModePreview))
            .default_open(true)
            .show(ui, |ui| {
                if ui.button(Msg::OpenGcode.tr(lang)).clicked() {
                    self.ask(Job::OpenGcode);
                }
                for key in [Key::SMax, Key::Accel, Key::LaserWatts, Key::VerifyOverscan] {
                    self.field_for(ui, key);
                }
            });
    }

    /// What the preview says about the G-code that was worked out or opened: what it does and what looks dangerous.
    fn preview_group(&mut self, ui: &mut egui::Ui) {
        let lines = self.win.preview_lines();
        if lines.is_empty() {
            return;
        }
        let check = self.win.check_lines();
        let energy = self.win.energy_line();
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionPreview))
            .default_open(true)
            .show(ui, |ui| {
                for (k, line) in lines.iter().enumerate() {
                    let text = egui::RichText::new(line.as_str());
                    ui.label(if k == 0 {
                        text
                    } else {
                        text.color(DANGER).small()
                    });
                }
                // the energy and the mean power, when the power of the laser is known
                if let Some(energy) = &energy {
                    ui.label(egui::RichText::new(energy.as_str()));
                }
                // the check of the program: green when it passes, red with what is wrong when it does not
                let passes = self.win.check.as_ref().is_some_and(|v| v.passes());
                for (k, line) in check.iter().enumerate() {
                    let text = egui::RichText::new(line.as_str());
                    ui.label(match (passes, k) {
                        (true, _) => text.color(egui::Color32::from_rgb(0x4c, 0xc3, 0x7a)),
                        (false, 0) => text.color(DANGER),
                        (false, _) => text.color(DANGER).small(),
                    });
                }
            });
    }

    /// The EH3D server: the panel of the suite (the address, the token, the machine, the limits of the link, the state of the link and the login with a name and a password), then the buttons that
    /// send the program of the window as a job, follow it, hold it and stop it with the emergency stop. It is open when the settings give a server or a job is followed.
    fn server_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        let lines = self.win.server_lines();
        let mut pane = self.server_pane.take().unwrap_or_else(|| {
            crate::linkpane::ServerPane::new(&self.win.settings.server, self.win.login.clone())
        });
        pane.refresh();
        let view_lang =
            eh3d_view::texts::Lang::from_code(lang.code()).unwrap_or(eh3d_view::texts::Lang::En);
        let mut header = egui::CollapsingHeader::new(self.heading(ui, Msg::SectionServer))
            .default_open(self.win.server_configured() || !lines.is_empty());
        if self.server_open {
            header = header.open(Some(true));
        }
        header.show(ui, |ui| {
            if let Some(action) = eh3d_view::server_gui::show(ui, &mut pane.panel, view_lang) {
                pane.handle(action, &mut self.win.settings.server);
            }
            ui.separator();
            let busy = self.win.server_active();
            // a program that was opened to be seen is sent as it is only when the person says so (and only after it passes the check)
            if self.win.mode == Mode::Preview {
                ui.checkbox(
                    &mut self.win.send_opened_confirmed,
                    Msg::ServerConfirmOpened.tr(lang),
                );
            }
            if ui
                .add_enabled(
                    self.win.can_send() && !busy,
                    egui::Button::new(Msg::ServerSend.tr(lang)),
                )
                .clicked()
            {
                self.win.send_to_server();
            }
            for line in &lines {
                ui.label(egui::RichText::new(line.as_str()));
            }
            if busy {
                ui.horizontal(|ui| {
                    let held = self.win.server_held();
                    let label = if held {
                        Msg::ServerResume
                    } else {
                        Msg::ServerPause
                    };
                    if ui.button(label.tr(lang)).clicked() {
                        if held {
                            self.win.server_resume();
                        } else {
                            self.win.server_pause();
                        }
                    }
                    let stop = egui::RichText::new(Msg::ServerStop.tr(lang)).color(DANGER);
                    if ui.button(stop).clicked() {
                        self.win.server_stop();
                    }
                });
            }
        });
        self.server_pane = Some(pane);
        // the job is asked after again a moment later, so that the window shows how it goes without a click
        if self.win.server_active() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(300));
        }
    }

    /// The question before the laser is turned on: the work is not made, and nothing is written, until the answer is yes.
    fn confirm_window(&mut self, ctx: &egui::Context) {
        let Some(then_write) = self.confirm else {
            return;
        };
        let lang = self.win.lang;
        let power = self.win.pilot_percent().unwrap_or(0.0).to_string();
        let mut answer: Option<bool> = None;
        egui::Window::new(Msg::ConfirmTitle.tr(lang))
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(fill(Msg::ConfirmText.tr(lang), &[&power])).color(DANGER),
                );
                ui.horizontal(|ui| {
                    if ui.button(Msg::ConfirmYes.tr(lang)).clicked() {
                        answer = Some(true);
                    }
                    if ui.button(Msg::Cancel.tr(lang)).clicked() {
                        answer = Some(false);
                    }
                });
            });
        match answer {
            Some(true) => {
                self.confirm = None;
                self.win.pilot_confirmed = true;
                if self.win.compute() {
                    self.fit_pending = true;
                    if then_write {
                        self.ask(Job::ExportGcode);
                    }
                }
            }
            Some(false) => self.confirm = None,
            None => {}
        }
    }

    /// The focus test: the lines, the heights and the power and speed they are burnt with. The step of the height has no default, and the test moves Z, which the person is told.
    fn focus_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::ModeFocus))
            .default_open(true)
            .show(ui, |ui| {
                self.fields_of(ui, Section::Focus);
                for key in [Key::SMax, Key::MaxPower, Key::MaxSpeed, Key::LaserWatts] {
                    self.field_for(ui, key);
                }
                ui.label(
                    egui::RichText::new(Msg::FocusWarning.tr(lang))
                        .color(DANGER)
                        .small(),
                );
            });
    }

    /// The text mode: the words to burn in a box of their own, then the height, the gaps, the power, the speed, the passes and the limits that are the keys of `[text]`.
    fn text_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::ModeText))
            .default_open(true)
            .show(ui, |ui| {
                ui.label(Msg::TextWords.tr(lang));
                let mut words = self.win.words.clone();
                let edit = egui::TextEdit::multiline(&mut words)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY);
                if ui.add(edit).changed() {
                    self.win.set_words(words);
                }
                self.fields_of(ui, Section::Text);
                let mut mirror = self.win.text_mirror;
                if ui.checkbox(&mut mirror, Msg::Mirror.tr(lang)).changed() {
                    self.win.set_text_mirror(mirror);
                }
                ui.label(Msg::TextAlign.tr(lang));
                let name = |a: TextAlign| {
                    let msg = match a {
                        TextAlign::Left => Msg::AlignLeft,
                        TextAlign::Centre => Msg::AlignCentre,
                        TextAlign::Right => Msg::AlignRight,
                    };
                    msg.tr(lang)
                };
                let mut align = self.win.text_align;
                egui::ComboBox::from_id_salt("text-align")
                    .selected_text(name(align))
                    .show_ui(ui, |ui| {
                        for a in [TextAlign::Left, TextAlign::Centre, TextAlign::Right] {
                            ui.selectable_value(&mut align, a, name(a));
                        }
                    });
                if align != self.win.text_align {
                    self.win.set_text_align(align);
                }
                for key in [
                    Key::SMax,
                    Key::OriginX,
                    Key::OriginY,
                    Key::MaxPower,
                    Key::MaxSpeed,
                    Key::LaserWatts,
                ] {
                    self.field_for(ui, key);
                }
            });
    }

    /// The QR mode: the data to burn, the level of error correction and the keys of `[qr]` (the side of a module, the distance between the lines, the power, the speed, the passes and the limits).
    fn qr_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::ModeQr))
            .default_open(true)
            .show(ui, |ui| {
                ui.label(Msg::QrData.tr(lang));
                let mut data = self.win.qr_data.clone();
                let edit = egui::TextEdit::multiline(&mut data)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY);
                if ui.add(edit).changed() {
                    self.win.set_qr_data(data);
                }
                ui.label(Msg::QrEccLabel.tr(lang));
                let mut ecc = self.win.qr_ecc;
                egui::ComboBox::from_id_salt("qr-ecc")
                    .selected_text(ecc.word())
                    .show_ui(ui, |ui| {
                        for e in [QrEcc::L, QrEcc::M, QrEcc::Q, QrEcc::H] {
                            ui.selectable_value(&mut ecc, e, e.word());
                        }
                    });
                if ecc != self.win.qr_ecc {
                    self.win.set_qr_ecc(ecc);
                }
                let mut mirror = self.win.qr_mirror;
                if ui.checkbox(&mut mirror, Msg::Mirror.tr(lang)).changed() {
                    self.win.set_qr_mirror(mirror);
                }
                let mut invert = self.win.qr_invert;
                if ui.checkbox(&mut invert, Msg::QrInvert.tr(lang)).changed() {
                    self.win.set_qr_invert(invert);
                }
                self.fields_of(ui, Section::Qr);
                for key in [
                    Key::SMax,
                    Key::OriginX,
                    Key::OriginY,
                    Key::MaxPower,
                    Key::MaxSpeed,
                    Key::LaserWatts,
                ] {
                    self.field_for(ui, key);
                }
            });
    }

    /// The barcode mode: the data to burn, the switch of the mirror image and the keys of `[barcode]` (the width of the narrowest bar, the height, the distance between the lines, the power, the speed, the passes and the limits).
    fn barcode_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::ModeBarcode))
            .default_open(true)
            .show(ui, |ui| {
                ui.label(Msg::BarcodeData.tr(lang));
                let mut data = self.win.barcode_data.clone();
                let edit = egui::TextEdit::singleline(&mut data).desired_width(f32::INFINITY);
                if ui.add(edit).changed() {
                    self.win.set_barcode_data(data);
                }
                let mut mirror = self.win.barcode_mirror;
                if ui.checkbox(&mut mirror, Msg::Mirror.tr(lang)).changed() {
                    self.win.set_barcode_mirror(mirror);
                }
                self.fields_of(ui, Section::Barcode);
                for key in [
                    Key::SMax,
                    Key::OriginX,
                    Key::OriginY,
                    Key::MaxPower,
                    Key::MaxSpeed,
                    Key::LaserWatts,
                ] {
                    self.field_for(ui, key);
                }
            });
    }

    /// The marks mode: the size of the rectangle, the cross in the middle and the keys of `[marks]` (the arm of an L, the margin, the power, the speed and the passes).
    fn marks_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::CollapsingHeader::new(self.heading(ui, Msg::ModeMarks))
            .default_open(true)
            .show(ui, |ui| {
                self.fields_of(ui, Section::Marks);
                if ui
                    .add_enabled(
                        self.win.framed_mode().is_some(),
                        egui::Button::new(Msg::MarksFromJob.tr(lang)),
                    )
                    .clicked()
                {
                    self.win.marks_from_job();
                }
                let mut cross = self.win.marks_cross;
                if ui.checkbox(&mut cross, Msg::MarksCross.tr(lang)).changed() {
                    self.win.set_marks_cross(cross);
                }
                for key in [
                    Key::SMax,
                    Key::OriginX,
                    Key::OriginY,
                    Key::MaxPower,
                    Key::MaxSpeed,
                    Key::LaserWatts,
                ] {
                    self.field_for(ui, key);
                }
            });
    }

    /// The settings that are not of one layer: how the file is read and the machine.
    fn vector_group(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionVector))
            .default_open(true)
            .show(ui, |ui| {
                self.fields_of(ui, Section::Vector);
                for key in [
                    Key::SMax,
                    Key::Overscan,
                    Key::OriginX,
                    Key::OriginY,
                    Key::MaxPower,
                    Key::MaxSpeed,
                    Key::LaserWatts,
                ] {
                    self.field_for(ui, key);
                }
            });
    }

    fn notes_group(&mut self, ui: &mut egui::Ui) {
        if self.win.warnings.is_empty() {
            return;
        }
        egui::CollapsingHeader::new(self.heading(ui, Msg::SectionNotes))
            .default_open(true)
            .show(ui, |ui| {
                for note in self.win.warnings.iter().take(20) {
                    ui.label(egui::RichText::new(note.as_str()).color(DIM).small());
                }
            });
    }

    /// The table of layers (or colours) of the drawing: what is done with each and with which settings.
    fn panel_layers(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        if self.win.mode != Mode::Cut || self.win.rows.is_empty() {
            ui.label(egui::RichText::new(Msg::PanelNoLayers.tr(lang)).color(DIM));
            return;
        }
        ui.scope(|ui| {
            ui.label(self.heading(ui, Msg::SectionLayers));
            egui::ScrollArea::both()
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    egui::Grid::new("layers_grid")
                        .striped(true)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            let heads = [
                                Msg::LayerName,
                                Msg::LayerModeHead,
                                Msg::LayerPower,
                                Msg::LayerSpeed,
                                Msg::Passes,
                                Msg::FocusStep,
                                Msg::Kerf,
                                Msg::KerfSide,
                                Msg::Tabs,
                                Msg::TabWidth,
                                Msg::HatchAngle,
                                Msg::HatchSpacing,
                                Msg::HatchCross,
                            ];
                            for h in heads {
                                let own = program_text(ui.visuals());
                                ui.label(egui::RichText::new(h.tr(lang)).strong().color(own));
                            }
                            ui.end_row();
                            for r in 0..self.win.rows.len() {
                                self.layer_row(ui, r);
                                ui.end_row();
                            }
                        });
                });
        });
    }

    /// The cells of one row of the table of layers.
    fn layer_row(&mut self, ui: &mut egui::Ui, r: usize) {
        let lang = self.win.lang;
        let key = self.win.rows[r].key.clone();
        let shapes = self.win.rows[r].shapes;
        ui.horizontal(|ui| {
            if let Some(color) = color_of_key(&key) {
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(12.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 2.0, color);
            }
            ui.label(format!("{key} ({shapes})"));
        });
        let mut mode = self.win.rows[r].mode;
        egui::ComboBox::from_id_salt(("layer-mode", r))
            .width(80.0)
            .selected_text(mode_word(mode).tr(lang))
            .show_ui(ui, |ui| {
                for m in [LayerMode::Cut, LayerMode::Engrave, LayerMode::Skip] {
                    ui.selectable_value(&mut mode, m, mode_word(m).tr(lang));
                }
            });
        if mode != self.win.rows[r].mode {
            self.win.set_layer_mode(r, mode);
        }
        for f in [
            LayerField::Power,
            LayerField::Speed,
            LayerField::Passes,
            LayerField::Focus,
            LayerField::Kerf,
        ] {
            self.layer_box(ui, r, f);
        }
        let mut side = self.win.rows[r].kerf_side;
        egui::ComboBox::from_id_salt(("layer-side", r))
            .width(80.0)
            .selected_text(side_word(side).tr(lang))
            .show_ui(ui, |ui| {
                for k in [
                    KerfSide::None,
                    KerfSide::Auto,
                    KerfSide::Inside,
                    KerfSide::Outside,
                ] {
                    ui.selectable_value(&mut side, k, side_word(k).tr(lang));
                }
            });
        if side != self.win.rows[r].kerf_side {
            self.win.set_kerf_side(r, side);
        }
        for f in [
            LayerField::Tabs,
            LayerField::TabWidth,
            LayerField::HatchAngle,
            LayerField::HatchSpacing,
        ] {
            self.layer_box(ui, r, f);
        }
        let mut cross = self.win.rows[r].hatch_cross;
        if ui.checkbox(&mut cross, "").changed() {
            self.win.set_layer_cross(r, cross);
        }
    }

    fn layer_box(&mut self, ui: &mut egui::Ui, r: usize, f: LayerField) {
        let mut text = self.win.rows[r].fields[f as usize].clone();
        if ui
            .add(egui::TextEdit::singleline(&mut text).desired_width(56.0))
            .changed()
        {
            self.win.set_layer_field(r, f, text);
        }
    }

    fn layers_group(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        ui.add_space(6.0);
        ui.checkbox(&mut self.win.show_travel, Msg::LegendTravel.tr(lang));
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                let (text, problem) = if self.win.stale {
                    (Msg::Stale.tr(lang).to_string(), true)
                } else {
                    (self.win.status.text(lang), self.win.status.is_problem())
                };
                ui.label(egui::RichText::new(text).color(if problem { DANGER } else { OK }));
                if self.win.truncated {
                    let note = fill(Msg::Truncated.tr(lang), &[&MAX_SEGMENTS.to_string()]);
                    ui.label(egui::RichText::new(note).color(DIM));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(Msg::ZoomHint.tr(lang)).color(DIM));
                    if let Some((x, y)) = self.cursor {
                        ui.separator();
                        ui.label(egui::RichText::new(self.win.cursor_text(x, y)).color(TEXT));
                    }
                });
            });
        });
    }

    fn panel_view(&mut self, ui: &mut egui::Ui) {
        let lang = self.win.lang;
        ui.scope(|ui| {
            let size = ui.available_size();
            let (response, painter) = ui.allocate_painter(size, egui::Sense::click_and_drag());
            let rect = response.rect;
            let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
            painter.rect_filled(rect, 0.0, BACKGROUND);
            self.cursor = None;
            if !self.win.has_work() && self.win.mode == Mode::Engrave {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    Msg::DropHint.tr(lang),
                    egui::FontId::proportional(18.0),
                    DIM,
                );
                return;
            }
            if self.fit_pending && w > 40.0 && h > 40.0 {
                self.win.fit(w, h);
                self.fit_pending = false;
            }
            if response.double_clicked() {
                self.fit_pending = true;
            }
            if response.dragged() {
                let d = response.drag_delta();
                self.win.viewport.pan_px(f64::from(d.x), f64::from(d.y));
            }
            if let Some(p) = response.hover_pos() {
                let at = p - rect.min;
                let (sx, sy) = (f64::from(at.x), f64::from(at.y));
                let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
                let factor = (f64::from(scroll) * 0.0015).exp() * f64::from(pinch);
                if (factor - 1.0).abs() > 1e-9 {
                    self.win.viewport.zoom_at(sx, sy, w, h, factor);
                }
                self.cursor = Some(self.win.viewport.to_world(sx, sy, w, h));
            }
            self.paint(&painter, rect);
        });
    }

    fn paint(&self, painter: &egui::Painter, rect: Rect) {
        let vp = self.win.viewport;
        let (w, h) = (f64::from(rect.width()), f64::from(rect.height()));
        let at = |x: f64, y: f64| -> Pos2 {
            let (px, py) = vp.to_screen(x, y, w, h);
            Pos2::new(rect.min.x + px as f32, rect.min.y + py as f32)
        };
        let painter = painter.with_clip_rect(rect);
        // the edge of the work
        if let Some((min, max)) = self.win.bounds() {
            let corners = vec![
                at(min.0, min.1),
                at(max.0, min.1),
                at(max.0, max.1),
                at(min.0, max.1),
            ];
            painter.add(egui::Shape::closed_line(corners, Stroke::new(1.0, EDGE)));
        }
        let line_mm = self.win.done.as_ref().map_or(0.1, |d| d.line_mm);
        let burn_width = ((line_mm * vp.px_per_mm) as f32).max(1.2);
        let drawing = matches!(self.win.mode, Mode::Cut | Mode::Trace);
        if self.win.show_travel {
            for s in self.win.segs.iter().filter(|s| !s.burn) {
                painter.line_segment([at(s.x0, s.y0), at(s.x1, s.y1)], Stroke::new(0.8, TRAVEL));
            }
        }
        for s in self.win.segs.iter().filter(|s| s.burn) {
            let [r, g, b] = fire(s.level);
            // a drawing is cut in one colour: its power is set by layer, not by the line
            let color = if drawing {
                CUT
            } else {
                Color32::from_rgb(r, g, b)
            };
            painter.line_segment(
                [at(s.x0, s.y0), at(s.x1, s.y1)],
                Stroke::new(burn_width, color),
            );
        }
        self.paint_key(&painter, rect);
    }

    /// The key of the colours, in the corner of the picture.
    fn paint_key(&self, painter: &egui::Painter, rect: Rect) {
        if self.win.segs.is_empty() {
            return;
        }
        let lang = self.win.lang;
        let mut y = rect.min.y + 12.0;
        if matches!(self.win.mode, Mode::Cut | Mode::Trace) {
            painter.rect_filled(
                Rect::from_min_size(Pos2::new(rect.min.x + 12.0, y), Vec2::new(24.0, 6.0)),
                0.0,
                CUT,
            );
            painter.text(
                Pos2::new(rect.min.x + 44.0, y + 3.0),
                egui::Align2::LEFT_CENTER,
                Msg::LegendCut.tr(lang),
                egui::FontId::proportional(13.0),
                TEXT,
            );
            return;
        }
        painter.text(
            Pos2::new(rect.min.x + 12.0, y + 7.0),
            egui::Align2::LEFT_CENTER,
            Msg::LegendBurn.tr(lang),
            egui::FontId::proportional(13.0),
            TEXT,
        );
        y += 20.0;
        let steps: u32 = 32;
        let bar = 160.0 / steps as f32;
        for k in 0..steps {
            let [r, g, b] = fire(f64::from(k) / f64::from(steps - 1));
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.min.x + 12.0 + bar * k as f32, y),
                    Vec2::new(bar + 0.5, 12.0),
                ),
                0.0,
                Color32::from_rgb(r, g, b),
            );
        }
        painter.text(
            Pos2::new(rect.min.x + 12.0, y + 24.0),
            egui::Align2::LEFT_CENTER,
            Msg::LegendLow.tr(lang),
            egui::FontId::proportional(12.0),
            DIM,
        );
        painter.text(
            Pos2::new(rect.min.x + 172.0, y + 24.0),
            egui::Align2::RIGHT_CENTER,
            Msg::LegendHigh.tr(lang),
            egui::FontId::proportional(12.0),
            DIM,
        );
    }

    fn dialog_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        match show_dialog(ctx, &mut dialog, self.win.lang) {
            None => self.dialog = Some(dialog),
            Some(Outcome::Cancel) => {}
            Some(Outcome::Pick(path)) => {
                if self.win.run_job(dialog.job, &path) {
                    match dialog.job {
                        Job::SuiteOpen | Job::SuiteSaveAs | Job::SuiteImport => {
                            self.after_suite();
                        }
                        Job::SuiteExtract => {}
                        Job::OpenImage => {
                            // the work of the new file is worked out at once
                            self.win.compute();
                            self.fit_pending = true;
                        }
                        Job::OpenConfig => {
                            if self.win.vector.is_some() {
                                self.win.compute();
                            }
                        }
                        Job::OpenGcode => self.fit_pending = true,
                        Job::OpenMaterials | Job::NewMaterials | Job::ExportGcode => {}
                    }
                } else {
                    // the window stays open so that another file or name can be tried
                    self.dialog = Some(dialog);
                }
            }
        }
    }
}

/// The word of the table for what is done with a layer.
fn mode_word(m: LayerMode) -> Msg {
    match m {
        LayerMode::Cut => Msg::WordCut,
        LayerMode::Engrave => Msg::WordEngrave,
        LayerMode::Skip => Msg::WordSkip,
    }
}

/// The word of the table for the side of the line the beam goes.
fn side_word(k: KerfSide) -> Msg {
    match k {
        KerfSide::None => Msg::WordNone,
        KerfSide::Auto => Msg::WordAuto,
        KerfSide::Inside => Msg::WordInside,
        KerfSide::Outside => Msg::WordOutside,
    }
}

/// The colour a layer named `#rrggbb` is drawn with, for the little square of the table.
fn color_of_key(key: &str) -> Option<Color32> {
    let hex = key.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some(Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// Draws the window that opens a file (or asks for the name of the file to write) and says what the person chose.
fn show_dialog(ctx: &egui::Context, d: &mut Dialog, lang: Lang) -> Option<Outcome> {
    let mut outcome = None;
    let mut open = true;
    let title = d
        .job
        .suite_title()
        .map_or_else(|| d.job.title().tr(lang), |s| s.tr(lang.code()));
    let accept = match d.job {
        Job::SuiteSaveAs => S::Write.tr(lang.code()),
        Job::SuiteExtract => S::Extract.tr(lang.code()),
        _ => Msg::Export.tr(lang),
    };
    egui::Window::new(title)
        .open(&mut open)
        .collapsible(false)
        .default_size([620.0, 460.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(Msg::Up.tr(lang)).clicked() {
                    d.browser.up();
                }
                let edit = egui::TextEdit::singleline(&mut d.browser.path_text)
                    .desired_width(ui.available_width());
                let response = ui.add(edit);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Some(p) = d.browser.go_typed() {
                        if d.export.is_none() {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                }
            });
            if let Some(e) = &d.browser.error {
                ui.colored_label(DANGER, e.as_str());
            }
            if ui
                .checkbox(&mut d.browser.show_all, Msg::AllFiles.tr(lang))
                .changed()
            {
                d.browser.refresh();
            }
            egui::ScrollArea::vertical()
                .max_height(300.0)
                .show(ui, |ui| {
                    if d.browser.entries.is_empty() {
                        ui.label(egui::RichText::new(Msg::EmptyFolder.tr(lang)).color(DIM));
                    }
                    for e in d.browser.entries.clone() {
                        let label = if e.is_dir {
                            format!("{}/", e.name)
                        } else {
                            format!("{}    ({})", e.name, browse::size_text(e.size))
                        };
                        if !ui.selectable_label(false, label).clicked() {
                            continue;
                        }
                        if e.is_dir {
                            d.browser.enter(&e.name);
                        } else if d.export.is_some() {
                            d.export = Some(e.name.clone());
                        } else if let Some(p) = d.browser.enter(&e.name) {
                            outcome = Some(Outcome::Pick(p));
                        }
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                if let Some(name) = d.export.as_mut() {
                    ui.label(Msg::FileName.tr(lang));
                    ui.add(egui::TextEdit::singleline(name).desired_width(260.0));
                    if ui.button(accept).clicked() && !name.trim().is_empty() {
                        outcome = Some(Outcome::Pick(d.browser.dir.join(name.trim())));
                    }
                }
                if ui.button(Msg::Cancel.tr(lang)).clicked() {
                    outcome = Some(Outcome::Cancel);
                }
            });
        });
    if !open && outcome.is_none() {
        outcome = Some(Outcome::Cancel);
    }
    outcome
}

/// How a panel of the window is drawn: with the program and the room the frame gives it.
type PanelDraw = fn(&mut App, &mut egui::Ui);

/// How often the arrangement of the panels is compared with what was written, in frames.
const LAYOUT_CHECK_EVERY: u32 = 20;

/// The window: the frame of the suite with the panels of the program in it, the menu bar above, and the bars of the toolbar, the status and the safety (the stop and the interlocks) around
/// it, which are never part of a panel.
struct Shell {
    app: Rc<RefCell<App>>,
    dock: Dock,
    bar: MenuBar<Cmd>,
    watch: LayoutWatch,
    /// How many frames were drawn, so the arrangement is looked at a few times a second and not at every frame.
    ticks: u32,
}

impl Shell {
    fn new(app: App) -> Shell {
        let lang = app.win.lang;
        let source = app.win.layout_source();
        let app = Rc::new(RefCell::new(app));
        let mut dock = Dock::new(panels::defs(), panels::workspaces());
        let draw: [(&str, PanelDraw); 13] = [
            (pid::VIEW, App::panel_view),
            (pid::LAYERS, App::panel_layers),
            (pid::CUT_SETTINGS, App::panel_cut_settings),
            (pid::PROPERTIES, App::panel_properties),
            (pid::LIBRARY, App::panel_library),
            (pid::MATERIALS, App::panel_materials),
            (pid::JOG, App::panel_jog),
            (pid::MACHINE, App::panel_machine),
            (pid::SIMULATION, App::panel_simulation),
            (pid::JOB, App::panel_job),
            (pid::REPORT, App::panel_report),
            (pid::CONSOLE, App::panel_console),
            (pid::ATTACHMENTS, App::panel_attachments),
        ];
        for (name, f) in draw {
            let a = app.clone();
            dock.register(name, move |ui| f(&mut a.borrow_mut(), ui));
        }
        dock.load_config(&source);
        dock.set_language(view_lang(lang));
        let watch = LayoutWatch::new(&dock.core().config_pairs());
        Shell {
            app,
            dock,
            bar: panels::menu_bar(),
            watch,
            ticks: 0,
        }
    }

    /// What the program says about itself in the window «About».
    fn about() -> AboutInfo {
        AboutInfo {
            app_name: "EH3D-PHOENIX-LASER".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            license: "GPL-3.0-or-later".to_string(),
            extra: Vec::new(),
        }
    }

    /// Does what the program asked of the frame (show a panel, change workspace).
    fn flush_dock(&mut self) {
        let actions = std::mem::take(&mut self.app.borrow_mut().pending_dock);
        for action in actions {
            let _ = self.dock.apply(action);
        }
    }

    /// Does what was chosen in the menus, and keeps the list of recent files.
    fn after(&mut self, chosen: Option<Chosen<Cmd>>, ctx: &egui::Context) {
        {
            let mut app = self.app.borrow_mut();
            match chosen {
                Some(Chosen::App(cmd)) => app.run_cmd(cmd, ctx),
                Some(Chosen::Recent(path)) => app.open_path(Path::new(&path)),
                Some(Chosen::Dock(_)) | None => {}
            }
        }
        self.flush_dock();
        let added = std::mem::take(&mut self.app.borrow_mut().recent_added);
        for path in added {
            self.dock.core_mut().recent_mut().push(&path);
        }
    }
}

/// The language of the window for the frame.
fn view_lang(lang: Lang) -> eh3d_view::texts::Lang {
    eh3d_view::texts::Lang::from_code(lang.code()).unwrap_or(eh3d_view::texts::Lang::En)
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.flush_dock();
        let (lang, facts) = {
            let mut app = self.app.borrow_mut();
            app.frame_start(&ctx);
            (app.win.lang, app.menu_facts())
        };
        self.dock.set_language(view_lang(lang));
        self.dock.set_about(Shell::about());
        self.dock.set_brand(panels::about_brand());
        self.dock
            .set_menu_state(panels::disabled_entries(&facts), Vec::new());
        // the menu bar, then the bars that no workspace can hide, then the frame with the panels
        let mut chosen = None;
        egui::Panel::top("menu").show(ui, |ui| {
            chosen = self.dock.menu_bar(ui, &self.bar);
        });
        {
            let mut app = self.app.borrow_mut();
            app.top_bar(ui);
            app.suite_bar(ui);
            app.status_bar(ui);
            app.safety_bar(ui);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.dock.show(ui));
        self.after(chosen, &ctx);
        {
            let mut app = self.app.borrow_mut();
            app.dialog_window(&ctx);
            app.confirm_window(&ctx);
            app.suite_list_window(&ctx);
            app.suite_confirm_window(&ctx);
            app.shot_tick(&ctx);
        }
        // the arrangement is kept in the settings file when it changed (a picture keeps nothing)
        self.ticks = self.ticks.wrapping_add(1);
        if self.ticks.is_multiple_of(LAYOUT_CHECK_EVERY) {
            let pairs = self.dock.core().config_pairs();
            if self.watch.changed(&pairs) {
                let mut app = self.app.borrow_mut();
                let picture = app.shot.is_some();
                app.win.save_layout(&pairs, picture);
            }
        }
    }
}

/// The icon of the program for the window, from the file of the brand.
fn window_icon() -> Option<egui::IconData> {
    let bytes: &[u8] = include_bytes!("../images/brand/icon-256.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(egui::IconData {
        rgba: buf,
        width: info.width,
        height: info.height,
    })
}

/// The fonts of the system that draw Japanese and Chinese are added to the ones of egui, which have no such letters.
fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/YuGothM.ttc",
        "C:/Windows/Fonts/meiryo.ttc",
        "C:/Windows/Fonts/simsun.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".into(), Arc::new(FontData::from_owned(bytes)));
            for family in [FontFamily::Monospace, FontFamily::Proportional] {
                fonts.families.entry(family).or_default().push("cjk".into());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}

/// The colour of the program for the words that stand out, legible on every ground of these visuals (light or dark).
fn program_text(v: &egui::Visuals) -> Color32 {
    let grounds = [v.panel_fill, v.window_fill, v.extreme_bg_color].map(|c| [c.r(), c.g(), c.b()]);
    let [r, g, b] = panels::title_colour(grounds);
    Color32::from_rgb(r, g, b)
}

/// The colours of the theme Forge for the controls of egui.
fn apply_theme(ctx: &egui::Context, theme: ShotTheme) {
    match theme {
        ShotTheme::Forge => {}
        ShotTheme::Dark => {
            ctx.set_visuals(egui::Visuals::dark());
            return;
        }
        ShotTheme::Light => {
            ctx.set_visuals(egui::Visuals::light());
            return;
        }
    }
    let mut v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = RAISED;
    v.extreme_bg_color = Color32::from_rgb(0x14, 0x11, 0x0e);
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110);
    v.selection.stroke = Stroke::new(1.5, ACCENT);
    v.widgets.inactive.bg_stroke =
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(0xff, 0xb0, 0x00, 110));
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, ACCENT);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x33, 0x28, 0x1c);
    v.widgets.active.bg_stroke = Stroke::new(2.0, ACCENT);
    v.widgets.inactive.corner_radius = 6.0.into();
    v.widgets.hovered.corner_radius = 6.0.into();
    v.widgets.active.corner_radius = 6.0.into();
    ctx.set_visuals(v);
}

/// Opens the window. A picture given here is opened at once.
pub fn run(initial: Option<PathBuf>, lang: Lang) -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1180.0, 760.0])
        .with_min_inner_size([760.0, 480.0])
        .with_title("EH3D-PHOENIX-LASER");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "EH3D-PHOENIX-LASER",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, ShotTheme::Forge);
            let mut shell = Shell::new(App::new(lang, initial));
            // the list of recent projects lives in the folder of the user
            shell
                .dock
                .keep_user_recent(eh3d_dock::about::Phoenix::Laser);
            Ok(Box::new(shell))
        }),
    )
}

/// How the picture mode ended: nothing yet, the file was written, or the reason it was not.
type ShotOutcome = Arc<Mutex<Option<Result<(), String>>>>;

/// The picture being taken.
struct ShotState {
    request: ShotRequest,
    frame: u32,
    outcome: ShotOutcome,
}

impl App {
    /// Counts the frames, asks for the picture after the frames waited, writes it and closes the window when it is written or when it does not come.
    fn shot_tick(&mut self, ctx: &egui::Context) {
        let Some(shot) = self.shot.as_mut() else {
            return;
        };
        shot.frame += 1;
        ctx.request_repaint();
        let due = 3 + shot.request.wait;
        let finished = match shot.outcome.lock() {
            Ok(slot) => slot.is_some(),
            Err(_) => true,
        };
        if finished {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if shot.frame == due {
            let (output, replace) = (shot.request.output.clone(), shot.request.replace);
            let (outcome, repaint) = (shot.outcome.clone(), ctx.clone());
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                egui::ScreenshotCallback::new(move |image| {
                    let result = save_shot_png(&image, &output, replace);
                    if let Ok(mut slot) = outcome.lock() {
                        *slot = Some(result);
                    }
                    repaint.request_repaint();
                }),
            ));
        } else if shot.frame > due + PATIENCE_FRAMES {
            if let Ok(mut slot) = shot.outcome.lock() {
                *slot = Some(Err(
                    "the picture did not come back from the window".to_owned()
                ));
            }
        }
    }
}

/// Writes the picture the window gave back as a PNG file, a new file unless `replace` is true.
fn save_shot_png(image: &egui::ColorImage, path: &Path, replace: bool) -> Result<(), String> {
    let mut open = std::fs::OpenOptions::new();
    open.write(true);
    if replace {
        open.create(true).truncate(true);
    } else {
        open.create_new(true);
    }
    let file = open
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let mut encoder = png::Encoder::new(
        std::io::BufWriter::new(file),
        image.size[0] as u32,
        image.size[1] as u32,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    let data: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
    writer.write_image_data(&data).map_err(|e| e.to_string())?;
    // the end of the file is written here, so that a failure is reported and not lost in a drop
    writer.finish().map_err(|e| e.to_string())
}

/// Opens the window at the size asked, shows the file, saves the picture as the request says and closes by itself. The window keeps no settings and reads no settings file. The answer
/// is the reason when the picture could not be written.
pub fn run_shot(files: Vec<PathBuf>, lang: Lang, request: ShotRequest) -> Result<(), String> {
    if !request.replace && request.output.exists() {
        return Err(format!(
            "{} exists; give --replace to write over it",
            request.output.display()
        ));
    }
    let folder = request
        .output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(Path::to_path_buf);
    if folder.is_some_and(|dir| !dir.is_dir()) {
        return Err(format!(
            "the folder of {} does not exist",
            request.output.display()
        ));
    }
    if let Some(missing) = files.iter().find(|f| !f.is_file()) {
        return Err(format!("{} is not a file", missing.display()));
    }
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([request.size.0 as f32, request.size.1 as f32])
        .with_title("EH3D-PHOENIX-LASER");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    let outcome: ShotOutcome = Arc::new(Mutex::new(None));
    let outcome_for_app = outcome.clone();
    let theme = request.theme;
    let window = request.window;
    let initial = files.into_iter().next();
    eframe::run_native(
        "EH3D-PHOENIX-LASER",
        options,
        Box::new(move |cc| {
            install_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, theme);
            let mut app = App::build(lang, initial, false);
            match window {
                Some(ShotWindow::Mode(mode)) => app.win.set_mode(mode),
                Some(ShotWindow::Server) => app.server_open = true,
                Some(ShotWindow::Workspace(name)) => app
                    .pending_dock
                    .push(DockAction::SwitchWorkspace(name.to_string())),
                None => {}
            }
            app.shot = Some(ShotState {
                request,
                frame: 0,
                outcome: outcome_for_app,
            });
            Ok(Box::new(Shell::new(app)))
        }),
    )
    .map_err(|e| e.to_string())?;
    let taken = outcome.lock().map_err(|e| e.to_string())?.take();
    match taken {
        Some(result) => result,
        None => Err("the window closed before the picture was taken".to_owned()),
    }
}
