//! The starting points for a laser that the common database of materials of the suite gives (`eh3d-materials`, the family of workpieces that a laser or a router works).
//!
//! The values are used exactly as the database gives them and nothing here adds a figure of its own: a power in percent of the nominal power of the laser, a speed in mm/min and a number of
//! passes, for a kind of laser (diode or CO2), a thickness (when cutting) and the nominal power of the laser in watts. The database says whether anybody tried a point on a machine; the
//! seed of the database never was, so every answer carries the note «starting point, not tried on a machine» in the language of the window, and a material that must never be put under a
//! laser is refused with the reasons the database gives. Outside the known range there is no answer, never an extrapolation.
//!
//! The database is taken with the feature `common-machining`, because the module `machining` of `eh3d-materials` was added to the common repository after the release this program is built
//! against. Without the feature nothing is offered (`available()` is false) and the window says so; with it, the code below is the whole of the integration.

use crate::lang::Lang;
use crate::materials::Material;

/// The kind of laser the starting point is asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Diode,
    Co2,
}

/// What the laser does with the material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum What {
    Cut,
    Engrave,
}

/// A material of the database as the window lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// `<family>/<name>`, the identifier the database uses.
    pub id: String,
    /// The name in the language of the window.
    pub name: String,
    /// The material must never be put under a laser.
    pub laser_unsafe: bool,
}

/// The values of a starting point, as the database gives them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Found {
    /// A percent of the nominal power of the laser.
    pub power_percent: f64,
    pub speed_mm_min: f64,
    pub passes: u32,
    pub air_assist: Option<bool>,
    /// The values come from a line between two known points.
    pub interpolated: bool,
    /// Every point they come from was tried on a machine.
    pub verified: bool,
}

/// What the database answers for a request.
#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub found: Option<Found>,
    /// Why there is no starting point (a material that is refused, a value outside what is known, a material without points), in the language of the window.
    pub warning: Option<String>,
    /// What a person should know about the values, in the language of the window: always the note that the point was not tried on a machine, when it was not.
    pub notes: Vec<String>,
    /// The reasons the database gives to refuse the laser or to take care, in the language of the window.
    pub reasons: Vec<String>,
}

/// The text that says nothing is offered in a build without the database.
pub const UNAVAILABLE: &str = "the common database of materials is not part of this build";

/// A starting point as a material of the library, so that the window can use it as it uses any other: it carries `verified = false` unless the database says it was tried.
pub fn as_material(id: &str, f: &Found) -> Material {
    Material {
        name: id.to_string(),
        note: String::new(),
        power: Some(f.power_percent),
        speed: Some(f.speed_mm_min),
        passes: Some(f.passes),
        verified: Some(f.verified),
        ..Material::default()
    }
}

#[cfg(feature = "common-machining")]
mod real {
    use super::{Answer, Entry, Found, Kind, What};
    use crate::lang::Lang;
    use eh3d_materials::i18n::Lang as MatLang;
    use eh3d_materials::machining::{
        LaserKind, LaserMode, LaserProcess, MachiningDatabase, StartNote,
    };

    fn lang_of(l: Lang) -> MatLang {
        MatLang::from_code(l.code()).unwrap_or(MatLang::En)
    }

    fn database() -> Option<MachiningDatabase> {
        MachiningDatabase::embedded().ok()
    }

    pub fn available() -> bool {
        database().is_some()
    }

    pub fn not_verified_note(lang: Lang) -> String {
        StartNote::NotVerified.text(lang_of(lang)).to_string()
    }

    pub fn entries(lang: Lang) -> Vec<Entry> {
        let l = lang_of(lang);
        database()
            .map(|db| {
                db.materials()
                    .iter()
                    .map(|w| Entry {
                        id: w.id.clone(),
                        name: w.names.get(l).to_string(),
                        laser_unsafe: w.laser_unsafe(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn lookup(
        id: &str,
        kind: Kind,
        what: What,
        watts: f64,
        thickness_mm: f64,
        lang: Lang,
    ) -> Answer {
        let l = lang_of(lang);
        let mut answer = Answer {
            found: None,
            warning: None,
            notes: Vec::new(),
            reasons: Vec::new(),
        };
        let Some(db) = database() else {
            answer.warning = Some(super::UNAVAILABLE.to_string());
            return answer;
        };
        let Some(w) = db.get(id) else {
            answer.warning = Some(format!("{id}: not in the common database of materials"));
            return answer;
        };
        answer.reasons = w.laser_reasons(l).into_iter().map(str::to_string).collect();
        let process = LaserProcess {
            kind: match kind {
                Kind::Diode => LaserKind::Diode,
                Kind::Co2 => LaserKind::Co2,
            },
            mode: match what {
                What::Cut => LaserMode::Cut,
                What::Engrave => LaserMode::Engrave,
            },
        };
        match w.laser_start(watts, thickness_mm, process) {
            Ok(s) => {
                answer.found = Some(Found {
                    power_percent: s.power_pct,
                    speed_mm_min: s.speed_mm_min,
                    passes: s.passes,
                    air_assist: s.air_assist,
                    interpolated: s.interpolated,
                    verified: s.verified,
                });
                answer.notes = s
                    .notes()
                    .into_iter()
                    .map(|n: StartNote| n.text(l).to_string())
                    .collect();
            }
            Err(e) => answer.warning = Some(e.text(l)),
        }
        answer
    }

    /// The second figures that other sources give for the same case: listed apart and never returned by `lookup`.
    pub fn alternatives(
        id: &str,
        kind: Kind,
        what: What,
        watts: f64,
        thickness_mm: f64,
    ) -> Vec<Found> {
        let Some(db) = database() else {
            return Vec::new();
        };
        let Some(w) = db.get(id) else {
            return Vec::new();
        };
        let process = LaserProcess {
            kind: match kind {
                Kind::Diode => LaserKind::Diode,
                Kind::Co2 => LaserKind::Co2,
            },
            mode: match what {
                What::Cut => LaserMode::Cut,
                What::Engrave => LaserMode::Engrave,
            },
        };
        w.laser_alternatives(process)
            .into_iter()
            .filter(|p| (p.power_w - watts).abs() < 1e-6)
            .filter(|p| {
                what == What::Engrave
                    || p.thickness_mm
                        .is_some_and(|t| (t - thickness_mm).abs() < 1e-6)
            })
            .map(|p| Found {
                power_percent: p.power_pct,
                speed_mm_min: p.speed_mm_min,
                passes: p.passes,
                air_assist: p.air_assist,
                interpolated: false,
                verified: p.verified,
            })
            .collect()
    }
}

#[cfg(not(feature = "common-machining"))]
mod real {
    use super::{Answer, Entry, Found, Kind, What};
    use crate::lang::Lang;

    pub fn available() -> bool {
        false
    }

    pub fn not_verified_note(_lang: Lang) -> String {
        String::new()
    }

    pub fn entries(_lang: Lang) -> Vec<Entry> {
        Vec::new()
    }

    pub fn lookup(
        _id: &str,
        _kind: Kind,
        _what: What,
        _watts: f64,
        _thickness_mm: f64,
        _lang: Lang,
    ) -> Answer {
        Answer {
            found: None,
            warning: Some(super::UNAVAILABLE.to_string()),
            notes: Vec::new(),
            reasons: Vec::new(),
        }
    }

    pub fn alternatives(
        _id: &str,
        _kind: Kind,
        _what: What,
        _watts: f64,
        _thickness_mm: f64,
    ) -> Vec<Found> {
        Vec::new()
    }
}

/// Whether this build has the database.
pub fn available() -> bool {
    real::available()
}

/// The note that every starting point of the database carries while nobody tried it on a machine, in a language (empty in a build without the database).
pub fn not_verified_note(lang: Lang) -> String {
    real::not_verified_note(lang)
}

/// The materials of the database, named in a language, in the order the database has them.
pub fn entries(lang: Lang) -> Vec<Entry> {
    real::entries(lang)
}

/// The starting point for a material, a kind of laser, what it does, the nominal power of the laser in watts and, for a cut, the thickness in mm.
pub fn lookup(
    id: &str,
    kind: Kind,
    what: What,
    watts: f64,
    thickness_mm: f64,
    lang: Lang,
) -> Answer {
    real::lookup(id, kind, what, watts, thickness_mm, lang)
}

/// The second figures that other sources give for the same case (the points of the database marked as alternatives), apart from the starting point: they are listed and never used for it.
/// Empty in a build without the database.
pub fn alternatives(id: &str, kind: Kind, what: What, watts: f64, thickness_mm: f64) -> Vec<Found> {
    real::alternatives(id, kind, what, watts, thickness_mm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_the_database_nothing_is_offered_and_the_answer_says_why() {
        if available() {
            return;
        }
        assert!(entries(Lang::En).is_empty());
        let a = lookup(
            "wood/birch-plywood",
            Kind::Diode,
            What::Cut,
            10.0,
            3.0,
            Lang::En,
        );
        assert!(a.found.is_none());
        assert_eq!(a.warning.as_deref(), Some(UNAVAILABLE));
    }

    #[test]
    fn a_starting_point_becomes_a_material_that_keeps_the_values_and_the_unverified_mark() {
        let f = Found {
            power_percent: 62.5,
            speed_mm_min: 420.0,
            passes: 2,
            air_assist: Some(true),
            interpolated: false,
            verified: false,
        };
        let m = as_material("wood/x", &f);
        assert_eq!(m.name, "wood/x");
        assert_eq!(m.power, Some(62.5));
        assert_eq!(m.speed, Some(420.0));
        assert_eq!(m.passes, Some(2));
        assert_eq!(m.verified, Some(false));
        assert_eq!(m.max_power, None);
        assert_eq!(m.dot, None);
    }

    #[cfg(feature = "common-machining")]
    mod with_the_database {
        use super::*;

        fn first_cuttable() -> (String, f64, f64) {
            // a material, a power and a thickness that the database knows, found by asking it (nothing is assumed here)
            let db = eh3d_materials::machining::MachiningDatabase::embedded().unwrap();
            for w in db.laser_cuttable() {
                for p in &w.laser {
                    if p.kind == eh3d_materials::machining::LaserKind::Diode
                        && p.mode == eh3d_materials::machining::LaserMode::Cut
                    {
                        if let Some(t) = p.thickness_mm {
                            return (w.id.clone(), p.power_w, t);
                        }
                    }
                }
            }
            panic!("the database has no diode cut");
        }

        #[test]
        fn the_database_is_there_and_lists_every_material_in_every_language() {
            assert!(available());
            let db = eh3d_materials::machining::MachiningDatabase::embedded().unwrap();
            for l in Lang::ALL {
                let list = entries(l);
                assert_eq!(list.len(), db.len());
                assert!(list.iter().all(|e| !e.name.trim().is_empty()));
            }
            assert!(entries(Lang::En).iter().any(|e| e.laser_unsafe));
        }

        #[test]
        fn a_known_point_is_given_as_the_database_has_it_with_the_note_that_it_was_not_tried() {
            let (id, watts, thickness) = first_cuttable();
            let a = lookup(&id, Kind::Diode, What::Cut, watts, thickness, Lang::En);
            let f = a.found.expect("the point the database has");
            assert!(f.power_percent > 0.0 && f.power_percent <= 100.0);
            assert!(f.speed_mm_min > 0.0 && f.passes >= 1);
            // the seed never says a point was tried on a machine, and the answer says it in the language of the window
            assert!(!f.verified);
            assert!(a.warning.is_none());
            assert!(
                a.notes.iter().any(|n| n.contains("not tried")),
                "{:?}",
                a.notes
            );
            let es = lookup(&id, Kind::Diode, What::Cut, watts, thickness, Lang::Es);
            assert!(
                es.notes.iter().any(|n| n.contains("no probado")),
                "{:?}",
                es.notes
            );
            // the same figures come back
            assert_eq!(es.found, a.found);
        }

        #[test]
        fn a_thickness_outside_what_is_known_is_refused_and_never_extrapolated() {
            let (id, watts, _) = first_cuttable();
            let a = lookup(&id, Kind::Diode, What::Cut, watts, 500.0, Lang::En);
            assert!(a.found.is_none());
            assert!(a
                .warning
                .as_deref()
                .is_some_and(|w| w.contains("extrapolated")));
        }

        #[test]
        fn a_material_that_must_not_meet_a_laser_is_refused_with_the_reasons() {
            let unsafe_one = entries(Lang::En)
                .into_iter()
                .find(|e| e.laser_unsafe)
                .expect("the database has an unsafe material");
            let a = lookup(&unsafe_one.id, Kind::Diode, What::Cut, 10.0, 3.0, Lang::En);
            assert!(a.found.is_none());
            assert!(a.warning.is_some());
            assert!(!a.reasons.is_empty());
        }

        #[test]
        fn an_unknown_identifier_is_refused() {
            let a = lookup(
                "nothing/here",
                Kind::Co2,
                What::Engrave,
                40.0,
                0.0,
                Lang::En,
            );
            assert!(a.found.is_none());
            assert!(a.warning.is_some());
        }
    }
}
