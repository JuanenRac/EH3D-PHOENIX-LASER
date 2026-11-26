//! The panels, the workspaces and the menus of the window as data, with no window in them: which panels there are and what each is called in the eight languages, the four arrangements the
//! window starts with (Prepare, Run, Review and Compact), the menu bar (File, Edit, View, Tools, Window and Help) with the actions of the program, which of its entries can be chosen at a
//! given moment, and the rule that decides when the arrangement is written to the settings file. The window (`gui`) draws these with the frame of the suite (`eh3d-dock`) and only does what
//! they say.
//!
//! The workspaces stand for tasks (what the person is doing), never for levels of experience: every setting is in the window in all of them.

use crate::lang::{Lang, Msg};
use crate::suitetext::S;
use eh3d_dock::about::{AboutBrand, Phoenix};
use eh3d_dock::core::PanelDef;
use eh3d_dock::geom::Side;
use eh3d_dock::menu::{DockAction, Entry, Item, Label, Menu, MenuBar, StdAction};
use eh3d_dock::model::{Layout, Node};
use eh3d_dock::palette;
use eh3d_dock::sectors::{titles, Sector};
use eh3d_dock::shortcuts::Shortcut;
use eh3d_dock::stdmenus::{
    edit_menu, file_menu, help_menu, tools_menu, view_menu, window_menu, HelpOptions,
};
use eh3d_dock::texts::Text8;
use eh3d_dock::workspaces::Workspace;

/// The colour of the titles and of the other words that stand out: the colour of the program, moved to a lightness that reads on each of the grounds (all light or all dark).
pub fn title_colour(grounds: [palette::Rgb; 3]) -> palette::Rgb {
    grounds
        .iter()
        .fold(Phoenix::Laser.accent(), |found, &ground| {
            palette::readable(found, ground)
        })
}

/// The look of the window «About»: which program of the family this is and its own picture, drawn in a ring of its colour.
pub fn about_brand() -> AboutBrand {
    AboutBrand {
        program: Phoenix::Laser,
        logo_png: include_bytes!("../images/brand/icon-256.png"),
    }
}

/// The names of the panels (what the settings file stores, so they never change).
pub mod id {
    /// The drawing of the work: the passes, the edge, the pointer.
    pub const VIEW: &str = "view";
    /// The table of layers of a drawing.
    pub const LAYERS: &str = "layers";
    /// The mode and the settings of the work: the picture, the laser, the grid, the text, the code...
    pub const CUT_SETTINGS: &str = "cut_settings";
    /// The file that is open, the machine profile and the notes of the reading.
    pub const PROPERTIES: &str = "properties";
    /// The library of materials of the person (open, choose, edit, save).
    pub const LIBRARY: &str = "library";
    /// The starting points of the common database of materials.
    pub const MATERIALS: &str = "materials";
    /// Framing the work with the laser off (or a pilot of low power) and the limits of the machine.
    pub const JOG: &str = "jog";
    /// The server: the link, the job that was sent, pause, resume and stop.
    pub const MACHINE: &str = "machine";
    /// What the program does: the moves, the lengths, the time, the energy and the check.
    pub const SIMULATION: &str = "simulation";
    /// The program that is worked out: the file, its hash, working it out and writing it.
    pub const JOB: &str = "job";
    /// The report written to be read before a job.
    pub const REPORT: &str = "report";
    /// What the program has to say: the state, the notes of the reading, the lines of the server.
    pub const CONSOLE: &str = "console";

    /// The attachments of the project of the suite: data sheets, manuals and other files, with their viewer.
    pub const ATTACHMENTS: &str = eh3d_attachpanel::PANEL_ID;

    /// Every panel, in the order of the View menu.
    pub const ALL: [&str; 13] = [
        VIEW,
        LAYERS,
        CUT_SETTINGS,
        PROPERTIES,
        LIBRARY,
        MATERIALS,
        JOG,
        MACHINE,
        SIMULATION,
        JOB,
        REPORT,
        CONSOLE,
        ATTACHMENTS,
    ];
}

/// The workspace of the work before the program is made: the file, the layers, the settings and the materials.
pub const WS_PREPARE: &str = "Prepare";
/// The workspace of the job: the view, the program, the machine and the framing.
pub const WS_RUN: &str = "Run";
/// The workspace of the check: the view with the layers, the report, the simulation and the settings.
pub const WS_REVIEW: &str = "Review";
/// The workspace with only the view and the job, the rest folded into the edges.
pub const WS_COMPACT: &str = "Compact";

/// The workspaces, in order.
pub const WORKSPACES: [&str; 4] = [WS_PREPARE, WS_RUN, WS_REVIEW, WS_COMPACT];

/// The things the window always shows, outside the frame: they are not panels, so no workspace can leave them out, no panel can be dragged over them and the layout of the person cannot
/// hide them. The stop of the job and the state of the interlocks must be in sight in every workspace, the compact one included.
pub const ALWAYS_VISIBLE: [&str; 2] = ["stop", "interlocks"];

/// A message as the text of the frame: the eight languages in the order English, Spanish, Italian, German, French, Portuguese (of Portugal), Chinese and Japanese.
pub fn t8(m: Msg) -> Text8 {
    Text8([
        m.tr(Lang::En),
        m.tr(Lang::Es),
        m.tr(Lang::It),
        m.tr(Lang::De),
        m.tr(Lang::Fr),
        m.tr(Lang::Pt),
        m.tr(Lang::Zh),
        m.tr(Lang::Ja),
    ])
}

/// A word of the project of the suite as the text of the frame, in the order of [`t8`].
pub fn s8(s: S) -> Text8 {
    let [en, es, de, fr, it, ja, zh, pt] = s.words();
    Text8([en, es, it, de, fr, pt, zh, ja])
}

fn def(id: &str, title: Text8, icon: &'static str, w: f32, h: f32) -> PanelDef {
    PanelDef::new(id, title).icon(icon).min(w, h)
}

/// The panels: the ones the suite proposes for a laser (without the camera, which the program does not have yet) and the ones this program adds.
pub fn defs() -> Vec<PanelDef> {
    let mut v: Vec<PanelDef> = Sector::Laser
        .panels()
        .into_iter()
        .filter(|p| p.id != "camera")
        .collect();
    v.push(def(id::PROPERTIES, titles::PROPERTIES, "P", 220.0, 140.0));
    v.push(def(id::MATERIALS, titles::MATERIALS, "M", 240.0, 160.0));
    v.push(def(id::MACHINE, titles::MACHINE, "S", 240.0, 160.0));
    v.push(def(
        id::SIMULATION,
        t8(Msg::PanelSimulation),
        "T",
        240.0,
        140.0,
    ));
    v.push(def(id::REPORT, titles::REPORT, "R", 260.0, 160.0));
    v.push(eh3d_attachpanel::panel_def());
    v
}

fn ws(name: &str, root: Node) -> Workspace {
    Workspace::new(name, Layout::with_root(root))
}

/// The four factory workspaces. The names are the English ones the settings file keeps; the frame shows them in the language of the window.
pub fn workspaces() -> Vec<Workspace> {
    let t = |ids: &[&str]| Node::tabs(ids);
    vec![
        // the work: the file and the materials on the left, the view with the layers below it, the settings and the job on the right
        ws(
            WS_PREPARE,
            Node::row(
                0.22,
                Node::column(
                    0.4,
                    t(&[id::PROPERTIES, id::ATTACHMENTS]),
                    t(&[id::LIBRARY, id::MATERIALS]),
                ),
                Node::row(
                    0.74,
                    Node::column(0.66, t(&[id::VIEW]), t(&[id::LAYERS, id::CONSOLE])),
                    t(&[id::CUT_SETTINGS, id::JOB]),
                ),
            ),
        ),
        // the job: the view and what the program does, the job beside the machine and the framing
        ws(
            WS_RUN,
            Node::row(
                0.7,
                Node::column(0.7, t(&[id::VIEW]), t(&[id::CONSOLE, id::SIMULATION])),
                Node::column(0.4, t(&[id::JOB]), t(&[id::MACHINE, id::JOG])),
            ),
        ),
        // the check: the view with the layers, the report and the simulation, the settings aside
        ws(
            WS_REVIEW,
            Node::row(
                0.64,
                Node::column(0.68, t(&[id::VIEW]), t(&[id::LAYERS, id::CONSOLE])),
                Node::column(
                    0.5,
                    t(&[id::REPORT]),
                    t(&[id::SIMULATION, id::CUT_SETTINGS]),
                ),
            ),
        ),
        Workspace::new(
            WS_COMPACT,
            Layout::with_root(t(&[id::VIEW, id::JOB]))
                .with_auto_hidden(id::PROPERTIES, Side::Left)
                .with_auto_hidden(id::CUT_SETTINGS, Side::Right)
                .with_auto_hidden(id::CONSOLE, Side::Bottom),
        ),
    ]
}

/// What the program does when an entry of its menus is chosen.
#[derive(Debug, Clone, PartialEq)]
pub enum Cmd {
    /// An action every program has (New, Open, Save, Quit...).
    Std(StdAction),
    /// Choose a picture or a drawing to open.
    OpenImage,
    /// Choose a file of settings to read.
    OpenSettings,
    /// Choose a G-code program to look at.
    OpenGcode,
    /// Choose a library of materials to read.
    OpenMaterials,
    /// Make a new library of materials in a file that is not there.
    NewMaterials,
    /// Open a project file of the suite.
    SuiteOpen,
    /// Save into the project of the window.
    SuiteSave,
    /// Save the project under another name.
    SuiteSaveAs,
    /// Import a drawing into the project, keeping the original.
    SuiteImport,
    /// Take a file of another branch.
    SuiteTake,
    /// Write the files of a branch into a folder.
    SuiteExtract,
    /// List the branches of the project.
    SuiteBranches,
    /// Work out the program for what is chosen.
    Compute,
    /// Write the program to a file.
    ExportGcode,
    /// Fit the work to the view.
    Fit,
    /// Make the report of the program.
    MakeReport,
    /// Send the program to the server.
    SendToServer,
    /// The emergency stop of the job on the server.
    EmergencyStop,
}

impl From<StdAction> for Cmd {
    fn from(a: StdAction) -> Cmd {
        Cmd::Std(a)
    }
}

fn own(id: &str, m: Msg, cmd: Cmd) -> Entry<Cmd> {
    Entry::app(id, Label::Own(t8(m)), cmd)
}

/// The entries of the project file of the suite, a submenu of File.
fn suite_menu() -> Menu<Cmd> {
    let entry = |id: &str, word: S, cmd: Cmd| Entry::app(id, Label::Own(s8(word)), cmd);
    Menu::new("file.suite", Label::Own(s8(S::Menu)))
        .entry(entry("file.suite.open", S::MenuOpen, Cmd::SuiteOpen))
        .entry(entry("file.suite.save", S::MenuSave, Cmd::SuiteSave))
        .entry(entry("file.suite.save_as", S::MenuSaveAs, Cmd::SuiteSaveAs))
        .sep()
        .entry(entry("file.suite.import", S::MenuImport, Cmd::SuiteImport))
        .entry(entry("file.suite.take", S::MenuTake, Cmd::SuiteTake))
        .entry(entry(
            "file.suite.extract",
            S::MenuExtract,
            Cmd::SuiteExtract,
        ))
        .sep()
        .entry(entry(
            "file.suite.branches",
            S::MenuBranches,
            Cmd::SuiteBranches,
        ))
        .entry(Entry::dock(
            "file.suite.attachments",
            Label::Own(eh3d_attachpanel::panel_title()),
            DockAction::ShowPanel(id::ATTACHMENTS.to_string()),
        ))
}

/// The menu bar: File, Edit, View, Tools, Window and Help.
pub fn menu_bar() -> MenuBar<Cmd> {
    let mut file: Menu<Cmd> = file_menu(true);
    let edit: Menu<Cmd> = edit_menu();
    let view: Menu<Cmd> = view_menu();
    let mut tools: Menu<Cmd> = tools_menu();
    let window: Menu<Cmd> = window_menu();
    let help: Menu<Cmd> = help_menu(HelpOptions {
        online_help: false,
        check_updates: false,
    });
    // File: the files this program opens go beside Open, and the export is the G-code
    let at = file
        .items
        .iter()
        .position(|i| matches!(i, Item::Entry(e) if e.id == "file.open"))
        .map_or(file.items.len(), |p| p + 1);
    let opens = [
        own("file.open_settings", Msg::OpenSettings, Cmd::OpenSettings),
        own("file.open_gcode", Msg::OpenGcode, Cmd::OpenGcode),
        own(
            "file.open_materials",
            Msg::OpenMaterials,
            Cmd::OpenMaterials,
        ),
        own(
            "file.new_materials",
            Msg::MaterialNewFile,
            Cmd::NewMaterials,
        ),
    ];
    let after_opens = at + opens.len();
    for (k, e) in opens.into_iter().enumerate() {
        file.items.insert(at + k, Item::Entry(e));
    }
    file.items.insert(after_opens, Item::Submenu(suite_menu()));
    for item in &mut file.items {
        if let Item::Entry(e) = item {
            if e.id == "file.export" {
                e.label = Label::Own(t8(Msg::ExportGcode));
            }
        }
    }
    // Tools: work it out, fit, report, send, stop
    tools = tools
        .entry(own("tools.compute", Msg::Compute, Cmd::Compute).key(Shortcut::plain("F5")))
        .entry(own("tools.fit", Msg::Fit, Cmd::Fit).key(Shortcut::ctrl("0")))
        .entry(own("tools.report", Msg::ReportMake, Cmd::MakeReport))
        .sep()
        .entry(own("tools.send", Msg::ServerSend, Cmd::SendToServer))
        .entry(own("tools.stop", Msg::ServerStop, Cmd::EmergencyStop).key(Shortcut::plain("F12")));
    MenuBar::new(vec![file, edit, view, tools, window, help])
}

/// What the window knows when it decides which entries of the menus can be chosen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MenuFacts {
    /// There is a picture, a drawing, a program or a result.
    pub has_work: bool,
    /// The program can be worked out for what is chosen.
    pub can_compute: bool,
    /// The program can be written.
    pub can_export: bool,
    /// There is a program that was worked out.
    pub has_done: bool,
    /// The program can be sent to the server.
    pub can_send: bool,
    /// A job that was sent is being followed.
    pub server_active: bool,
    /// The window works on a project file of the suite.
    pub has_suite: bool,
}

/// The ids of the entries that cannot be chosen now. The ones for things the program does not have (new, save, undo, redo, cut, copy, paste) are always off.
pub fn disabled_entries(f: &MenuFacts) -> Vec<String> {
    let mut off: Vec<&str> = vec![
        "file.new",
        "file.save",
        "file.save_as",
        "edit.undo",
        "edit.redo",
        "edit.cut",
        "edit.copy",
        "edit.paste",
    ];
    let mut when = |cond: bool, ids: &[&'static str]| {
        if cond {
            off.extend_from_slice(ids);
        }
    };
    when(!f.can_export, &["file.export"]);
    when(!f.can_compute, &["tools.compute"]);
    when(!f.has_work, &["tools.fit"]);
    when(!f.has_done, &["tools.report"]);
    when(!f.can_send || f.server_active, &["tools.send"]);
    when(!f.server_active, &["tools.stop"]);
    when(
        !f.has_suite,
        &[
            "file.suite.import",
            "file.suite.take",
            "file.suite.extract",
            "file.suite.branches",
        ],
    );
    let mut v: Vec<String> = off.into_iter().map(str::to_string).collect();
    v.sort();
    v.dedup();
    v
}

/// Decides when the arrangement of the panels is written to the settings file: when the lines of the frame are not the ones last written (or read). It writes nothing when nothing changed, so
/// dragging a panel writes once and a quiet window writes never.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LayoutWatch {
    saved: String,
}

impl LayoutWatch {
    /// A watch that knows the lines of the frame as they were read.
    pub fn new(pairs: &[(String, String)]) -> LayoutWatch {
        LayoutWatch {
            saved: LayoutWatch::flat(pairs),
        }
    }

    fn flat(pairs: &[(String, String)]) -> String {
        pairs.iter().map(|(k, v)| format!("{k}={v}\n")).collect()
    }

    /// Whether the lines changed since the last time this answered yes (or since the start); a yes counts them as written.
    pub fn changed(&mut self, pairs: &[(String, String)]) -> bool {
        let now = LayoutWatch::flat(pairs);
        if now == self.saved {
            return false;
        }
        self.saved = now;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eh3d_dock::core::DockCore;
    use eh3d_dock::menu::{DockAction, MenuContext};
    use eh3d_dock::sectors::workspace_title;
    use eh3d_view::texts::Lang as ViewLang;

    fn core() -> DockCore {
        DockCore::new(defs(), workspaces())
    }

    fn bar_ids(bar: &MenuBar<Cmd>) -> Vec<String> {
        fn walk(items: &[Item<Cmd>], out: &mut Vec<String>) {
            for i in items {
                match i {
                    Item::Entry(e) => out.push(e.id.clone()),
                    Item::Submenu(m) => walk(&m.items, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for m in &bar.menus {
            walk(&m.items, &mut out);
        }
        out
    }

    #[test]
    fn the_panels_have_names_that_hold_and_titles_in_the_eight_languages() {
        let d = defs();
        assert_eq!(d.len(), 13);
        let mut names: Vec<&str> = d.iter().map(|p| p.id.as_str()).collect();
        names.sort_unstable();
        let mut want = id::ALL.to_vec();
        want.sort_unstable();
        assert_eq!(names, want);
        for p in &d {
            assert!(
                p.id.len() <= 48
                    && p.id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{}",
                p.id
            );
            for t in p.title.0 {
                assert!(!t.trim().is_empty(), "{}", p.id);
            }
        }
        // the camera is a panel of the sector that this program does not have yet
        assert!(!names.contains(&"camera"));
        // the title of the simulation is the message of the program, in the order the frame wants (English, Spanish, Italian, German, French, Portuguese, Chinese, Japanese)
        let sim = d.iter().find(|p| p.id == id::SIMULATION).unwrap();
        assert_eq!(sim.title.0[0], "Simulation and time");
        assert_eq!(sim.title.0[1], "Simulación y tiempo");
        assert_eq!(sim.title.0[2], "Simulazione e tempo");
        assert_eq!(sim.title.0[3], "Simulation und Zeit");
        assert_eq!(sim.title.0[5], "Simulação e tempo");
        assert_eq!(sim.title.0[7], "シミュレーションと時間");
    }

    #[test]
    fn the_factory_arrangement_is_sound_and_every_workspace_has_a_name_in_eight_languages() {
        let c = core();
        assert_eq!(
            c.workspaces().names(),
            vec!["Prepare", "Run", "Review", "Compact"]
        );
        assert_eq!(c.workspaces().names(), WORKSPACES.to_vec());
        assert!(c.warnings().is_empty(), "{:?}", c.warnings());
        assert!(c.layout_is_sound());
        for name in c.workspaces().names() {
            let t = workspace_title(name).unwrap_or_else(|| panic!("{name}"));
            assert!(t.0.iter().all(|x| !x.trim().is_empty()));
        }
        // the names are the ones the owner asked for, in Spanish
        assert_eq!(workspace_title("Prepare").unwrap().0[1], "Preparar");
        assert_eq!(workspace_title("Run").unwrap().0[1], "Ejecutar");
        assert_eq!(workspace_title("Review").unwrap().0[1], "Revisar");
        assert_eq!(workspace_title("Compact").unwrap().0[1], "Compacto");
        // every panel is in some workspace, so the View menu can bring any of them back from a factory layout
        for p in id::ALL {
            let shown = c
                .workspaces()
                .factory()
                .iter()
                .any(|w| w.layout.is_shown(p));
            assert!(shown, "{p} is in no workspace");
        }
    }

    #[test]
    fn each_workspace_shows_what_its_task_needs() {
        let c = core();
        let f = c.workspaces().factory();
        let of = |name: &str| f.iter().find(|w| w.name == name).unwrap();
        let shows = |name: &str, p: &str| of(name).layout.is_shown(p);
        // preparing: the file, the layers, the settings and the materials
        for p in [
            id::VIEW,
            id::PROPERTIES,
            id::LAYERS,
            id::CUT_SETTINGS,
            id::LIBRARY,
            id::MATERIALS,
            id::JOB,
        ] {
            assert!(shows(WS_PREPARE, p), "Prepare: {p}");
        }
        // running: the view, the job, the machine, the framing, the simulation and the console
        for p in [
            id::VIEW,
            id::JOB,
            id::MACHINE,
            id::JOG,
            id::SIMULATION,
            id::CONSOLE,
        ] {
            assert!(shows(WS_RUN, p), "Run: {p}");
        }
        // reviewing: the view, the layers, the report, the simulation and the settings
        for p in [
            id::VIEW,
            id::LAYERS,
            id::REPORT,
            id::SIMULATION,
            id::CUT_SETTINGS,
        ] {
            assert!(shows(WS_REVIEW, p), "Review: {p}");
        }
        // the compact one has the view and the job, and the rest is folded into the edges (not lost)
        assert!(shows(WS_COMPACT, id::VIEW) && shows(WS_COMPACT, id::JOB));
        assert!(!shows(WS_COMPACT, id::REPORT));
        // there is no workspace for beginners or for experts: the settings are in the same panels in all of them, and none of the names says a level
        for name in WORKSPACES {
            let lower = name.to_lowercase();
            assert!(
                !["begin", "novice", "expert", "advanced", "basic", "easy"]
                    .iter()
                    .any(|w| lower.contains(w)),
                "{name}"
            );
        }
        for w in WORKSPACES {
            if w != WS_COMPACT {
                assert!(shows(w, id::CUT_SETTINGS) || w == WS_RUN, "{w}");
            }
        }
    }

    #[test]
    fn the_stop_and_the_interlocks_are_outside_every_panel_so_no_workspace_hides_them() {
        let panel_ids: Vec<&str> = id::ALL.to_vec();
        for always in ALWAYS_VISIBLE {
            // they are not panels: the frame cannot fold, float, hide or lock them away
            assert!(!panel_ids.contains(&always), "{always} is a panel");
            for p in defs() {
                assert_ne!(p.id, always);
            }
        }
        // every workspace, the compact one included, leaves the whole window to the frame: a panel can fold into an edge but the bar of the stop is not part of the tree
        let c = core();
        for w in c.workspaces().factory() {
            for always in ALWAYS_VISIBLE {
                assert!(!w.layout.is_shown(always), "{} names {always}", w.name);
            }
        }
    }

    #[test]
    fn the_menu_bar_is_well_made_and_has_no_shortcut_in_conflict_in_any_language() {
        let bar = menu_bar();
        assert!(bar.validate().is_empty(), "{:?}", bar.validate());
        let titles: Vec<&str> = bar.menus.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(titles, ["file", "edit", "view", "tools", "window", "help"]);
        for l in [
            ViewLang::En,
            ViewLang::Es,
            ViewLang::De,
            ViewLang::Fr,
            ViewLang::It,
            ViewLang::Pt,
            ViewLang::Zh,
            ViewLang::Ja,
        ] {
            let ctx = MenuContext::empty(l);
            assert!(
                bar.conflicts(&ctx).is_empty(),
                "{:?} in {l:?}",
                bar.conflicts(&ctx)
            );
        }
        let ids = bar_ids(&bar);
        for want in [
            "file.open",
            "file.open_settings",
            "file.open_gcode",
            "file.open_materials",
            "file.new_materials",
            "file.export",
            "file.quit",
            "file.suite.open",
            "file.suite.save",
            "file.suite.save_as",
            "file.suite.import",
            "file.suite.take",
            "file.suite.extract",
            "file.suite.branches",
            "file.suite.attachments",
            "edit.preferences",
            "tools.compute",
            "tools.fit",
            "tools.report",
            "tools.send",
            "tools.stop",
            "help.shortcuts",
            "help.about",
        ] {
            assert!(ids.iter().any(|x| x == want), "{want}");
        }
        // no id twice
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len());
    }

    #[test]
    fn the_files_of_the_program_come_after_open_and_the_export_is_the_gcode() {
        let bar = menu_bar();
        let file = &bar.menus[0];
        let pos = |wanted: &str| {
            file.items
                .iter()
                .position(|i| matches!(i, Item::Entry(e) if e.id == wanted))
                .unwrap_or_else(|| panic!("no entry {wanted}"))
        };
        assert_eq!(pos("file.open_settings"), pos("file.open") + 1);
        assert_eq!(pos("file.open_gcode"), pos("file.open") + 2);
        assert_eq!(pos("file.new_materials"), pos("file.open") + 4);
        let export = file
            .items
            .iter()
            .find_map(|i| match i {
                Item::Entry(e) if e.id == "file.export" => Some(e),
                _ => None,
            })
            .unwrap();
        let Label::Own(t) = &export.label else {
            panic!("the export has a standard label")
        };
        assert!(t.0[0].starts_with("Export G-code"), "{}", t.0[0]);
        assert!(t.0[1].starts_with("Exportar G-code"), "{}", t.0[1]);
    }

    #[test]
    fn the_entries_that_are_off_follow_what_there_is_to_work_with() {
        let nothing = disabled_entries(&MenuFacts::default());
        for id in [
            "file.new",
            "file.save",
            "file.save_as",
            "edit.undo",
            "edit.paste",
            "file.export",
            "tools.compute",
            "tools.fit",
            "tools.report",
            "tools.send",
            "tools.stop",
        ] {
            assert!(nothing.iter().any(|x| x == id), "{id}");
        }
        for id in ["file.suite.import", "file.suite.take", "file.suite.extract"] {
            assert!(nothing.iter().any(|x| x == id), "{id}");
        }
        assert!(!nothing.iter().any(|x| x == "file.suite.open"));
        assert!(!nothing.iter().any(|x| x == "file.suite.save"));
        // every id that can be off is an entry of the bar
        let ids = bar_ids(&menu_bar());
        for off in &nothing {
            assert!(ids.contains(off), "{off} is not in the menus");
        }
        let all = disabled_entries(&MenuFacts {
            has_work: true,
            can_compute: true,
            can_export: true,
            has_done: true,
            can_send: true,
            server_active: false,
            has_suite: true,
        });
        assert_eq!(
            all,
            [
                "edit.copy",
                "edit.cut",
                "edit.paste",
                "edit.redo",
                "edit.undo",
                "file.new",
                "file.save",
                "file.save_as",
                "tools.stop"
            ]
        );
        // a job that is followed cannot be sent again, and it can be stopped
        let busy = disabled_entries(&MenuFacts {
            can_send: true,
            server_active: true,
            ..MenuFacts::default()
        });
        assert!(busy.contains(&"tools.send".to_string()));
        assert!(!busy.contains(&"tools.stop".to_string()));
    }

    #[test]
    fn the_stop_has_a_key_of_its_own_that_no_other_entry_takes() {
        let bar = menu_bar();
        let mut keys: Vec<String> = Vec::new();
        for m in &bar.menus {
            for i in &m.items {
                if let Item::Entry(e) = i {
                    if let Some(s) = &e.shortcut {
                        keys.push(s.to_string());
                    }
                }
            }
        }
        let stop = bar
            .menus
            .iter()
            .flat_map(|m| m.items.iter())
            .find_map(|i| match i {
                Item::Entry(e) if e.id == "tools.stop" => e.shortcut.clone(),
                _ => None,
            })
            .expect("the stop has a key");
        assert_eq!(keys.iter().filter(|k| **k == stop.to_string()).count(), 1);
    }

    #[test]
    fn the_layout_is_written_when_it_changes_and_only_then() {
        let a = vec![("dock.current".to_string(), "0".to_string())];
        let b = vec![("dock.current".to_string(), "1".to_string())];
        let mut w = LayoutWatch::new(&a);
        assert!(!w.changed(&a));
        assert!(w.changed(&b));
        assert!(!w.changed(&b));
        assert!(w.changed(&a));
        // an empty watch writes a first non-empty arrangement
        let mut e = LayoutWatch::default();
        assert!(!e.changed(&[]));
        assert!(e.changed(&a));
    }

    #[test]
    fn a_layout_the_person_made_survives_a_round_trip_through_the_settings_text() {
        // a person floats the console and hides the report; the lines go to the settings text and come back
        let mut c = core();
        c.apply(&DockAction::HidePanel(id::REPORT.to_string()))
            .unwrap();
        c.apply(&DockAction::FloatPanel(id::CONSOLE.to_string()))
            .ok();
        let lines = c.config_pairs();
        assert!(lines.iter().all(|(k, _)| k.starts_with("dock.")));
        let text = eh3d_dock::config::replace_block("[machine]\ns_max = 1000\n", &lines);
        let mut back = core();
        back.load_config(&text);
        assert_eq!(back.config_pairs(), lines);
        assert!(back.warnings().is_empty());
    }
}

#[cfg(test)]
mod about_tests {
    use super::*;

    #[test]
    fn the_window_about_has_the_name_the_repository_and_the_picture_of_this_program() {
        let brand = about_brand();
        assert_eq!(brand.program, Phoenix::Laser);
        assert_eq!(brand.program.name(), "EH3D-PHOENIX-LASER");
        assert_eq!(
            brand.program.repository(),
            "https://github.com/JuanenRac/EH3D-PHOENIX-LASER"
        );
        let png = brand.logo_png;
        assert_eq!(png.get(1..4), Some(&b"PNG"[..]));
        // the width and the height in the header of the picture
        assert_eq!(png.get(16..24), Some(&[0, 0, 1, 0, 0, 0, 1, 0][..]));
    }
}
