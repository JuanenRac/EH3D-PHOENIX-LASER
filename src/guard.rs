// EH3D-PHOENIX-LASER - the forbidden zones of the bed and the interlocks.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Two more rules that keep the beam where it belongs, both held in the settings file and never in the code. A **forbidden zone** (`[zone NAME]`) is a rectangle of the bed, in millimetres
//! of the machine, where the beam must not burn (`no_burn`: the head may cross it with the laser off, as over a clamp that is lower than the head) or where the head must not go at all
//! (`no_go`: a fixture, a camera, the end of a rail). A program, or a vector job, that reaches a zone fails its check and is not sent. The **interlocks** (`[interlocks]`) are the state of
//! the emergency stop, of the enclosure and of the air assist as they are written in the file: this program reads no sensor, so the file is where a simulator, a gateway or the person
//! says what the machine is doing, and a required interlock that is not in its safe state (a state that is not known counts as unsafe) stops everything that would send or fire.

use crate::preview::Program;
use crate::vector::Pt;

/// A file with more zones than this is refused.
pub const MAX_ZONES: usize = 64;
/// The longest name of a zone.
pub const MAX_ZONE_NAME_CHARS: usize = 80;
/// The largest number (mm) a place or a size of a zone may be.
pub const MAX_ZONE_MM: f64 = 100_000.0;

/// What a zone forbids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKind {
    /// The laser must be off inside the zone; the head may cross it.
    NoBurn,
    /// The head must not enter the zone, with the laser on or off.
    NoGo,
}

impl ZoneKind {
    /// The kind a word names (`no_burn` or `no_go`, with a hyphen or a space instead of the underscore too).
    pub fn from_word(word: &str) -> Option<ZoneKind> {
        match word
            .trim()
            .to_ascii_lowercase()
            .replace(['-', ' '], "_")
            .as_str()
        {
            "no_burn" => Some(ZoneKind::NoBurn),
            "no_go" => Some(ZoneKind::NoGo),
            _ => None,
        }
    }

    /// The word of the settings file.
    pub fn word(self) -> &'static str {
        match self {
            ZoneKind::NoBurn => "no_burn",
            ZoneKind::NoGo => "no_go",
        }
    }
}

/// A rectangle of the bed that a program must keep out of.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// The name of the zone (lower case), as the section of the settings file gives it.
    pub name: String,
    /// The lower left corner, mm of the machine.
    pub x_mm: f64,
    pub y_mm: f64,
    /// The size, mm. A zone with no size is not valid: it would forbid nothing.
    pub width_mm: f64,
    pub height_mm: f64,
    pub kind: ZoneKind,
}

impl Default for Zone {
    fn default() -> Self {
        Zone {
            name: String::new(),
            x_mm: 0.0,
            y_mm: 0.0,
            width_mm: 0.0,
            height_mm: 0.0,
            // the strongest rule is the one a zone has when its kind is not said
            kind: ZoneKind::NoGo,
        }
    }
}

impl Zone {
    /// Whether the zone can be used: a name, a place and a size that are finite numbers inside the limits.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty()
            || self.name.chars().count() > MAX_ZONE_NAME_CHARS
            || self.name.chars().any(char::is_control)
        {
            return Err(format!(
                "a zone has a name of 1 to {MAX_ZONE_NAME_CHARS} characters with no control character"
            ));
        }
        let place = |what: &str, v: f64| {
            if v.is_finite() && v.abs() <= MAX_ZONE_MM {
                Ok(())
            } else {
                Err(format!(
                    "zone {}: {what} is {v}: it must be from -{MAX_ZONE_MM} to {MAX_ZONE_MM}",
                    self.name
                ))
            }
        };
        place("x_mm", self.x_mm)?;
        place("y_mm", self.y_mm)?;
        let size = |what: &str, v: f64| {
            if v.is_finite() && v > 0.0 && v <= MAX_ZONE_MM {
                Ok(())
            } else {
                Err(format!(
                    "zone {}: {what} is {v}: it must be more than 0 and up to {MAX_ZONE_MM}",
                    self.name
                ))
            }
        };
        size("width_mm", self.width_mm)?;
        size("height_mm", self.height_mm)
    }

    /// Whether the straight move from `a` to `b` touches the zone (its edges count as inside, so a move along an edge is a hit).
    pub fn hits(&self, a: Pt, b: Pt) -> bool {
        let (x0, x1) = (self.x_mm, self.x_mm + self.width_mm);
        let (y0, y1) = (self.y_mm, self.y_mm + self.height_mm);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        for (p, q) in [
            (-dx, a.0 - x0),
            (dx, x1 - a.0),
            (-dy, a.1 - y0),
            (dy, y1 - a.1),
        ] {
            if p == 0.0 {
                if q < 0.0 {
                    return false;
                }
            } else {
                let t = q / p;
                if p < 0.0 {
                    if t > t1 {
                        return false;
                    }
                    t0 = t0.max(t);
                } else {
                    if t < t0 {
                        return false;
                    }
                    t1 = t1.min(t);
                }
            }
        }
        t0 <= t1
    }

    /// Whether a path (closed ones come back to their first point) touches the zone.
    pub fn hits_path(&self, pts: &[Pt], closed: bool) -> bool {
        if let [only] = pts {
            return self.hits(*only, *only);
        }
        let closing = closed
            .then(|| pts.first().zip(pts.last()))
            .flatten()
            .is_some_and(|(first, last)| self.hits(*last, *first));
        closing || pts.windows(2).any(|w| self.hits(w[0], w[1]))
    }

    /// The zone as a sentence: its name, its kind and its place.
    pub fn describe(&self) -> String {
        format!(
            "{} ({}, {},{} to {},{} mm)",
            self.name,
            self.kind.word(),
            self.x_mm,
            self.y_mm,
            self.x_mm + self.width_mm,
            self.y_mm + self.height_mm
        )
    }
}

/// What a program does inside the zones, one problem for each zone that it reaches: a burning move that enters a `no_burn` zone, any move that enters a `no_go` zone.
pub fn program_problems(program: &Program, zones: &[Zone]) -> Vec<String> {
    let mut out = Vec::new();
    for z in zones {
        let mut count = 0usize;
        let mut first: Option<usize> = None;
        for m in &program.moves {
            let forbidden = match z.kind {
                ZoneKind::NoBurn => m.power > 0.0,
                ZoneKind::NoGo => true,
            };
            if forbidden && z.hits(m.from, m.to) {
                count += 1;
                first.get_or_insert(m.line);
            }
        }
        if let Some(line) = first {
            out.push(match z.kind {
                ZoneKind::NoBurn => format!(
                    "the program burns inside the forbidden zone {}: {count} move(s), the first at line {line}",
                    z.describe()
                ),
                ZoneKind::NoGo => format!(
                    "the head goes into the no-go zone {}: {count} move(s), the first at line {line}",
                    z.describe()
                ),
            });
        }
    }
    out
}

/// What the paths of a vector job do inside the zones (a job burns or cuts every path, so both kinds of zone forbid it): the layer of the first path that reaches each zone.
pub fn path_problems<'a>(
    paths: impl Iterator<Item = (&'a str, &'a [Pt], bool)> + Clone,
    zones: &[Zone],
) -> Vec<String> {
    let mut out = Vec::new();
    for z in zones {
        let mut hit = paths
            .clone()
            .filter(|(_, pts, closed)| z.hits_path(pts, *closed));
        if let Some((layer, _, _)) = hit.next() {
            let more = hit.count();
            out.push(format!(
                "the layer {layer} reaches the forbidden zone {}{}",
                z.describe(),
                if more > 0 {
                    format!(" (and {more} more path(s))")
                } else {
                    String::new()
                }
            ));
        }
    }
    out
}

/// The state of the emergency stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estop {
    Released,
    Pressed,
}

/// The state of the enclosure (the lid of the machine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enclosure {
    Unknown,
    Closed,
    Open,
}

/// The state of the air assist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirAssist {
    Unknown,
    On,
    Off,
}

/// The interlocks as the settings file says them. By default nothing is required and the emergency stop is released, so a machine with no `[interlocks]` section behaves as before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interlocks {
    pub estop: Estop,
    pub enclosure: Enclosure,
    pub air_assist: AirAssist,
    /// Nothing is sent or fired unless the enclosure is closed.
    pub require_enclosure: bool,
    /// Nothing is sent or fired unless the air assist is on.
    pub require_air_assist: bool,
}

impl Default for Interlocks {
    fn default() -> Self {
        Interlocks {
            estop: Estop::Released,
            enclosure: Enclosure::Unknown,
            air_assist: AirAssist::Unknown,
            require_enclosure: false,
            require_air_assist: false,
        }
    }
}

impl Interlocks {
    /// The emergency stop a word names (`released` or `pressed`).
    pub fn estop_from_word(word: &str) -> Option<Estop> {
        match word.trim().to_ascii_lowercase().as_str() {
            "released" => Some(Estop::Released),
            "pressed" => Some(Estop::Pressed),
            _ => None,
        }
    }

    /// The enclosure a word names (`closed`, `open` or `unknown`).
    pub fn enclosure_from_word(word: &str) -> Option<Enclosure> {
        match word.trim().to_ascii_lowercase().as_str() {
            "closed" => Some(Enclosure::Closed),
            "open" => Some(Enclosure::Open),
            "unknown" => Some(Enclosure::Unknown),
            _ => None,
        }
    }

    /// The air assist a word names (`on`, `off` or `unknown`).
    pub fn air_assist_from_word(word: &str) -> Option<AirAssist> {
        match word.trim().to_ascii_lowercase().as_str() {
            "on" => Some(AirAssist::On),
            "off" => Some(AirAssist::Off),
            "unknown" => Some(AirAssist::Unknown),
            _ => None,
        }
    }

    /// What stops the machine: the emergency stop that is pressed, and a required interlock that is not in its safe state (a state that is not known is not safe). Empty when nothing does.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.estop == Estop::Pressed {
            out.push(
                "the emergency stop is pressed (estop of [interlocks]): nothing is sent or fired until it is released"
                    .to_string(),
            );
        }
        if self.require_enclosure && self.enclosure != Enclosure::Closed {
            out.push(format!(
                "the enclosure must be closed (require_enclosure of [interlocks]) and it is {}",
                match self.enclosure {
                    Enclosure::Open => "open",
                    _ => "not known to be closed",
                }
            ));
        }
        if self.require_air_assist && self.air_assist != AirAssist::On {
            out.push(format!(
                "the air assist must be on (require_air_assist of [interlocks]) and it is {}",
                match self.air_assist {
                    AirAssist::Off => "off",
                    _ => "not known to be on",
                }
            ));
        }
        out
    }

    /// Whether anything is required or pressed: a machine with the defaults has nothing to check.
    pub fn is_active(&self) -> bool {
        self.estop == Estop::Pressed || self.require_enclosure || self.require_air_assist
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::read;

    fn zone(x: f64, y: f64, w: f64, h: f64, kind: ZoneKind) -> Zone {
        Zone {
            name: "clamp".to_string(),
            x_mm: x,
            y_mm: y,
            width_mm: w,
            height_mm: h,
            kind,
        }
    }

    #[test]
    fn a_move_that_crosses_touches_or_ends_in_a_zone_is_a_hit_and_one_that_misses_is_not() {
        let z = zone(10.0, 10.0, 10.0, 10.0, ZoneKind::NoGo);
        // across it
        assert!(z.hits((0.0, 15.0), (30.0, 15.0)));
        assert!(z.hits((15.0, 0.0), (15.0, 30.0)));
        // corner to corner
        assert!(z.hits((0.0, 0.0), (30.0, 30.0)));
        // starting, or ending, inside
        assert!(z.hits((15.0, 15.0), (50.0, 50.0)));
        assert!(z.hits((50.0, 50.0), (12.0, 12.0)));
        // along an edge, and touching a corner: the edges count as inside
        assert!(z.hits((0.0, 10.0), (30.0, 10.0)));
        assert!(z.hits((20.0, 0.0), (20.0, 30.0)));
        assert!(z.hits((0.0, 20.0), (10.0, 20.0)));
        // a point is a move that does not go anywhere
        assert!(z.hits((15.0, 15.0), (15.0, 15.0)));
        assert!(!z.hits((5.0, 5.0), (5.0, 5.0)));
        // misses: beside, above, below, a diagonal that passes the corner
        assert!(!z.hits((0.0, 9.9), (30.0, 9.9)));
        assert!(!z.hits((0.0, 20.1), (30.0, 20.1)));
        assert!(!z.hits((9.9, 0.0), (9.9, 30.0)));
        assert!(!z.hits((0.0, 8.0), (8.0, 0.0)));
        assert!(!z.hits((0.0, 25.0), (25.0, 30.0)));
    }

    #[test]
    fn a_path_is_a_hit_when_one_of_its_pieces_is_and_a_closed_path_counts_its_closing_side() {
        let z = zone(10.0, 10.0, 5.0, 5.0, ZoneKind::NoBurn);
        let open = [(0.0, 0.0), (0.0, 30.0), (30.0, 30.0)];
        assert!(!z.hits_path(&open, false));
        // closing the path draws the side from (30, 30) back to (0, 0), which crosses the zone
        assert!(z.hits_path(&open, true));
        assert!(z.hits_path(&[(12.0, 12.0)], false));
        assert!(!z.hits_path(&[(1.0, 1.0)], false));
        assert!(!z.hits_path(&[], true));
    }

    #[test]
    fn a_zone_needs_a_name_a_place_and_a_size_and_nothing_that_is_not_a_number() {
        assert!(zone(0.0, 0.0, 1.0, 1.0, ZoneKind::NoGo).validate().is_ok());
        assert!(zone(-5.0, -5.0, 1.0, 1.0, ZoneKind::NoGo)
            .validate()
            .is_ok());
        for bad in [
            zone(0.0, 0.0, 0.0, 1.0, ZoneKind::NoGo),
            zone(0.0, 0.0, 1.0, -1.0, ZoneKind::NoGo),
            zone(f64::NAN, 0.0, 1.0, 1.0, ZoneKind::NoGo),
            zone(0.0, f64::INFINITY, 1.0, 1.0, ZoneKind::NoGo),
            zone(0.0, 0.0, f64::NAN, 1.0, ZoneKind::NoGo),
            zone(0.0, 0.0, 1.0, 100_001.0, ZoneKind::NoGo),
            zone(100_001.0, 0.0, 1.0, 1.0, ZoneKind::NoGo),
            Zone::default(),
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
        let mut nameless = zone(0.0, 0.0, 1.0, 1.0, ZoneKind::NoGo);
        nameless.name.clear();
        assert!(nameless.validate().is_err());
        nameless.name = "a\u{7}b".to_string();
        assert!(nameless.validate().is_err());
        nameless.name = "x".repeat(MAX_ZONE_NAME_CHARS + 1);
        assert!(nameless.validate().is_err());
        // a zone with no kind said is the strongest one
        assert_eq!(Zone::default().kind, ZoneKind::NoGo);
    }

    #[test]
    fn the_kinds_are_read_with_their_variants_and_name_themselves() {
        for kind in [ZoneKind::NoBurn, ZoneKind::NoGo] {
            assert_eq!(ZoneKind::from_word(kind.word()), Some(kind));
        }
        assert_eq!(ZoneKind::from_word(" No-Burn "), Some(ZoneKind::NoBurn));
        assert_eq!(ZoneKind::from_word("NO GO"), Some(ZoneKind::NoGo));
        assert_eq!(ZoneKind::from_word("keep out"), None);
        assert_eq!(ZoneKind::from_word(""), None);
    }

    #[test]
    fn a_no_burn_zone_lets_the_head_cross_with_the_laser_off_and_a_no_go_zone_does_not() {
        // burn across the zone, then travel across it
        let burn = "G90\nF1000\nM4 S100\nG1 X0 Y15\nG1 X30 Y15\nM5\nM2\n";
        let travel = "G90\nG0 X0 Y15\nG0 X30 Y15\nM5\nM2\n";
        let no_burn = [zone(10.0, 10.0, 10.0, 10.0, ZoneKind::NoBurn)];
        let no_go = [zone(10.0, 10.0, 10.0, 10.0, ZoneKind::NoGo)];
        let p = program_problems(&read(burn), &no_burn);
        assert_eq!(p.len(), 1);
        assert!(
            p[0].contains("burns inside the forbidden zone clamp"),
            "{p:?}"
        );
        assert!(p[0].contains("line 5"), "{p:?}");
        assert!(program_problems(&read(travel), &no_burn).is_empty());
        let q = program_problems(&read(travel), &no_go);
        assert_eq!(q.len(), 1);
        assert!(
            q[0].contains("no-go zone clamp") && q[0].contains("line 3"),
            "{q:?}"
        );
        assert_eq!(program_problems(&read(burn), &no_go).len(), 1);
        // a program that stays away is clean, and no zones is always clean
        let away = "G90\nF1000\nM4 S100\nG1 X0 Y0\nG1 X5 Y5\nM5\nM2\n";
        assert!(program_problems(&read(away), &no_go).is_empty());
        assert!(program_problems(&read(burn), &[]).is_empty());
    }

    #[test]
    fn each_zone_is_told_once_with_how_many_moves_reach_it() {
        let zones = [
            zone(10.0, 10.0, 10.0, 10.0, ZoneKind::NoGo),
            Zone {
                name: "camera".to_string(),
                ..zone(100.0, 0.0, 5.0, 5.0, ZoneKind::NoGo)
            },
        ];
        let text = "G90\nF1000\nM4 S100\nG1 X0 Y15\nG1 X30 Y15\nG1 X30 Y12\nG1 X5 Y12\nG1 X102 Y2\nM5\nM2\n";
        let p = program_problems(&read(text), &zones);
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].contains("3 move(s)"), "{p:?}");
        assert!(
            p[1].contains("camera") && p[1].contains("1 move(s)"),
            "{p:?}"
        );
    }

    #[test]
    fn the_arc_of_a_program_is_checked_where_it_really_goes() {
        // the two ends are outside the zone but the arc between them bulges into it
        let zones = [zone(0.0, 4.0, 10.0, 3.0, ZoneKind::NoBurn)];
        let arc = "G90\nF1000\nM4 S100\nG0 X0 Y0\nG3 X10 Y0 I5 J0\nM5\nM2\n";
        // counter-clockwise from (0, 0) about (5, 0): the arc goes below y = 0 and never up to 4
        assert!(program_problems(&read(arc), &zones).is_empty());
        let over = "G90\nF1000\nM4 S100\nG0 X0 Y0\nG2 X10 Y0 I5 J0\nM5\nM2\n";
        // clockwise it goes over, up to y = 5, through the zone
        assert_eq!(program_problems(&read(over), &zones).len(), 1);
    }

    #[test]
    fn the_paths_of_a_vector_job_are_checked_against_every_kind_of_zone() {
        let a: Vec<Pt> = vec![(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)];
        let b: Vec<Pt> = vec![(12.0, 12.0), (14.0, 12.0)];
        let c: Vec<Pt> = vec![(13.0, 13.0), (14.0, 14.0)];
        let paths = [
            ("outline", &a[..], true),
            ("hole", &b[..], false),
            ("dot", &c[..], false),
        ];
        let zones = [zone(10.0, 10.0, 10.0, 10.0, ZoneKind::NoBurn)];
        let p = path_problems(paths.iter().copied(), &zones);
        assert_eq!(p.len(), 1);
        assert!(
            p[0].contains("layer hole") && p[0].contains("and 1 more"),
            "{p:?}"
        );
        assert!(path_problems(paths.iter().copied(), &[]).is_empty());
        let far = [zone(100.0, 100.0, 1.0, 1.0, ZoneKind::NoGo)];
        assert!(path_problems(paths.iter().copied(), &far).is_empty());
    }

    #[test]
    fn with_nothing_said_the_interlocks_stop_nothing() {
        let i = Interlocks::default();
        assert!(i.problems().is_empty());
        assert!(!i.is_active());
        assert_eq!(i.estop, Estop::Released);
    }

    #[test]
    fn a_pressed_emergency_stop_stops_everything() {
        let i = Interlocks {
            estop: Estop::Pressed,
            ..Interlocks::default()
        };
        let p = i.problems();
        assert_eq!(p.len(), 1);
        assert!(p[0].contains("emergency stop is pressed"), "{p:?}");
        assert!(i.is_active());
    }

    #[test]
    fn a_required_interlock_must_be_in_its_safe_state_and_a_state_not_known_is_not_safe() {
        let mut i = Interlocks {
            require_enclosure: true,
            require_air_assist: true,
            ..Interlocks::default()
        };
        // nothing is known
        let p = i.problems();
        assert_eq!(p.len(), 2, "{p:?}");
        assert!(p[0].contains("not known to be closed") && p[1].contains("not known to be on"));
        // what is known to be wrong
        i.enclosure = Enclosure::Open;
        i.air_assist = AirAssist::Off;
        let p = i.problems();
        assert!(p[0].contains("is open") && p[1].contains("is off"), "{p:?}");
        // safe
        i.enclosure = Enclosure::Closed;
        i.air_assist = AirAssist::On;
        assert!(i.problems().is_empty());
        // an interlock that is not required is not looked at
        let loose = Interlocks {
            enclosure: Enclosure::Open,
            air_assist: AirAssist::Off,
            ..Interlocks::default()
        };
        assert!(loose.problems().is_empty());
    }

    #[test]
    fn the_words_of_the_interlocks_are_read_in_any_case() {
        assert_eq!(
            Interlocks::estop_from_word(" Pressed "),
            Some(Estop::Pressed)
        );
        assert_eq!(
            Interlocks::estop_from_word("RELEASED"),
            Some(Estop::Released)
        );
        assert_eq!(Interlocks::estop_from_word("maybe"), None);
        assert_eq!(
            Interlocks::enclosure_from_word("Closed"),
            Some(Enclosure::Closed)
        );
        assert_eq!(
            Interlocks::enclosure_from_word("open"),
            Some(Enclosure::Open)
        );
        assert_eq!(
            Interlocks::enclosure_from_word("unknown"),
            Some(Enclosure::Unknown)
        );
        assert_eq!(Interlocks::enclosure_from_word("shut"), None);
        assert_eq!(Interlocks::air_assist_from_word("ON"), Some(AirAssist::On));
        assert_eq!(
            Interlocks::air_assist_from_word("off"),
            Some(AirAssist::Off)
        );
        assert_eq!(Interlocks::air_assist_from_word("auto"), None);
    }
}
