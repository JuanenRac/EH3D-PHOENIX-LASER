//! The live adjustment of the power and of the speed: only a calculation, held by the range of `[live]`, the limits of the machine, the limit sent to the server and the limit of the pilot,
//! with a warning in the eight languages whenever a value was held. Nothing is sent to a machine. The random cases use a fixed seed.

use eh3d_phoenix_laser::lang::Lang;
use eh3d_phoenix_laser::live::{power, speed, Held, LiveSettings, Quantity, Target};
use eh3d_phoenix_laser::vconfig::{parse, Settings};

fn settings(text: &str) -> Settings {
    parse(text).unwrap_or_else(|e| panic!("{e}"))
}

/// A small generator of numbers that gives the same series every time.
struct Lcg(u64);

impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

#[test]
fn the_default_range_lowers_a_value_and_never_raises_it_at_all() {
    let s = Settings::default();
    assert_eq!(
        s.live,
        LiveSettings {
            power_min_percent: 10.0,
            power_max_percent: 100.0,
            speed_min_percent: 10.0,
            speed_max_percent: 100.0
        }
    );
    let mut rng = Lcg(0x11FE_0001);
    for _ in 0..3000 {
        let programmed = rng.unit() * 100.0;
        let asked = 0.1 + rng.unit() * 300.0;
        let a = power(&s, Target::Program, programmed, asked).unwrap();
        assert!(a.value <= programmed + 1e-9, "{programmed} {asked}: {a:?}");
        assert!(a.value <= s.machine.max_power_percent.min(s.send.max_power_percent) + 1e-9);
        assert!(a.applied_percent <= 100.0 + 1e-9 && a.applied_percent >= 0.0);
        let v = speed(&s, 1.0 + rng.unit() * 20_000.0, asked).unwrap();
        assert!(v.value <= v.programmed + 1e-9, "{v:?}");
    }
}

#[test]
fn a_range_that_allows_more_is_still_held_by_the_machine_and_by_the_server() {
    // the file lets the adjustment go to 200 %: the lowest limit wins and is named
    let wide = "[live]\npower_max_percent = 200\nspeed_max_percent = 200\n";
    let s = settings(&format!(
        "{wide}[machine]\nmax_power_percent = 50\n[send]\nmax_power_percent = 100\n"
    ));
    let a = power(&s, Target::Program, 30.0, 200.0).unwrap();
    assert_eq!((a.value, a.held), (50.0, Some(Held::Machine)));
    assert!((a.applied_percent - 50.0 / 30.0 * 100.0).abs() < 1e-9);
    // the limit sent to the server (40 % unless the file says another) holds under the machine's
    let s = settings(&format!("{wide}[machine]\nmax_power_percent = 90\n"));
    let a = power(&s, Target::Program, 30.0, 200.0).unwrap();
    assert_eq!((a.value, a.held), (40.0, Some(Held::Server)));
    // under both limits the value is what was asked
    let a = power(&s, Target::Program, 20.0, 150.0).unwrap();
    assert_eq!((a.value, a.held), (30.0, None));
    // the speed is held by the limit of speed of the machine
    let s = settings(&format!("{wide}[machine]\nmax_speed_mm_min = 1000\n"));
    let v = speed(&s, 800.0, 200.0).unwrap();
    assert_eq!((v.value, v.held), (1000.0, Some(Held::Machine)));
    assert!((v.applied_percent - 125.0).abs() < 1e-9);
}

#[test]
fn the_pilot_is_held_by_its_own_limit_and_by_the_machine() {
    let wide = "[live]\npower_max_percent = 200\n";
    // the pilot cap of the program is 5 %
    let s = settings(wide);
    let a = power(&s, Target::Pilot, 4.0, 200.0).unwrap();
    assert_eq!((a.value, a.held), (5.0, Some(Held::Pilot)));
    // a pilot cap that a person raises to 20 % holds at 20
    let s = settings(&format!("{wide}[frame]\nmax_pilot_percent = 20\n"));
    let a = power(&s, Target::Pilot, 15.0, 200.0).unwrap();
    assert_eq!((a.value, a.held), (20.0, Some(Held::Pilot)));
    // and a limit of the machine below the pilot cap wins
    let s = settings(&format!(
        "{wide}[frame]\nmax_pilot_percent = 20\n[machine]\nmax_power_percent = 8\n[defaults]\npower = 5\n"
    ));
    let a = power(&s, Target::Pilot, 15.0, 100.0).unwrap();
    assert_eq!((a.value, a.held), (8.0, Some(Held::Machine)));
    // the limit of the server does not hold a pilot, which does not travel as a job
    let s = settings(&format!(
        "{wide}[frame]\nmax_pilot_percent = 20\n[send]\nmax_power_percent = 5\n"
    ));
    let a = power(&s, Target::Pilot, 10.0, 100.0).unwrap();
    assert_eq!((a.value, a.held), (10.0, None));
}

#[test]
fn a_value_that_was_held_says_so_in_every_language_and_one_that_was_not_says_nothing() {
    let s = settings("[live]\npower_max_percent = 200\n");
    let held = power(&s, Target::Program, 30.0, 200.0).unwrap();
    assert_eq!(held.quantity, Quantity::Power);
    let en = held.warning(Lang::En).unwrap();
    assert!(
        en.contains("200.0") && en.contains("133.3") && en.contains("server"),
        "{en}"
    );
    for l in Lang::ALL {
        let w = held.warning(l).unwrap();
        assert!(w.contains("200.0") && w.contains("133.3"), "{w}");
        assert!(!w.contains('{'), "{w}");
        if l != Lang::En {
            assert_ne!(w, en, "{}", l.code());
        }
    }
    let free = power(&s, Target::Program, 10.0, 100.0).unwrap();
    assert!(free.warning(Lang::Es).is_none());
    let slow = speed(&s, 500.0, 1.0).unwrap();
    assert_eq!(slow.held, Some(Held::Range));
    assert!(slow.warning(Lang::Pt).unwrap().contains("velocidade"));
}

#[test]
fn numbers_that_are_not_a_request_are_refused() {
    let s = Settings::default();
    for bad in [0.0, -5.0, f64::NAN, f64::INFINITY, 1_000.1] {
        assert!(power(&s, Target::Program, 20.0, bad).is_err(), "{bad}");
        assert!(speed(&s, 500.0, bad).is_err(), "{bad}");
    }
    for bad in [-1.0, 100.1, f64::NAN, f64::INFINITY] {
        assert!(power(&s, Target::Program, bad, 50.0).is_err(), "{bad}");
    }
    for bad in [0.0, -3.0, f64::NAN, f64::INFINITY] {
        assert!(speed(&s, bad, 50.0).is_err(), "{bad}");
    }
    // no power at all stays no power
    let zero = power(&s, Target::Program, 0.0, 100.0).unwrap();
    assert_eq!(zero.value, 0.0);
}

#[test]
fn the_range_of_the_settings_file_is_checked_with_its_line() {
    let s = settings("[live]\npower_min_percent = 5\npower_max_percent = 150\nspeed_min_percent = 20\nspeed_max_percent = 120\n");
    assert_eq!(s.live.power_max_percent, 150.0);
    for (text, want) in [
        ("[live]\npower_min_percent = 0.5\n", "power_min_percent"),
        ("[live]\npower_max_percent = 201\n", "power_max_percent"),
        ("[live]\nspeed_min_percent = 150\n", "speed_min_percent"),
        (
            "[live]\npower_min_percent = 50\npower_max_percent = 40\n",
            "more than",
        ),
        ("[live]\nspeed_max_percent = x\n", "line 2"),
        ("[live]\nspeed_top = 100\n", "line 2"),
    ] {
        let e = parse(text).unwrap_err();
        assert!(e.contains(want), "{text:?}: {e}");
    }
}

#[test]
fn damaged_ranges_never_panic_and_nothing_read_goes_over_a_limit() {
    let mut rng = Lcg(0x11FE_0002);
    let words = [
        "power_min_percent",
        "power_max_percent",
        "speed_min_percent",
        "speed_max_percent",
    ];
    let (mut accepted, mut refused) = (0, 0);
    for _ in 0..800 {
        let mut text =
            String::from("[machine]\nmax_power_percent = 70\nmax_speed_mm_min = 6000\n[live]\n");
        for _ in 0..(1 + (rng.unit() * 4.0) as usize) {
            let k = words[(rng.unit() * 4.0) as usize % 4];
            let v = (rng.unit() * 260.0 - 20.0).round();
            text.push_str(&format!("{k} = {v}\n"));
        }
        match parse(&text) {
            Ok(s) => {
                accepted += 1;
                let programmed = rng.unit() * 100.0;
                let asked = 0.1 + rng.unit() * 500.0;
                let a = power(&s, Target::Program, programmed, asked).unwrap();
                assert!(a.value <= 40.0 + 1e-9, "{a:?}");
                let p = power(&s, Target::Pilot, programmed.min(100.0), asked).unwrap();
                assert!(p.value <= 5.0 + 1e-9, "{p:?}");
                let v = speed(&s, 1.0 + rng.unit() * 20_000.0, asked).unwrap();
                assert!(v.value <= 6000.0 + 1e-9, "{v:?}");
            }
            Err(_) => refused += 1,
        }
    }
    assert!(accepted > 20 && refused > 20, "{accepted} {refused}");
}
