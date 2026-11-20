//! What the window of panels needs from the model, without a screen: a project opened fills the table of layers, the report of the program, the line of the interlocks, the arrangement of
//! the panels kept in the settings file next to the settings, and the starting points of the common database of materials.

use eh3d_phoenix_laser::lang::{Lang, Msg};
use eh3d_phoenix_laser::panels;
use eh3d_phoenix_laser::vconfig::{self, LayerMode};
use eh3d_phoenix_laser::window::{Job, Key, LayerField, Mode, Status, Window};
use std::fs;
use std::path::PathBuf;

struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir =
            std::env::temp_dir().join(format!("eh3d-laser-dockwin-{}-{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let p = self.0.join(name);
        fs::write(&p, text).unwrap();
        p
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const PROJECT: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<LightBurnProject AppVersion=\"9.9.00\" FormatVersion=\"1\">\n    <CutSetting type=\"Cut\">\n        <index Value=\"1\"/>\n        <name Value=\"C01\"/>\n        <maxPower Value=\"85\"/>\n        <speed Value=\"8\"/>\n        <numPasses Value=\"2\"/>\n        <priority Value=\"3\"/>\n    </CutSetting>\n    <CutSetting type=\"Scan\">\n        <index Value=\"0\"/>\n        <name Value=\"C00\"/>\n        <maxPower Value=\"20\"/>\n        <speed Value=\"100\"/>\n        <interval Value=\"0.1\"/>\n    </CutSetting>\n    <Shape Type=\"Rect\" CutIndex=\"1\" W=\"30\" H=\"20\" Cr=\"0\">\n        <XForm>1 0 0 1 50 50</XForm>\n    </Shape>\n    <Shape Type=\"Rect\" CutIndex=\"0\" W=\"10\" H=\"10\" Cr=\"0\">\n        <XForm>1 0 0 1 50 50</XForm>\n    </Shape>\n</LightBurnProject>\n";

const SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"30mm\" height=\"20mm\" viewBox=\"0 0 30 20\"><rect x=\"1\" y=\"1\" width=\"28\" height=\"18\" fill=\"none\" stroke=\"#ff0000\"/></svg>";

fn set(w: &mut Window, key: Key, text: &str) {
    let i = w.fields.iter().position(|f| f.key == key).expect("a box");
    w.set_field(i, text.to_string());
}

fn text_of(w: &Window, key: Key) -> String {
    w.fields
        .iter()
        .find(|f| f.key == key)
        .expect("a box")
        .text
        .clone()
}

fn row<'a>(w: &'a Window, key: &str) -> &'a eh3d_phoenix_laser::window::LayerRow {
    w.rows
        .iter()
        .find(|r| r.key == key)
        .unwrap_or_else(|| panic!("no layer {key}"))
}

// ---------------------------------------------------------------- a project in the window

#[test]
fn a_project_that_is_opened_fills_the_table_of_layers_with_what_the_file_gives_for_each() {
    let s = Scratch::new("hints");
    let file = s.write("plate.lbrn2", PROJECT);
    let mut w = Window::new(Lang::En);
    assert!(w.open_dropped(&file));
    assert_eq!(w.mode, Mode::Cut);
    assert!(
        matches!(
            w.status,
            Status::VectorOpened {
                shapes: 2,
                layers: 2,
                ..
            }
        ),
        "{:?}",
        w.status
    );
    let c01 = row(&w, "c01");
    assert_eq!(c01.mode, LayerMode::Cut);
    assert_eq!(c01.fields[LayerField::Power as usize], "85");
    // the file gives mm/s, the table mm/min
    assert_eq!(c01.fields[LayerField::Speed as usize], "480");
    assert_eq!(c01.fields[LayerField::Passes as usize], "2");
    let c00 = row(&w, "c00");
    assert_eq!(c00.mode, LayerMode::Engrave);
    assert_eq!(c00.fields[LayerField::Power as usize], "20");
    assert_eq!(c00.fields[LayerField::HatchSpacing as usize], "0.1");
    // the notes of the reading say that the values are the project's
    assert!(
        w.warnings
            .iter()
            .any(|n| n.contains("check them for this machine")),
        "{:?}",
        w.warnings
    );
}

#[test]
fn a_layer_that_the_settings_file_names_keeps_what_the_settings_file_says() {
    let s = Scratch::new("named");
    let cfg = s.write(
        "laser.cfg",
        "[layer c01]\nmode = cut\npower = 20\nspeed = 300\npasses = 1\n",
    );
    let file = s.write("plate.lbrn2", PROJECT);
    let mut w = Window::new(Lang::En);
    assert!(w.load_config(&cfg));
    assert!(w.open_dropped(&file));
    let c01 = row(&w, "c01");
    assert_eq!(c01.fields[LayerField::Power as usize], "20");
    assert_eq!(c01.fields[LayerField::Speed as usize], "300");
    // the layer the file does not name takes the hint
    assert_eq!(row(&w, "c00").fields[LayerField::Power as usize], "20");
    assert_eq!(row(&w, "c00").mode, LayerMode::Engrave);
}

#[test]
fn a_power_from_a_project_over_the_limit_of_the_machine_is_refused_and_never_lowered() {
    let s = Scratch::new("limit");
    let file = s.write("plate.lbrn2", PROJECT);
    let mut w = Window::new(Lang::En);
    assert!(w.open_dropped(&file));
    set(&mut w, Key::MaxPower, "50");
    assert!(!w.compute());
    assert!(w.status.is_problem(), "{:?}", w.status);
    // the table still says 85: nothing was lowered in silence
    assert_eq!(row(&w, "c01").fields[LayerField::Power as usize], "85");
    // the person lowers it and the work is made
    set(&mut w, Key::MaxPower, "100");
    assert!(w.compute(), "{:?}", w.status);
}

#[test]
fn the_window_offers_and_accepts_projects_and_says_so_when_it_refuses_another_file() {
    assert!(Job::OpenImage.extensions().contains(&"lbrn2"));
    let s = Scratch::new("kinds");
    let other = s.write("notes.txt", "x");
    let mut w = Window::new(Lang::En);
    assert!(!w.open_dropped(&other));
    let Status::Failed(m) = &w.status else {
        panic!("{:?}", w.status)
    };
    assert!(m.contains("LBRN2"), "{m}");
    // the sentence that lists the files is in every language and names the project
    for l in Lang::ALL {
        assert!(Msg::Formats.tr(l).contains("LBRN2"), "{}", l.code());
    }
}

#[test]
fn a_project_that_cannot_be_read_leaves_the_drawing_that_was_open() {
    let s = Scratch::new("bad");
    let svg = s.write("plate.svg", SVG);
    let bad = s.write("bad.lbrn2", "this is not a project");
    let mut w = Window::new(Lang::En);
    assert!(w.open_dropped(&svg));
    assert!(!w.open_dropped(&bad));
    assert!(w.status.is_problem());
    let still = w.vector.as_ref().expect("the first drawing is still there");
    assert_eq!(still.ext, "svg");
}

// ---------------------------------------------------------------- the report

#[test]
fn the_report_is_made_from_the_program_and_dropped_when_anything_changes() {
    let s = Scratch::new("report");
    let file = s.write("plate.lbrn2", PROJECT);
    let mut w = Window::new(Lang::En);
    // nothing has been worked out yet
    assert!(!w.make_report());
    assert!(w.report_text.is_none());
    assert!(w.open_dropped(&file));
    assert!(w.compute(), "{:?}", w.status);
    assert!(w.make_report(), "{:?}", w.status);
    let en = w.report_text.clone().unwrap();
    assert!(en.contains("plate.lbrn2"), "{en}");
    assert!(en.to_lowercase().contains("sha-256"), "{en}");
    // a setting that changes after the program was worked out drops the report
    set(&mut w, Key::SMax, "500");
    assert!(w.report_text.is_none());
    // in another language the same report is written in that language
    set(&mut w, Key::SMax, "1000");
    assert!(w.compute());
    w.lang = Lang::Es;
    assert!(w.make_report());
    let es = w.report_text.clone().unwrap();
    assert_ne!(es, en);
    assert!(es.contains("plate.lbrn2"));
}

// ---------------------------------------------------------------- the line of the interlocks

#[test]
fn the_line_of_the_interlocks_says_each_state_in_the_language_of_the_window() {
    let mut w = Window::new(Lang::En);
    assert_eq!(
        w.safety_line(),
        "Emergency stop: released · Cover: unknown · Air: unknown"
    );
    assert!(!w.safety_problem());
    w.lang = Lang::Es;
    assert_eq!(
        w.safety_line(),
        "Paro de emergencia: liberado · Tapa: desconocido · Aire: desconocido"
    );
    for l in Lang::ALL {
        w.lang = l;
        let line = w.safety_line();
        assert!(
            !line.contains('{') && !line.trim().is_empty(),
            "{}: {line}",
            l.code()
        );
    }
}

#[test]
fn a_pressed_stop_and_a_required_interlock_that_is_not_safe_show_the_line_as_a_problem() {
    let s = Scratch::new("safety");
    let cfg = s.write(
        "laser.cfg",
        "[interlocks]\nestop = pressed\nenclosure = open\nair_assist = on\n",
    );
    let mut w = Window::new(Lang::En);
    assert!(w.load_config(&cfg));
    assert_eq!(
        w.safety_line(),
        "Emergency stop: pressed · Cover: open · Air: on"
    );
    assert!(w.safety_problem());
    let cfg2 = s.write("laser2.cfg", "[interlocks]\nrequire_enclosure = on\n");
    let mut v = Window::new(Lang::En);
    assert!(v.load_config(&cfg2));
    assert!(
        v.safety_problem(),
        "a cover that is required and not known to be closed is a problem"
    );
    let cfg3 = s.write(
        "laser3.cfg",
        "[interlocks]\nrequire_enclosure = on\nenclosure = closed\n",
    );
    let mut z = Window::new(Lang::En);
    assert!(z.load_config(&cfg3));
    assert!(!z.safety_problem());
}

// ---------------------------------------------------------------- the arrangement of the panels in the settings file

#[test]
fn the_reader_of_the_settings_leaves_the_lines_of_the_frame_alone_wherever_they_are() {
    let text = "dock.version=1\n[machine]\ns_max = 500\ndock.current = 2\ndock.ws.1.tree=H200(T0(view),T0(job))\n";
    let s = vconfig::parse(text).expect("lines of the frame are not settings");
    assert_eq!(s.machine.s_max, 500);
    // only the lines that start with dock. are the frame's: anything else that is not a setting is still refused
    assert!(vconfig::parse("[machine]\ndocker = 1\n").is_err());
    assert!(vconfig::parse("docks = 1\n").is_err());
    assert!(vconfig::parse("[machine]\ndock = 1\n").is_err());
}

#[test]
fn the_arrangement_is_written_next_to_the_settings_without_touching_them_and_only_when_it_changes()
{
    let s = Scratch::new("layout");
    let original = "# my laser\n[machine]\ns_max = 800\n\n[layer c01]\nmode = cut\npower = 30\n";
    let cfg = s.write("laser.cfg", original);
    let mut w = Window::new(Lang::En);
    assert!(w.load_config(&cfg));
    let core = eh3d_dock::core::DockCore::new(panels::defs(), panels::workspaces());
    let pairs = core.config_pairs();
    assert!(!pairs.is_empty());
    // a picture keeps nothing
    assert!(!w.save_layout(&pairs, true));
    assert_eq!(fs::read_to_string(&cfg).unwrap(), original);
    assert!(w.save_layout(&pairs, false));
    let written = fs::read_to_string(&cfg).unwrap();
    assert!(written.starts_with(original), "{written}");
    assert!(written.contains("dock.version=1"));
    // the same arrangement is not written again
    assert!(!w.save_layout(&pairs, false));
    // the file still reads, with the same settings
    let again = vconfig::load(&cfg).unwrap();
    assert_eq!(again.machine.s_max, 800);
    // and what the frame reads back from it is what it wrote
    let mut back = eh3d_dock::core::DockCore::new(panels::defs(), panels::workspaces());
    back.load_config(&w.layout_source());
    assert_eq!(back.config_pairs(), pairs);
    assert!(back.warnings().is_empty(), "{:?}", back.warnings());
}

#[test]
fn a_person_who_moved_the_panels_gets_them_back_where_they_left_them() {
    let s = Scratch::new("moved");
    let cfg = s.write("laser.cfg", "[machine]\ns_max = 800\n");
    let mut w = Window::new(Lang::En);
    assert!(w.load_config(&cfg));
    let mut core = eh3d_dock::core::DockCore::new(panels::defs(), panels::workspaces());
    core.apply(&eh3d_dock::menu::DockAction::HidePanel(
        panels::id::REPORT.to_string(),
    ))
    .unwrap();
    core.apply(&eh3d_dock::menu::DockAction::SwitchWorkspace(
        panels::WS_RUN.to_string(),
    ))
    .unwrap();
    assert!(w.save_layout(&core.config_pairs(), false));
    let mut next = eh3d_dock::core::DockCore::new(panels::defs(), panels::workspaces());
    next.load_config(&w.layout_source());
    assert_eq!(next.workspaces().current_name(), panels::WS_RUN);
    assert_eq!(next.config_pairs(), core.config_pairs());
    // no settings file, no arrangement kept
    let mut bare = Window::new(Lang::En);
    assert!(bare.layout_source().is_empty());
    assert!(!bare.save_layout(&core.config_pairs(), false));
}

// ---------------------------------------------------------------- the common database of materials

#[test]
fn without_the_database_nothing_is_offered_and_nothing_is_changed() {
    if eh3d_phoenix_laser::commonmat::available() {
        return;
    }
    let mut w = Window::new(Lang::En);
    assert!(w.common_entries().is_empty());
    assert!(w.common_answer().is_none());
    w.common_id = Some("wood/x".to_string());
    assert!(!w.use_common_start());
    assert_eq!(
        text_of(&w, Key::Power),
        text_of(&Window::new(Lang::En), Key::Power)
    );
}

#[cfg(feature = "common-machining")]
mod common {
    use super::*;
    use eh3d_materials::machining::{LaserKind, LaserMode, MachiningDatabase};
    use eh3d_phoenix_laser::commonmat::{Kind, What};

    /// A material, a power and a thickness that the database has a diode cut for.
    fn known() -> (String, f64, f64, f64, f64, u32) {
        let db = MachiningDatabase::embedded().unwrap();
        for w in db.laser_cuttable() {
            for p in &w.laser {
                if p.kind == LaserKind::Diode && p.mode == LaserMode::Cut {
                    if let Some(t) = p.thickness_mm {
                        return (
                            w.id.clone(),
                            p.power_w,
                            t,
                            p.power_pct,
                            p.speed_mm_min,
                            p.passes,
                        );
                    }
                }
            }
        }
        panic!("no diode cut in the database");
    }

    #[test]
    fn the_materials_of_the_database_are_listed_in_the_language_of_the_window() {
        let mut w = Window::new(Lang::En);
        let en = w.common_entries();
        assert!(en.len() > 20);
        w.lang = Lang::Es;
        let es = w.common_entries();
        assert_eq!(en.len(), es.len());
        assert!(en.iter().zip(&es).any(|(a, b)| a.name != b.name));
    }

    #[test]
    fn a_starting_point_needs_the_power_of_the_laser_and_a_thickness_that_is_a_number() {
        let (id, _, t, ..) = known();
        let mut w = Window::new(Lang::En);
        assert!(w.common_answer().is_none());
        w.common_id = Some(id);
        // no power of the laser in the box: no answer, and nothing is applied
        assert!(matches!(
            w.common_answer(),
            Some(Err(Status::BadValue(Msg::LaserWatts)))
        ));
        assert!(!w.use_common_start());
        set(&mut w, Key::LaserWatts, "10");
        w.common_thickness = "abc".to_string();
        assert!(matches!(
            w.common_answer(),
            Some(Err(Status::BadValue(Msg::CommonMatThickness)))
        ));
        w.common_thickness = (-1.0_f64).to_string();
        assert!(matches!(w.common_answer(), Some(Err(_))));
        w.common_thickness = t.to_string();
        assert!(matches!(w.common_answer(), Some(Ok(_))));
    }

    #[test]
    fn a_known_point_goes_into_the_boxes_and_the_table_as_the_database_has_it_and_stays_unverified()
    {
        let (id, watts, t, power, speed, passes) = known();
        let s = Scratch::new("common");
        let file = s.write("plate.lbrn2", PROJECT);
        let mut w = Window::new(Lang::En);
        assert!(w.open_dropped(&file));
        set(&mut w, Key::LaserWatts, &format!("{watts}"));
        w.common_id = Some(id.clone());
        w.common_kind = Kind::Diode;
        w.common_what = What::Cut;
        w.common_thickness = t.to_string();
        let a = w.common_answer().unwrap().unwrap();
        let f = a.found.expect("the database has the point");
        assert!(
            (f.power_percent - power).abs() < 1e-9
                && (f.speed_mm_min - speed).abs() < 1e-9
                && f.passes == passes
        );
        assert!(!f.verified);
        // the answer carries the note that it was not tried on a machine
        assert!(
            a.notes.iter().any(|n| n.contains("not tried")),
            "{:?}",
            a.notes
        );
        assert!(w.use_common_start());
        assert_eq!(w.material.as_deref(), Some(id.as_str()));
        assert!(matches!(w.status, Status::MaterialApplied(ref n) if *n == id));
        assert_eq!(text_of(&w, Key::Power).parse::<f64>().unwrap(), power);
        assert_eq!(text_of(&w, Key::Feed).parse::<f64>().unwrap(), speed);
        for key in ["c00", "c01"] {
            let r = row(&w, key);
            assert_eq!(
                r.fields[LayerField::Power as usize].parse::<f64>().unwrap(),
                power,
                "{key}"
            );
            assert_eq!(
                r.fields[LayerField::Speed as usize].parse::<f64>().unwrap(),
                speed,
                "{key}"
            );
            assert_eq!(r.fields[LayerField::Passes as usize], passes.to_string());
        }
        // the limits of the machine still hold over the values of the database
        assert!(w.stale || w.done.is_none());
    }

    #[test]
    fn a_thickness_the_database_does_not_know_gives_a_warning_and_changes_nothing() {
        let (id, watts, ..) = known();
        let mut w = Window::new(Lang::En);
        set(&mut w, Key::LaserWatts, &format!("{watts}"));
        w.common_id = Some(id);
        w.common_thickness = "900".to_string();
        let before = text_of(&w, Key::Power);
        let a = w.common_answer().unwrap().unwrap();
        assert!(a.found.is_none());
        assert!(a.warning.is_some());
        assert!(!w.use_common_start());
        assert!(w.status.is_problem());
        assert_eq!(text_of(&w, Key::Power), before);
    }

    #[test]
    fn a_material_that_must_not_meet_a_laser_is_refused_with_its_reasons_in_every_language() {
        let mut w = Window::new(Lang::En);
        set(&mut w, Key::LaserWatts, "10");
        let bad = w
            .common_entries()
            .into_iter()
            .find(|e| e.laser_unsafe)
            .expect("an unsafe material");
        w.common_id = Some(bad.id);
        w.common_thickness = "3".to_string();
        for l in Lang::ALL {
            w.lang = l;
            let a = w.common_answer().unwrap().unwrap();
            assert!(a.found.is_none(), "{}", l.code());
            assert!(
                a.warning.as_deref().is_some_and(|t| !t.trim().is_empty()),
                "{}",
                l.code()
            );
            assert!(!a.reasons.is_empty(), "{}", l.code());
        }
        assert!(!w.use_common_start());
    }

    #[test]
    fn engraving_has_no_thickness_to_ask_for() {
        let db = MachiningDatabase::embedded().unwrap();
        let hit = db.materials().iter().find_map(|m| {
            m.laser
                .iter()
                .find(|p| p.kind == LaserKind::Diode && p.mode == LaserMode::Engrave)
                .map(|p| (m.id.clone(), p.power_w))
        });
        let Some((id, watts)) = hit else {
            return;
        };
        let mut w = Window::new(Lang::En);
        set(&mut w, Key::LaserWatts, &format!("{watts}"));
        w.common_id = Some(id);
        w.common_what = What::Engrave;
        w.common_thickness = "not a number".to_string();
        assert!(matches!(w.common_answer(), Some(Ok(_))));
    }
}
