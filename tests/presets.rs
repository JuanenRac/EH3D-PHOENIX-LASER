//! Presets of a material and a thickness, given to a layer by copy (assign) or by link: a change of a preset reaches the layers linked to it and says which, a change that any of them cannot take
//! is refused as a whole, a value over a limit of the machine is never lowered in silence, and a starting point that nobody tried on a machine carries its warning in the eight languages. The
//! values of the common database of materials are used exactly as the database gives them (with the feature `common-machining`). The random series use a fixed seed.

use eh3d_phoenix_laser::commonmat::{Found, What};
use eh3d_phoenix_laser::lang::Lang;
use eh3d_phoenix_laser::presets::{Book, Preset, Target};
use eh3d_phoenix_laser::vconfig::{parse, LayerMode, Settings};

fn settings(extra: &str) -> Settings {
    parse(&format!(
        "{extra}[layer red]\nmode = cut\npower = 50\nspeed = 500\n[layer blue]\nmode = engrave\npower = 20\nspeed = 2000\nhatch_spacing = 0.3\n[sublayer red/deep]\npower = 45\n"
    ))
    .unwrap_or_else(|e| panic!("{e}"))
}

fn preset(name: &str, power: f64, speed: f64, passes: u32) -> Preset {
    Preset {
        name: name.to_string(),
        material: "wood/birch-plywood".to_string(),
        thickness_mm: Some(3.0),
        what: What::Cut,
        power_percent: power,
        speed_mm_min: speed,
        passes,
        air_assist: Some(true),
        verified: false,
        alternative: false,
    }
}

fn book() -> Book {
    let mut b = Book::new();
    b.add(preset("Birch 3 mm", 60.0, 300.0, 2)).unwrap();
    b
}

fn params<'a>(s: &'a Settings, t: &Target) -> &'a eh3d_phoenix_laser::vconfig::LayerParams {
    if t.label.is_empty() {
        &s.layers.iter().find(|(n, _)| *n == t.layer).unwrap().1
    } else {
        &s.sublayers
            .iter()
            .find(|x| x.layer == t.layer && x.label == t.label)
            .unwrap()
            .params
    }
}

#[test]
fn an_assigned_preset_is_copied_and_the_layer_stays_its_own() {
    let mut s = settings("");
    let mut b = book();
    let red = Target::layer("Red");
    b.assign(&mut s, &red, " birch 3 MM ", false, Lang::En)
        .unwrap();
    let p = params(&s, &red);
    assert_eq!(
        (p.power_percent, p.speed_mm_min, p.passes, p.mode),
        (60.0, 300.0, 2, LayerMode::Cut)
    );
    // what the preset does not hold is left as it was
    assert_eq!(p.kerf_mm, 0.0);
    let before = s.clone();
    let updates = b
        .change(&mut s, "birch 3 mm", Some(40.0), None, None)
        .unwrap();
    assert!(updates.is_empty());
    assert_eq!(s, before);
    // the preset did change
    assert_eq!(b.find("Birch 3 mm").unwrap().power_percent, 40.0);
}

#[test]
fn a_linked_preset_carries_its_change_to_the_layer_and_says_which_layers_changed() {
    let mut s = settings("");
    let mut b = book();
    let red = Target::layer("red");
    let deep = Target::sublayer("red", "deep");
    b.link(&mut s, &red, "Birch 3 mm", false, Lang::En).unwrap();
    b.link(&mut s, &deep, "Birch 3 mm", false, Lang::En)
        .unwrap();
    let updates = b
        .change(&mut s, "Birch 3 mm", Some(40.0), Some(250.0), None)
        .unwrap();
    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0].target, red);
    assert_eq!(updates[0].from, (60.0, 300.0, 2));
    assert_eq!(updates[0].to, (40.0, 250.0, 2));
    for t in [&red, &deep] {
        let p = params(&s, t);
        assert_eq!(
            (p.power_percent, p.speed_mm_min, p.passes),
            (40.0, 250.0, 2)
        );
    }
    // the layer that was never given the preset does not move
    assert_eq!(params(&s, &Target::layer("blue")).power_percent, 20.0);
    // the change is shown, in every language
    for l in Lang::ALL {
        let text = Book::updated_text("Birch 3 mm", &updates, l);
        assert!(
            text.contains("Birch 3 mm") && text.contains('2') && !text.contains('{'),
            "{text}"
        );
    }
}

#[test]
fn the_book_shows_what_is_linked_and_what_is_assigned() {
    let mut s = settings("");
    let mut b = book();
    b.link(&mut s, &Target::layer("red"), "birch 3 mm", false, Lang::En)
        .unwrap();
    b.assign(
        &mut s,
        &Target::sublayer("red", "deep"),
        "birch 3 mm",
        false,
        Lang::En,
    )
    .unwrap();
    let lines = b.describe(Lang::En);
    assert_eq!(
        lines[0], "red <- Birch 3 mm [linked]: 60 %, 300 mm/min, 2x",
        "{lines:?}"
    );
    assert_eq!(
        lines[1],
        "red/deep <- Birch 3 mm [assigned]: 60 %, 300 mm/min, 2x"
    );
    // one warning for the preset, however many layers hold it
    assert_eq!(lines.len(), 3, "{lines:?}");
    assert!(lines[2].contains("not tried on a machine"), "{lines:?}");
    let es = b.describe(Lang::Es);
    assert!(
        es[0].contains("[vinculado]") && es[1].contains("[asignado]"),
        "{es:?}"
    );
    assert!(es[2].contains("no probado"), "{es:?}");
    for l in Lang::ALL {
        assert!(b.describe(l).len() == 3, "{}", l.code());
    }
    // a preset that somebody tried carries no warning
    let mut tried = preset("tried", 30.0, 800.0, 1);
    tried.verified = true;
    assert!(tried.warning(Lang::De).is_none());
}

#[test]
fn a_change_makes_the_preset_not_tried_again() {
    let mut s = settings("");
    let mut b = Book::new();
    let mut p = preset("kept", 30.0, 800.0, 1);
    p.verified = true;
    b.add(p).unwrap();
    b.link(&mut s, &Target::layer("red"), "kept", false, Lang::En)
        .unwrap();
    assert!(b
        .describe(Lang::En)
        .iter()
        .all(|l| !l.contains("not tried")));
    b.change(&mut s, "kept", Some(31.0), None, None).unwrap();
    assert!(!b.find("kept").unwrap().verified);
    assert!(b.describe(Lang::En).iter().any(|l| l.contains("not tried")));
}

#[test]
fn a_link_that_is_cut_keeps_its_values_and_follows_the_preset_no_more() {
    let mut s = settings("");
    let mut b = book();
    let red = Target::layer("red");
    b.link(&mut s, &red, "birch 3 mm", false, Lang::En).unwrap();
    assert!(b.unlink(&red));
    assert!(!b.unlink(&red), "it was cut already");
    assert!(!b.unlink(&Target::layer("blue")));
    let before = s.clone();
    assert!(b
        .change(&mut s, "birch 3 mm", Some(10.0), None, None)
        .unwrap()
        .is_empty());
    assert_eq!(s, before);
    assert!(!b.binding(&red).unwrap().linked);
    // linking again replaces the binding and does not add a second one
    b.link(&mut s, &red, "birch 3 mm", false, Lang::En).unwrap();
    assert_eq!(b.bindings.len(), 1);
    assert!(b.binding(&red).unwrap().linked);
}

#[test]
fn a_change_that_a_linked_layer_cannot_take_is_refused_and_nothing_moves() {
    let mut s = settings("[machine]\nmax_power_percent = 65\n");
    let mut b = book();
    b.link(&mut s, &Target::layer("red"), "birch 3 mm", false, Lang::En)
        .unwrap();
    b.link(
        &mut s,
        &Target::layer("blue"),
        "birch 3 mm",
        false,
        Lang::En,
    )
    .unwrap();
    let (s0, b0) = (s.clone(), b.clone());
    let e = b
        .change(&mut s, "birch 3 mm", Some(70.0), None, None)
        .unwrap_err();
    assert!(
        e.contains("max_power_percent") && e.contains("nothing was changed"),
        "{e}"
    );
    assert_eq!((&s, &b), (&s0, &b0));
    // the same for a speed over the limit of speed and for a value out of its range
    for (p, v, n) in [
        (None, Some(40_000.0), None),
        (Some(0.0), None, None),
        (None, None, Some(0)),
    ] {
        assert!(b.change(&mut s, "birch 3 mm", p, v, n).is_err());
        assert_eq!((&s, &b), (&s0, &b0));
    }
    assert!(b.change(&mut s, "nothing", Some(10.0), None, None).is_err());
}

#[test]
fn a_preset_over_a_limit_is_refused_with_its_name_and_never_lowered() {
    let mut s = settings("[machine]\nmax_power_percent = 50\n");
    let mut b = book();
    let (s0, b0) = (s.clone(), b.clone());
    let e = b
        .assign(&mut s, &Target::layer("red"), "birch 3 mm", false, Lang::En)
        .unwrap_err();
    assert!(
        e.contains("Birch 3 mm") && e.contains("red") && e.contains("max_power_percent"),
        "{e}"
    );
    assert_eq!((&s, &b), (&s0, &b0));
    // the speed limit and the passes limit too
    b.add(preset("fast", 30.0, 4000.0, 1)).unwrap();
    let mut s = settings("[machine]\nmax_speed_mm_min = 3000\n");
    let e = b
        .link(&mut s, &Target::layer("red"), "fast", false, Lang::En)
        .unwrap_err();
    assert!(e.contains("max_speed_mm_min"), "{e}");
    let mut s = settings("[machine]\nmax_passes = 1\n");
    assert!(b
        .link(&mut s, &Target::layer("red"), "birch 3 mm", false, Lang::En)
        .is_err());
    assert!(b.bindings.is_empty());
}

#[test]
fn a_preset_or_a_layer_that_does_not_exist_is_an_error() {
    let mut s = settings("");
    let mut b = book();
    assert!(b
        .assign(&mut s, &Target::layer("red"), "none", false, Lang::En)
        .unwrap_err()
        .contains("no preset named"));
    assert!(b
        .assign(
            &mut s,
            &Target::layer("green"),
            "birch 3 mm",
            false,
            Lang::En
        )
        .unwrap_err()
        .contains("no layer or sub-layer"));
    assert!(b
        .assign(
            &mut s,
            &Target::sublayer("red", "nope"),
            "birch 3 mm",
            false,
            Lang::En
        )
        .is_err());
    assert!(b.bindings.is_empty());
}

#[test]
fn an_alternative_is_kept_apart_and_is_not_given_unless_it_is_asked_for() {
    let mut s = settings("");
    let mut b = book();
    b.add(preset("second source", 70.0, 200.0, 1).as_alternative())
        .unwrap();
    let e = b
        .assign(
            &mut s,
            &Target::layer("red"),
            "second source",
            false,
            Lang::En,
        )
        .unwrap_err();
    assert!(e.contains("second figure"), "{e}");
    assert!(b.bindings.is_empty());
    // the lists leave it out unless they are asked for it
    assert_eq!(
        b.for_material("Wood/Birch-Plywood", Some(3.0), false).len(),
        1
    );
    assert_eq!(b.for_material("wood/birch-plywood", None, true).len(), 2);
    assert_eq!(
        b.for_material("wood/birch-plywood", Some(4.0), true).len(),
        0
    );
    assert_eq!(b.for_material("metal/steel", None, true).len(), 0);
    // when it is asked for, it is given
    b.assign(
        &mut s,
        &Target::layer("red"),
        "second source",
        true,
        Lang::En,
    )
    .unwrap();
    assert_eq!(params(&s, &Target::layer("red")).power_percent, 70.0);
}

#[test]
fn an_engraving_preset_makes_the_layer_an_engraving_and_keeps_its_hatch() {
    let mut s = settings("");
    let mut b = Book::new();
    let mut p = preset("mark", 15.0, 3000.0, 1);
    p.what = What::Engrave;
    p.thickness_mm = None;
    b.add(p).unwrap();
    b.link(&mut s, &Target::layer("red"), "mark", false, Lang::En)
        .unwrap();
    let r = params(&s, &Target::layer("red"));
    assert_eq!(r.mode, LayerMode::Engrave);
    assert_eq!(r.hatch_spacing_mm, 0.1);
    // the layer that was an engraving keeps its own hatch
    b.assign(&mut s, &Target::layer("blue"), "mark", false, Lang::En)
        .unwrap();
    assert_eq!(params(&s, &Target::layer("blue")).hatch_spacing_mm, 0.3);
}

#[test]
fn the_values_of_a_starting_point_are_kept_exactly_as_they_come() {
    let f = Found {
        power_percent: 62.5,
        speed_mm_min: 421.25,
        passes: 3,
        air_assist: Some(false),
        interpolated: true,
        verified: false,
    };
    let p = Preset::from_found(" wood/x 3 ", "wood/x", Some(3.0), What::Cut, &f);
    assert_eq!(
        (
            p.name.as_str(),
            p.power_percent,
            p.speed_mm_min,
            p.passes,
            p.air_assist,
            p.verified,
            p.alternative
        ),
        ("wood/x 3", 62.5, 421.25, 3, Some(false), false, false)
    );
    let applied = p.apply_to(&eh3d_phoenix_laser::vconfig::LayerParams::default());
    assert_eq!(
        (applied.power_percent, applied.speed_mm_min, applied.passes),
        (62.5, 421.25, 3)
    );
}

#[test]
fn a_book_holds_a_limited_number_of_bindings_and_a_name_once() {
    let mut s = settings("");
    let mut b = book();
    assert!(b.add(preset("BIRCH 3 mm", 10.0, 100.0, 1)).is_err());
    // linking the same target over and over never grows the book
    for _ in 0..50 {
        b.link(&mut s, &Target::layer("red"), "birch 3 mm", false, Lang::En)
            .unwrap();
    }
    assert_eq!(b.bindings.len(), 1);
}

/// A small generator of numbers that gives the same series every time.
struct Lcg(u64);

impl Lcg {
    fn below(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % n
    }
}

#[test]
fn any_series_of_operations_keeps_every_linked_layer_equal_to_its_preset_and_inside_the_limits() {
    let mut rng = Lcg(0x9E57_0001);
    let targets = [
        Target::layer("red"),
        Target::layer("blue"),
        Target::sublayer("red", "deep"),
    ];
    let names = ["a", "b", "c"];
    let mut s = settings("[machine]\nmax_power_percent = 80\nmax_speed_mm_min = 5000\n");
    let mut b = Book::new();
    for (i, n) in names.iter().enumerate() {
        b.add(preset(
            n,
            20.0 + 10.0 * i as f64,
            300.0 + 100.0 * i as f64,
            1 + i as u32,
        ))
        .unwrap();
    }
    let (mut ok, mut refused) = (0, 0);
    for _ in 0..1500 {
        let t = &targets[rng.below(3)];
        let n = names[rng.below(3)];
        let r = match rng.below(4) {
            0 => b.assign(&mut s, t, n, false, Lang::En).map(|_| ()),
            1 => b.link(&mut s, t, n, false, Lang::En).map(|_| ()),
            2 => {
                b.unlink(t);
                Ok(())
            }
            _ => {
                let power = [Some(10.0 + rng.below(90) as f64), None][rng.below(2)];
                let speed = [Some(100.0 + rng.below(6000) as f64), None][rng.below(2)];
                b.change(&mut s, n, power, speed, Some(1 + rng.below(3) as u32))
                    .map(|_| ())
            }
        };
        if r.is_ok() {
            ok += 1;
        } else {
            refused += 1;
        }
        s.validate().unwrap();
        for bind in b.bindings.iter().filter(|x| x.linked) {
            let p = b.find(&bind.preset).unwrap();
            let q = params(&s, &bind.target);
            assert_eq!(
                (q.power_percent, q.speed_mm_min, q.passes),
                (p.power_percent, p.speed_mm_min, p.passes),
                "{} <- {}",
                bind.target.name(),
                p.name
            );
        }
    }
    assert!(ok > 200 && refused > 20, "{ok} {refused}");
}

#[cfg(feature = "common-machining")]
mod with_the_database {
    use super::*;
    use eh3d_materials::machining::{LaserKind, LaserMode, MachiningDatabase};
    use eh3d_phoenix_laser::commonmat::{alternatives, lookup, Kind};

    /// A material, a power and a thickness of a diode cut that the database knows, found by asking it.
    fn a_diode_cut() -> (String, f64, f64) {
        let db = MachiningDatabase::embedded().unwrap();
        for w in db.laser_cuttable() {
            for p in &w.laser {
                if p.kind == LaserKind::Diode && p.mode == LaserMode::Cut && !p.alternative {
                    if let Some(t) = p.thickness_mm {
                        return (w.id.clone(), p.power_w, t);
                    }
                }
            }
        }
        panic!("the database has no diode cut");
    }

    #[test]
    fn a_preset_from_the_database_has_its_values_and_the_warning_that_nobody_tried_them() {
        let (id, watts, t) = a_diode_cut();
        let a = lookup(&id, Kind::Diode, What::Cut, watts, t, Lang::En);
        let f = a.found.expect("the point the database has");
        let p = Preset::from_found("from the database", &id, Some(t), What::Cut, &f);
        assert_eq!(
            (p.power_percent, p.speed_mm_min, p.passes),
            (f.power_percent, f.speed_mm_min, f.passes)
        );
        assert!(!p.verified && !p.alternative);
        let mut s = settings("");
        let mut b = Book::new();
        b.add(p).unwrap();
        // the seed of the database holds values that a weak laser may not take: a limit of the machine refuses them, otherwise they are given
        let given = b.link(
            &mut s,
            &Target::layer("red"),
            "from the database",
            false,
            Lang::Es,
        );
        match given {
            Ok(warnings) => {
                assert!(
                    warnings.iter().any(|w| w.contains("from the database")),
                    "{warnings:?}"
                );
                assert!(b
                    .describe(Lang::Es)
                    .iter()
                    .any(|l| l.contains("no probado")));
            }
            Err(e) => assert!(e.contains("from the database"), "{e}"),
        }
    }

    #[test]
    fn the_second_figures_of_other_sources_are_listed_apart_and_never_become_the_starting_point() {
        let db = MachiningDatabase::embedded().unwrap();
        let mut checked = 0;
        for w in db.laser_cuttable() {
            for p in w
                .laser
                .iter()
                .filter(|p| p.alternative && p.mode == LaserMode::Cut)
            {
                let kind = if p.kind == LaserKind::Diode {
                    Kind::Diode
                } else {
                    Kind::Co2
                };
                let t = p.thickness_mm.unwrap();
                let alts = alternatives(&w.id, kind, What::Cut, p.power_w, t);
                assert!(!alts.is_empty(), "{}", w.id);
                // the answer for the starting point is a main point: a preset made from it is not an alternative
                let main = lookup(&w.id, kind, What::Cut, p.power_w, t, Lang::En);
                if let Some(f) = main.found {
                    let preset = Preset::from_found("x", &w.id, Some(t), What::Cut, &f);
                    assert!(!preset.alternative);
                }
                // and one made from the second figure is refused until it is asked for
                let alt =
                    Preset::from_found("y", &w.id, Some(t), What::Cut, &alts[0]).as_alternative();
                let mut s = settings("");
                let mut b = Book::new();
                b.add(alt).unwrap();
                assert!(b
                    .assign(&mut s, &Target::layer("red"), "y", false, Lang::En)
                    .is_err());
                checked += 1;
            }
        }
        assert!(checked > 0, "the database has no alternative cut");
    }
}
