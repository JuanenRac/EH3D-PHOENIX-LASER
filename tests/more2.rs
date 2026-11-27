//! The limits of the engraving: every value of the parameters at its edge, the numbers the engraving reports (rows, runs, length, time) and the shape of the lines at the ends.

use eh3d_phoenix_laser::dither::Dots;
use eh3d_phoenix_laser::raster::{engrave, overscan_for, Params};

fn dots(rows: &[&str]) -> Dots {
    let (h, w) = (rows.len(), rows[0].len());
    Dots {
        w,
        h,
        burn: rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect(),
    }
}

fn p() -> Params {
    Params {
        pixel_mm: 1.0,
        feed_mm_min: 600.0,
        s_max: 1000,
        power_percent: 50.0,
        overscan_mm: 0.0,
        origin: (0.0, 0.0),
    }
}

fn with(f: impl Fn(&mut Params)) -> Params {
    let mut q = p();
    f(&mut q);
    q
}

#[test]
fn every_value_of_the_parameters_is_accepted_at_its_edges_and_refused_one_step_past_them() {
    for ok in [
        with(|q| q.pixel_mm = 0.01),
        with(|q| q.pixel_mm = 5.0),
        with(|q| q.feed_mm_min = 1.0),
        with(|q| q.feed_mm_min = 30_000.0),
        with(|q| q.power_percent = 0.0),
        with(|q| q.power_percent = 100.0),
        with(|q| q.overscan_mm = 50.0),
        with(|q| q.origin = (-100_000.0, 100_000.0)),
        with(|q| q.s_max = 1),
        with(|q| q.s_max = 100_000),
    ] {
        assert!(ok.validate().is_ok(), "{ok:?}");
    }
    for (bad, name) in [
        (with(|q| q.pixel_mm = 0.0099), "pixel_mm"),
        (with(|q| q.pixel_mm = 5.01), "pixel_mm"),
        (with(|q| q.feed_mm_min = 0.99), "feed_mm_min"),
        (with(|q| q.feed_mm_min = 30_000.1), "feed_mm_min"),
        (with(|q| q.power_percent = -0.1), "power_percent"),
        (with(|q| q.power_percent = 100.1), "power_percent"),
        (with(|q| q.overscan_mm = -0.1), "overscan_mm"),
        (with(|q| q.overscan_mm = 50.1), "overscan_mm"),
        (with(|q| q.origin.0 = 100_001.0), "origin x"),
        (with(|q| q.origin.1 = -100_001.0), "origin y"),
        (with(|q| q.s_max = 0), "s_max"),
        (with(|q| q.s_max = 100_001), "s_max"),
    ] {
        let e = bad.validate().unwrap_err();
        assert!(e.contains(name), "{name}: {e}");
    }
}

#[test]
fn a_value_that_is_not_a_number_is_refused_in_every_field() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for q in [
            with(|q| q.pixel_mm = bad),
            with(|q| q.feed_mm_min = bad),
            with(|q| q.power_percent = bad),
            with(|q| q.overscan_mm = bad),
            with(|q| q.origin.0 = bad),
            with(|q| q.origin.1 = bad),
        ] {
            assert!(q.validate().is_err(), "{q:?}");
        }
    }
}

#[test]
fn the_power_is_zero_at_zero_percent_the_whole_s_max_at_a_hundred_and_rounds_down_in_between() {
    assert_eq!(with(|q| q.power_percent = 0.0).power(), 0);
    assert_eq!(with(|q| q.power_percent = 100.0).power(), 1000);
    assert_eq!(
        with(|q| {
            q.s_max = 255;
            q.power_percent = 50.0
        })
        .power(),
        127
    );
    assert_eq!(
        with(|q| {
            q.s_max = 1;
            q.power_percent = 49.0
        })
        .power(),
        0
    );
    assert_eq!(
        with(|q| {
            q.s_max = 1;
            q.power_percent = 50.0
        })
        .power(),
        0
    );
    assert_eq!(Params::default().power(), 500);
}

#[test]
fn nothing_to_burn_is_a_program_that_only_sets_up_and_ends_and_counts_nothing() {
    let e = engrave(&dots(&["...", "..."]), &p()).unwrap();
    assert_eq!((e.rows, e.runs), (0, 0));
    assert_eq!((e.burn_mm, e.seconds), (0.0, 0.0));
    assert_eq!(e.gcode, "G21\nG90\nM4 S0\nF600\nM5\nM2\n");
}

#[test]
fn a_picture_that_is_all_burning_is_one_run_in_every_row_and_the_time_is_the_burning_at_the_feed() {
    let e = engrave(&dots(&["####", "####", "####"]), &p()).unwrap();
    assert_eq!((e.rows, e.runs), (3, 3));
    assert!((e.burn_mm - 12.0).abs() < 1e-9);
    // 600 mm/min is 10 mm/s; no overscan; the head goes back to the start of each row by burning, so only the travel to the first dot counts
    assert!(e.seconds >= 1.2 - 1e-9, "{}", e.seconds);
}

#[test]
fn a_single_dot_is_a_run_of_one_pixel_long() {
    let e = engrave(&dots(&[".#."]), &p()).unwrap();
    assert_eq!((e.rows, e.runs), (1, 1));
    assert!((e.burn_mm - 1.0).abs() < 1e-9);
    assert!(e.gcode.contains("G0 X1 Y0\n"), "{}", e.gcode);
    assert!(e.gcode.contains("G1 X2 S500\n"), "{}", e.gcode);
}

#[test]
fn two_runs_in_a_row_are_two_burning_moves_with_a_move_with_the_laser_off_between() {
    let e = engrave(&dots(&["##..##"]), &p()).unwrap();
    assert_eq!((e.rows, e.runs), (1, 2));
    let lines: Vec<&str> = e.gcode.lines().collect();
    let at = lines
        .iter()
        .position(|l| *l == "G1 X2 S500")
        .expect("first run");
    assert_eq!(lines[at + 1], "G1 X4 S0");
    assert_eq!(lines[at + 2], "G1 X6 S500");
}

#[test]
fn the_overscan_is_added_at_both_ends_of_every_line_and_only_when_it_is_more_than_zero() {
    let with_overscan = engrave(&dots(&["..##.."]), &with(|q| q.overscan_mm = 3.0)).unwrap();
    // the approach is 3 mm before the first dot, and the head runs 3 mm past the last
    assert!(
        with_overscan.gcode.contains("G0 X-1 Y0\n"),
        "{}",
        with_overscan.gcode
    );
    assert!(with_overscan.gcode.contains("G1 X2 S0\n"));
    assert!(
        with_overscan.gcode.contains("G1 X7 S0\n"),
        "{}",
        with_overscan.gcode
    );
    let none = engrave(&dots(&["..##.."]), &p()).unwrap();
    assert!(!none.gcode.contains("G1 X7 S0"));
    assert!(with_overscan.seconds > none.seconds);
}

#[test]
fn the_rows_without_dots_are_skipped_and_the_direction_alternates_only_over_the_rows_that_burn() {
    let e = engrave(&dots(&["##", "..", "..", "##", "##"]), &p()).unwrap();
    assert_eq!(e.rows, 3);
    let starts: Vec<&str> = e.gcode.lines().filter(|l| l.starts_with("G0 ")).collect();
    assert_eq!(starts.len(), 3);
    // the first row goes to the right (it starts at x = 0), the second to the left (it starts at x = 2), the third to the right again
    assert!(starts[0].starts_with("G0 X0 "));
    assert!(starts[1].starts_with("G0 X2 "));
    assert!(starts[2].starts_with("G0 X0 "));
}

#[test]
fn the_top_row_of_the_picture_is_the_highest_y_and_the_origin_moves_everything() {
    let e = engrave(&dots(&["#", ".", "#"]), &with(|q| q.origin = (10.0, 20.0))).unwrap();
    let ys: Vec<&str> = e.gcode.lines().filter(|l| l.starts_with("G0 ")).collect();
    assert_eq!(ys[0], "G0 X10 Y22");
    assert_eq!(ys[1], "G0 X11 Y20");
}

#[test]
fn a_pixel_of_a_fraction_of_a_millimetre_keeps_its_positions_without_noise() {
    let e = engrave(&dots(&["###"]), &with(|q| q.pixel_mm = 0.1)).unwrap();
    assert!(e.gcode.contains("G1 X0.3 S500\n"), "{}", e.gcode);
    assert!(!e.gcode.contains("0.30000000000000004"));
    assert!(!e.gcode.contains("e-"));
}

#[test]
fn a_parameter_that_is_refused_stops_the_engraving_before_any_code_is_made() {
    assert!(engrave(&dots(&["#"]), &with(|q| q.feed_mm_min = 0.0)).is_err());
    assert!(engrave(&dots(&["#"]), &with(|q| q.s_max = 0)).is_err());
}

#[test]
fn the_run_up_grows_with_the_square_of_the_feed_and_falls_with_the_acceleration() {
    let a = overscan_for(3000.0, 1000.0).unwrap();
    assert!((a - 1.25).abs() < 1e-9, "{a}");
    assert!((overscan_for(6000.0, 1000.0).unwrap() - 4.0 * a).abs() < 1e-9);
    assert!((overscan_for(3000.0, 2000.0).unwrap() - a / 2.0).abs() < 1e-9);
    assert!(overscan_for(0.0, 1000.0).is_err());
    assert!(overscan_for(3000.0, 0.0).is_err());
    assert!(overscan_for(f64::NAN, 1000.0).is_err());
}

#[test]
fn the_program_always_starts_with_the_units_the_absolute_mode_and_dynamic_power_and_ends_with_the_laser_off(
) {
    let e = engrave(&dots(&["#.#", ".#."]), &p()).unwrap();
    assert!(e.gcode.starts_with("G21\nG90\nM4 S0\nF600\n"));
    assert!(e.gcode.ends_with("M5\nM2\n"));
    assert_eq!(e.gcode.matches("M4").count(), 1);
    assert_eq!(e.gcode.matches("M5").count(), 1);
}

#[test]
fn the_laser_is_off_whenever_the_head_moves_without_burning() {
    let e = engrave(
        &dots(&["#..#", ".##.", "#..#"]),
        &with(|q| q.overscan_mm = 2.0),
    )
    .unwrap();
    for line in e.gcode.lines() {
        if line.starts_with("G0 ") {
            assert!(!line.contains(" S"), "a rapid with a power: {line}");
        }
    }
}

// Bug hunt (round 4): limits with numbers out of range, the gate before a vector job leaves, answers of the server, ids, energy and zones at their edges.
mod hunt_round4 {
    // Bug hunt: the limits of power and speed with numbers that are not in the range, the gate before a vector job leaves, the answers of the server with huge or wrong values, the ids that go
    // into a path, the energy of a layer and the forbidden zones at their edges.

    use eh3d_phoenix_laser::energy::{describe, hatch_energy_j_per_mm2, line_energy_j_per_mm};
    use eh3d_phoenix_laser::guard::{
        path_problems, AirAssist, Enclosure, Estop, Interlocks, Zone, ZoneKind, MAX_ZONE_MM,
    };
    use eh3d_phoenix_laser::safety::{
        check_power, check_speed, pilot_percent, validate_max, validate_max_pilot,
        validate_max_speed,
    };
    use eh3d_phoenix_laser::server::{
        check_before_send, check_test_fire, check_vector_before_send, job_request, safe_segment,
        test_fire_request, JobInfo, MachineInfo, VectorJob, VectorOp, VectorPath, MAX_LINES,
        MAX_SEND_BYTES,
    };
    use eh3d_phoenix_laser::vconfig::{LayerMode, LayerParams, Settings};
    use eh3d_serverclient::Json;

    fn one_job(power: f64, speed: f64, points: Vec<(f64, f64)>) -> VectorJob {
        let mut job = VectorJob::default();
        job.paths.push(VectorPath {
            layer: "c00".to_string(),
            points,
            closed: false,
        });
        job.ops.push(VectorOp {
            layer: "c00".to_string(),
            mode: "cut",
            power_pct: power,
            speed_mm_min: speed,
            passes: 1,
        });
        job
    }

    fn settings() -> Settings {
        let mut s = Settings::default();
        s.machine.max_power_percent = 100.0;
        s.send.max_power_percent = 100.0;
        s.machine.max_speed_mm_min = 10_000.0;
        s.machine.bed_mm = None;
        s
    }

    fn zone(x: f64, y: f64, w: f64, h: f64) -> Zone {
        Zone {
            name: "z".to_string(),
            x_mm: x,
            y_mm: y,
            width_mm: w,
            height_mm: h,
            ..Zone::default()
        }
    }

    // ---- the limits of power and speed ----

    #[test]
    fn a_negative_power_is_refused_by_the_limit_of_power() {
        for bad in [-0.1, -5.0, -1e9, f64::NEG_INFINITY] {
            assert!(check_power("--power", bad, 50.0).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_negative_speed_is_refused_and_zero_is_not_a_speed_over_the_limit() {
        for bad in [-0.1, -600.0, f64::NEG_INFINITY, f64::NAN, f64::INFINITY] {
            assert!(check_speed("--feed", bad, 3000.0).is_err(), "{bad}");
        }
        assert!(check_speed("--feed", 0.0, 3000.0).is_ok());
        assert!(check_speed("--feed", 3000.0, 3000.0).is_ok());
        assert!(check_speed("--feed", 3000.01, 3000.0).is_err());
    }

    #[test]
    fn a_power_of_minus_zero_and_the_exact_limit_pass() {
        assert!(check_power("p", -0.0, 50.0).is_ok());
        assert!(check_power("p", 50.0, 50.0).is_ok());
        assert!(check_power("p", 0.1, 0.1).is_ok());
    }

    #[test]
    fn a_limit_that_is_not_valid_refuses_every_power_and_speed() {
        for bad in [f64::NAN, 0.0, -1.0, 100.5, f64::INFINITY] {
            assert!(check_power("p", 1.0, bad).is_err(), "{bad}");
        }
        for bad in [f64::NAN, 0.0, 0.99, 30_000.5, f64::INFINITY] {
            assert!(check_speed("s", 1.0, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_limits_of_speed_and_pilot_power_have_exact_edges() {
        assert!(validate_max_speed(1.0).is_ok());
        assert!(validate_max_speed(30_000.0).is_ok());
        assert!(validate_max_speed(0.999_999).is_err());
        assert!(validate_max_speed(30_000.000_001).is_err());
        assert!(validate_max_pilot(0.0).is_ok());
        assert!(validate_max_pilot(100.0).is_ok());
        assert!(validate_max_pilot(-0.000_001).is_err());
        assert!(validate_max_pilot(f64::NAN).is_err());
        assert!(validate_max(0.1).is_ok());
        assert!(validate_max(0.099_999).is_err());
    }

    #[test]
    fn the_pilot_power_stays_off_without_the_confirmation_whatever_the_cap_is() {
        assert_eq!(pilot_percent(0.0, false, 100.0, 5.0), Ok(0.0));
        assert_eq!(pilot_percent(-0.0, false, 100.0, 5.0), Ok(0.0));
        assert!(pilot_percent(0.001, false, 100.0, 100.0).is_err());
    }

    #[test]
    fn the_pilot_power_obeys_the_lower_of_its_cap_and_the_limit_of_the_machine() {
        assert_eq!(pilot_percent(4.0, true, 100.0, 5.0), Ok(4.0));
        assert_eq!(pilot_percent(5.0, true, 100.0, 5.0), Ok(5.0));
        assert!(pilot_percent(5.1, true, 100.0, 5.0).is_err());
        assert!(pilot_percent(4.0, true, 3.0, 5.0).is_err());
        assert!(pilot_percent(0.5, true, 100.0, 0.0).is_err());
    }

    #[test]
    fn a_pilot_power_that_is_not_a_number_or_is_negative_is_refused_even_confirmed() {
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.5] {
            assert!(pilot_percent(bad, true, 100.0, 5.0).is_err(), "{bad}");
            assert!(pilot_percent(bad, false, 100.0, 5.0).is_err(), "{bad}");
        }
    }

    // ---- the gate before a vector job leaves ----

    #[test]
    fn a_vector_job_with_a_power_that_is_not_a_number_is_not_sent() {
        let s = settings();
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let job = one_job(bad, 600.0, vec![(0.0, 0.0), (1.0, 1.0)]);
            assert!(check_vector_before_send(&job, &s).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_vector_job_with_a_speed_that_is_not_a_number_or_not_above_zero_is_not_sent() {
        let s = settings();
        for bad in [f64::NAN, f64::NEG_INFINITY, 0.0, -1.0] {
            let job = one_job(10.0, bad, vec![(0.0, 0.0), (1.0, 1.0)]);
            assert!(check_vector_before_send(&job, &s).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_vector_job_with_a_negative_power_is_not_sent() {
        let job = one_job(-1.0, 600.0, vec![(0.0, 0.0), (1.0, 1.0)]);
        assert!(check_vector_before_send(&job, &settings()).is_err());
    }

    #[test]
    fn a_vector_job_with_a_point_that_is_not_a_number_is_not_sent_even_without_a_bed() {
        let s = settings();
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let job = one_job(10.0, 600.0, vec![(0.0, 0.0), (bad, 1.0)]);
            let e = check_vector_before_send(&job, &s).unwrap_err();
            assert!(e.contains("not numbers"), "{e}");
            let job = one_job(10.0, 600.0, vec![(0.0, bad), (1.0, 1.0)]);
            assert!(check_vector_before_send(&job, &s).is_err());
        }
    }

    #[test]
    fn a_vector_job_at_the_exact_limits_is_sent_and_one_step_past_them_is_not() {
        let mut s = settings();
        s.machine.max_power_percent = 60.0;
        s.send.max_power_percent = 40.0;
        s.machine.max_speed_mm_min = 3000.0;
        s.machine.bed_mm = Some((100.0, 50.0));
        let at = one_job(40.0, 3000.0, vec![(0.0, 0.0), (100.0, 50.0)]);
        assert!(check_vector_before_send(&at, &s).is_ok());
        for over in [
            one_job(40.001, 3000.0, vec![(0.0, 0.0), (1.0, 1.0)]),
            one_job(40.0, 3000.001, vec![(0.0, 0.0), (1.0, 1.0)]),
            one_job(40.0, 3000.0, vec![(0.0, 0.0), (100.001, 1.0)]),
            one_job(40.0, 3000.0, vec![(-0.001, 0.0), (1.0, 1.0)]),
            one_job(40.0, 3000.0, vec![(0.0, 0.0), (1.0, 50.001)]),
        ] {
            assert!(check_vector_before_send(&over, &s).is_err());
        }
    }

    #[test]
    fn every_problem_of_a_vector_job_is_told_in_one_message() {
        let job = one_job(-1.0, 0.0, vec![(f64::NAN, 0.0), (1.0, 1.0)]);
        let e = check_vector_before_send(&job, &settings()).unwrap_err();
        assert!(e.starts_with("the drawing does not pass the check and is not sent"));
        assert!(e.matches("; ").count() >= 2, "{e}");
    }

    #[test]
    fn a_vector_job_that_reaches_a_zone_or_meets_a_pressed_stop_is_not_sent() {
        let mut s = settings();
        s.zones.push(zone(10.0, 10.0, 10.0, 10.0));
        let job = one_job(10.0, 600.0, vec![(0.0, 15.0), (30.0, 15.0)]);
        assert!(check_vector_before_send(&job, &s).is_err());
        let mut s = settings();
        s.interlocks.estop = Estop::Pressed;
        let job = one_job(10.0, 600.0, vec![(0.0, 0.0), (1.0, 1.0)]);
        assert!(check_vector_before_send(&job, &s).is_err());
    }

    // ---- the program that leaves ----

    #[test]
    fn a_program_of_more_lines_than_the_server_takes_is_not_sent() {
        let program = "G0 X0\n".repeat(MAX_LINES + 1);
        let e = check_before_send(&program, &Settings::default()).unwrap_err();
        assert!(e.contains("lines"), "{e}");
    }

    #[test]
    fn a_program_of_more_bytes_than_the_server_takes_is_not_sent() {
        let program = " ".repeat(MAX_SEND_BYTES + 1);
        let e = check_before_send(&program, &Settings::default()).unwrap_err();
        assert!(e.contains("bytes"), "{e}");
    }

    #[test]
    fn a_program_of_garbage_bytes_never_panics_the_check() {
        let junk = "\u{0}\u{7f}G1 X\u{ff}\u{feff} S-1e999 F\r\n\r\r\n;;(((\u{1f600}\n".repeat(50);
        let _ = check_before_send(&junk, &Settings::default());
    }

    // ---- the test fire ----

    #[test]
    fn a_test_fire_has_exact_edges_in_power_and_time() {
        let mut s = settings();
        s.send.max_power_percent = 40.0;
        assert!(check_test_fire(40.0, 10.0, &s).is_ok());
        assert!(check_test_fire(0.001, 0.01, &s).is_ok());
        assert!(check_test_fire(0.0, 1.0, &s).is_err());
        assert!(check_test_fire(40.001, 1.0, &s).is_err());
        assert!(check_test_fire(10.0, 10.000_001, &s).is_err());
        assert!(check_test_fire(10.0, 0.009_999, &s).is_err());
        assert!(check_test_fire(10.0, 0.0, &s).is_err());
        assert!(check_test_fire(10.0, -1.0, &s).is_err());
    }

    #[test]
    fn a_test_fire_is_not_made_with_a_pressed_stop_whatever_the_numbers() {
        let mut s = settings();
        s.interlocks.estop = Estop::Pressed;
        assert!(check_test_fire(10.0, 1.0, &s).is_err());
    }

    #[test]
    fn the_request_of_a_test_fire_reads_back_as_the_numbers_asked() {
        let v = Json::parse(&test_fire_request(12.5, 0.25)).unwrap();
        let text = format!("{v:?}");
        assert!(text.contains("12.5") && text.contains("0.25"), "{text}");
    }

    // ---- the answers of the server and the ids ----

    #[test]
    fn an_id_with_a_path_or_a_strange_character_never_goes_into_a_request() {
        for bad in [
            "",
            "..",
            ".",
            "a/b",
            "a\\b",
            "../x",
            "a b",
            "a%2e",
            "a\n",
            "a\0",
            "é",
            "日本",
            "a;b",
            "a?b",
            "a#b",
            "a:b",
            "@",
            "a@b",
            "user:pw@host",
            "C:\\x",
            "/abs",
        ] {
            assert!(!safe_segment(bad), "{bad:?}");
        }
    }

    #[test]
    fn an_id_of_letters_digits_underscore_and_hyphen_goes_in_up_to_sixty_four_bytes() {
        assert!(safe_segment("a"));
        assert!(safe_segment("Job_1-b"));
        assert!(safe_segment(&"x".repeat(64)));
        assert!(!safe_segment(&"x".repeat(65)));
    }

    #[test]
    fn a_huge_count_of_lines_done_does_not_overflow_the_percent() {
        let j = JobInfo {
            done: u64::MAX,
            total: u64::MAX,
            ..JobInfo::default()
        };
        assert_eq!(j.percent(), 100);
        let j = JobInfo {
            done: 1 << 62,
            total: 1 << 63,
            ..JobInfo::default()
        };
        assert_eq!(j.percent(), 50);
        let j = JobInfo {
            done: u64::MAX,
            total: 1,
            ..JobInfo::default()
        };
        assert_eq!(j.percent(), 100);
    }

    #[test]
    fn the_percent_of_a_job_is_clamped_and_needs_a_total() {
        let at = |done, total| {
            JobInfo {
                done,
                total,
                ..JobInfo::default()
            }
            .percent()
        };
        assert_eq!(at(0, 0), 0);
        assert_eq!(at(5, 0), 0);
        assert_eq!(at(1, 3), 33);
        assert_eq!(at(2, 3), 66);
        assert_eq!(at(3, 3), 100);
        assert_eq!(at(9, 3), 100);
    }

    #[test]
    fn an_answer_with_the_wrong_types_gives_an_empty_job_and_never_panics() {
        for text in [
            "{\"id\":5,\"state\":null,\"done\":\"7\",\"total\":-1,\"message\":[1]}",
            "{}",
            "[]",
            "null",
            "\"text\"",
            "{\"done\":1.5,\"total\":2.5}",
        ] {
            let v = Json::parse(text).unwrap();
            let j = JobInfo::from_json(&v);
            assert_eq!(j.percent(), 0, "{text}");
            assert!(!j.finished() && !j.active() && !j.held(), "{text}");
            let m = MachineInfo::from_json(&v);
            assert!(!m.latched && !m.beam_on, "{text}");
        }
    }

    #[test]
    fn the_state_of_a_job_decides_finished_active_and_held() {
        for (state, finished, active, held) in [
            ("queued", false, false, false),
            ("running", false, true, false),
            ("paused", false, true, true),
            ("done", true, false, false),
            ("failed", true, false, false),
            ("cancelled", true, false, false),
            ("DONE", false, false, false),
            ("", false, false, false),
        ] {
            let j = JobInfo {
                state: state.to_string(),
                ..JobInfo::default()
            };
            assert_eq!(
                (j.finished(), j.active(), j.held()),
                (finished, active, held),
                "{state}"
            );
        }
    }

    #[test]
    fn a_machine_with_a_latched_stop_is_read_from_its_detail() {
        let v =
            Json::parse("{\"id\":\"m1\",\"detail\":{\"estop_latched\":true,\"beam_on\":\"yes\"}}")
                .unwrap();
        let m = MachineInfo::from_json(&v);
        assert_eq!(m.id, "m1");
        assert!(m.latched);
        assert!(!m.beam_on, "a text is not a true");
    }

    #[test]
    fn a_job_request_keeps_a_hostile_name_and_a_program_with_quotes_exactly() {
        let name = "a\"b\\c\nd\te\u{0}f\u{1f600}\u{2028}";
        let gcode = "G1 X1 ;\"quote\" \\ \r\n\u{7}";
        let v = Json::parse(&job_request("m1", name, gcode)).unwrap();
        assert_eq!(v.get("name").and_then(Json::as_str), Some(name));
        let request = v.get("request").unwrap();
        assert_eq!(request.get("text").and_then(Json::as_str), Some(gcode));
        assert_eq!(request.get("kind").and_then(Json::as_str), Some("gcode"));
    }

    // ---- the energy of a layer ----

    fn layer(power: f64, speed: f64) -> LayerParams {
        LayerParams {
            power_percent: power,
            speed_mm_min: speed,
            ..LayerParams::default()
        }
    }

    #[test]
    fn the_energy_of_a_line_is_the_power_over_the_speed_exactly() {
        // 10 W at 100 % and 600 mm/min (10 mm/s) is 1 J for each millimetre
        assert_eq!(line_energy_j_per_mm(10.0, &layer(100.0, 600.0)), Some(1.0));
        assert_eq!(line_energy_j_per_mm(10.0, &layer(50.0, 600.0)), Some(0.5));
    }

    #[test]
    fn no_energy_is_told_for_a_number_that_is_not_known_or_not_usable() {
        let p = layer(100.0, 600.0);
        for watts in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(line_energy_j_per_mm(watts, &p), None, "{watts}");
        }
        for speed in [0.0, -600.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                line_energy_j_per_mm(10.0, &layer(100.0, speed)),
                None,
                "{speed}"
            );
        }
        for power in [0.0, -5.0, f64::NAN] {
            assert_eq!(
                line_energy_j_per_mm(10.0, &layer(power, 600.0)),
                None,
                "{power}"
            );
        }
    }

    #[test]
    fn a_hatch_needs_lines_that_are_apart_and_a_cross_hatch_doubles_the_energy() {
        let mut p = layer(100.0, 600.0);
        p.hatch_spacing_mm = 0.5;
        assert_eq!(hatch_energy_j_per_mm2(10.0, &p), Some(2.0));
        p.hatch_cross = true;
        assert_eq!(hatch_energy_j_per_mm2(10.0, &p), Some(4.0));
        for spacing in [0.0, -0.1, f64::NAN, f64::INFINITY] {
            p.hatch_spacing_mm = spacing;
            assert_eq!(hatch_energy_j_per_mm2(10.0, &p), None, "{spacing}");
        }
    }

    #[test]
    fn a_skipped_layer_or_an_unknown_laser_has_no_words_of_energy() {
        let mut p = layer(100.0, 600.0);
        p.mode = LayerMode::Skip;
        assert_eq!(describe(10.0, &p), "");
        p.mode = LayerMode::Cut;
        assert_eq!(describe(0.0, &p), "");
        assert!(describe(10.0, &p).contains("J/mm a pass"));
        p.mode = LayerMode::Engrave;
        assert!(describe(10.0, &p).contains("J/mm2 a pass"));
    }

    // ---- the forbidden zones ----

    #[test]
    fn a_zone_needs_a_clean_name_and_numbers_inside_the_limits() {
        assert!(zone(0.0, 0.0, 1.0, 1.0).validate().is_ok());
        let mut z = zone(0.0, 0.0, 1.0, 1.0);
        z.name = String::new();
        assert!(z.validate().is_err());
        z.name = "a\nb".to_string();
        assert!(z.validate().is_err());
        z.name = "x".repeat(81);
        assert!(z.validate().is_err());
        z.name = "x".repeat(80);
        assert!(z.validate().is_ok());
        z.name = "é".repeat(80);
        assert!(
            z.validate().is_ok(),
            "the limit counts characters, not bytes"
        );
    }

    #[test]
    fn a_zone_is_valid_at_the_limits_of_its_place_and_size_and_not_past_them() {
        assert!(zone(MAX_ZONE_MM, -MAX_ZONE_MM, MAX_ZONE_MM, MAX_ZONE_MM)
            .validate()
            .is_ok());
        for bad in [
            zone(MAX_ZONE_MM + 0.001, 0.0, 1.0, 1.0),
            zone(0.0, -MAX_ZONE_MM - 0.001, 1.0, 1.0),
            zone(f64::NAN, 0.0, 1.0, 1.0),
            zone(0.0, f64::INFINITY, 1.0, 1.0),
            zone(0.0, 0.0, 0.0, 1.0),
            zone(0.0, 0.0, 1.0, -1.0),
            zone(0.0, 0.0, MAX_ZONE_MM + 0.001, 1.0),
            zone(0.0, 0.0, 1.0, f64::NAN),
        ] {
            assert!(bad.validate().is_err(), "{bad:?}");
        }
    }

    #[test]
    fn a_move_that_touches_the_edge_or_a_corner_of_a_zone_hits_it() {
        let z = zone(10.0, 10.0, 10.0, 10.0);
        assert!(
            z.hits((0.0, 15.0), (10.0, 15.0)),
            "the edge counts as inside"
        );
        assert!(z.hits((0.0, 0.0), (10.0, 10.0)), "a corner");
        assert!(z.hits((10.0, 0.0), (10.0, 30.0)), "along an edge");
        assert!(z.hits((15.0, 15.0), (15.0, 15.0)), "a point inside");
    }

    #[test]
    fn a_move_that_stays_outside_a_zone_does_not_hit_it() {
        let z = zone(10.0, 10.0, 10.0, 10.0);
        assert!(!z.hits((0.0, 15.0), (9.999, 15.0)));
        assert!(!z.hits((5.0, 0.0), (5.0, 30.0)));
        assert!(!z.hits((0.0, 0.0), (30.0, 0.0)));
        assert!(!z.hits((0.0, 9.0), (9.0, 30.0)));
        assert!(!z.hits((25.0, 25.0), (25.0, 25.0)));
    }

    #[test]
    fn a_move_with_a_coordinate_that_is_not_a_number_is_not_taken_as_safe_while_the_other_is_in_the_zone(
    ) {
        let z = zone(10.0, 10.0, 10.0, 10.0);
        assert!(z.hits((f64::NAN, 15.0), (0.0, 15.0)));
        assert!(z.hits((0.0, 15.0), (f64::NAN, f64::NAN)));
    }

    #[test]
    fn a_closed_path_counts_its_closing_move_and_an_open_one_does_not() {
        let z = zone(10.0, 10.0, 10.0, 10.0);
        let triangle = [(0.0, 0.0), (30.0, 0.0), (30.0, 30.0)];
        assert!(!z.hits_path(&triangle, false));
        assert!(z.hits_path(&triangle, true));
    }

    #[test]
    fn a_path_of_no_point_or_one_point_is_read_without_a_panic() {
        let z = zone(10.0, 10.0, 10.0, 10.0);
        assert!(!z.hits_path(&[], false));
        assert!(!z.hits_path(&[], true));
        assert!(z.hits_path(&[(15.0, 15.0)], false));
        assert!(!z.hits_path(&[(0.0, 0.0)], true));
    }

    #[test]
    fn the_problems_of_the_paths_count_the_ones_after_the_first() {
        let z = zone(10.0, 10.0, 10.0, 10.0);
        let a = [(0.0, 15.0), (30.0, 15.0)];
        let b = [(15.0, 0.0), (15.0, 30.0)];
        let c = [(0.0, 0.0), (1.0, 1.0)];
        let items = [
            ("a", a.as_slice(), false),
            ("b", b.as_slice(), false),
            ("c", c.as_slice(), false),
        ];
        let out = path_problems(items.iter().copied(), std::slice::from_ref(&z));
        assert_eq!(out.len(), 1);
        assert!(
            out[0].contains("the layer a ") && out[0].contains("(and 1 more path(s))"),
            "{}",
            out[0]
        );
        assert!(path_problems(items.iter().copied(), &[]).is_empty());
    }

    #[test]
    fn the_kind_of_a_zone_is_read_from_a_word_and_a_wrong_word_is_nothing() {
        assert_eq!(ZoneKind::from_word(" No-Burn "), Some(ZoneKind::NoBurn));
        assert_eq!(ZoneKind::from_word("NO GO"), Some(ZoneKind::NoGo));
        assert_eq!(ZoneKind::from_word("no_go"), Some(ZoneKind::NoGo));
        for bad in ["", "nogo", "no__go", "burn", "no_go\n x"] {
            assert_eq!(ZoneKind::from_word(bad), None, "{bad:?}");
        }
        assert_eq!(
            ZoneKind::from_word(ZoneKind::NoBurn.word()),
            Some(ZoneKind::NoBurn)
        );
    }

    #[test]
    fn the_interlocks_stop_the_machine_unless_every_required_one_is_in_its_safe_state() {
        assert!(Interlocks::default().problems().is_empty());
        let pressed = Interlocks {
            estop: Estop::Pressed,
            ..Interlocks::default()
        };
        assert_eq!(pressed.problems().len(), 1);
        let unknown = Interlocks {
            require_enclosure: true,
            require_air_assist: true,
            ..Interlocks::default()
        };
        assert_eq!(
            unknown.problems().len(),
            2,
            "a state that is not known is not safe"
        );
        let safe = Interlocks {
            enclosure: Enclosure::Closed,
            air_assist: AirAssist::On,
            ..unknown
        };
        assert!(safe.problems().is_empty());
        let open = Interlocks {
            enclosure: Enclosure::Open,
            ..safe
        };
        assert!(open.problems()[0].contains("open"));
    }
}
