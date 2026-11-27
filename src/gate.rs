// EH3D-PHOENIX-LASER - no program is written that the program's own check would fail.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Every G-code program the commands and the window write goes through the check of `verify` first, with the limits of the work and the forbidden zones of the settings: a program that leaves
//! the bed, burns in a zone or breaks a limit is not written. The interlocks are not part of it: they say how the machine is at this moment, and they are looked at when a program is checked,
//! reported or sent.

use crate::guard::Zone;
use crate::verify::{self, Guard, Limits};

/// What the work is held to: the full power of the controller, the limits of power and speed, the bed and how far past it the head may go with the laser off.
pub fn limits(
    s_max: u32,
    max_power_percent: f64,
    max_speed_mm_min: f64,
    bed_mm: Option<(f64, f64)>,
    overscan_mm: f64,
) -> Limits {
    Limits {
        s_max,
        max_power_percent,
        max_speed_mm_min,
        bed_mm,
        overscan_mm,
        ..Limits::default()
    }
}

/// The check before `program` is written. The error says every reason.
pub fn before_write(program: &str, limits: &Limits, zones: &[Zone]) -> Result<(), String> {
    let guard = Guard {
        zones,
        interlocks: None,
    };
    let verdict = verify::verify_with(program.as_bytes(), limits, &guard)?;
    if verdict.passes() {
        Ok(())
    } else {
        Err(format!(
            "the program was not written, it does not pass the check: {}",
            verdict.problems.join("; ")
        ))
    }
}
