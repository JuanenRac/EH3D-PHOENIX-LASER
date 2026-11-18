// EH3D-PHOENIX-LASER - the limit of power and the rules that keep the laser off.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! A laser can blind and set fire. Two rules keep the program on the safe side. The first is the limit of power (`max_power_percent` in `[machine]`, or `--max-power`): a program whose power is
//! more than the limit is refused, never quietly lowered, so what is written is what was asked for. The second is that nothing turns the beam on to position the work: the frame of the work
//! is traced with the laser off, and a low power for the frame needs the words `--confirm-laser-on` and stays under a small limit of its own.

/// The limit of power when nothing says another: no limit below full power.
pub const DEFAULT_MAX_POWER_PERCENT: f64 = 100.0;

/// The most that a power to show the frame may be, whatever the limit of the machine is: a percent of full power.
pub const MAX_PILOT_PERCENT: f64 = 5.0;

/// A limit of power must be from 0.1 to 100 percent.
pub fn validate_max(max_percent: f64) -> Result<(), String> {
    if max_percent.is_finite() && (0.1..=100.0).contains(&max_percent) {
        Ok(())
    } else {
        Err(format!(
            "max_power_percent is {max_percent}: it must be from 0.1 to 100"
        ))
    }
}

/// Refuses a power over the limit; `what` names where the power comes from (`--power`, a layer...).
pub fn check_power(what: &str, percent: f64, max_percent: f64) -> Result<(), String> {
    validate_max(max_percent)?;
    if percent.is_finite() && percent <= max_percent {
        Ok(())
    } else {
        Err(format!(
            "{what} is {percent} %: it is more than the limit of power, max_power_percent, which is {max_percent} %"
        ))
    }
}

/// The power to show the frame with, as a percent of full power: 0 (the laser stays off) unless the person said `confirmed`, and then at most [`MAX_PILOT_PERCENT`] and at most the limit of
/// power of the machine.
pub fn pilot_percent(requested: f64, confirmed: bool, max_percent: f64) -> Result<f64, String> {
    if !requested.is_finite() || requested < 0.0 {
        return Err(format!(
            "the power of the frame is {requested}: it must be 0 or more"
        ));
    }
    if requested == 0.0 {
        return Ok(0.0);
    }
    if !confirmed {
        return Err(
            "a power for the frame turns the laser on: say it with --confirm-laser-on, and keep the eyes and the material safe"
                .to_string(),
        );
    }
    if requested > MAX_PILOT_PERCENT {
        return Err(format!(
            "the power of the frame is {requested} %: it must be {MAX_PILOT_PERCENT} % or less"
        ));
    }
    check_power("the power of the frame", requested, max_percent)?;
    Ok(requested)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_limit_must_be_from_a_tenth_to_one_hundred() {
        assert!(validate_max(0.1).is_ok());
        assert!(validate_max(100.0).is_ok());
        for bad in [0.0, 0.09, 100.1, -5.0, f64::NAN, f64::INFINITY] {
            assert!(validate_max(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_power_over_the_limit_is_refused_and_one_at_the_limit_is_not() {
        assert!(check_power("--power", 50.0, 50.0).is_ok());
        assert!(check_power("--power", 0.0, 50.0).is_ok());
        let e = check_power("--power", 50.1, 50.0).unwrap_err();
        assert!(
            e.contains("--power") && e.contains("max_power_percent"),
            "{e}"
        );
        assert!(check_power("--power", f64::NAN, 50.0).is_err());
        assert!(check_power("--power", 10.0, 0.0).is_err());
    }

    #[test]
    fn the_frame_is_shown_with_the_laser_off_unless_it_is_confirmed() {
        assert_eq!(pilot_percent(0.0, false, 100.0).unwrap(), 0.0);
        assert_eq!(pilot_percent(0.0, true, 100.0).unwrap(), 0.0);
        let e = pilot_percent(1.0, false, 100.0).unwrap_err();
        assert!(e.contains("--confirm-laser-on"), "{e}");
        assert_eq!(pilot_percent(1.0, true, 100.0).unwrap(), 1.0);
        assert_eq!(pilot_percent(5.0, true, 100.0).unwrap(), 5.0);
    }

    #[test]
    fn the_power_of_the_frame_has_a_small_limit_and_obeys_the_limit_of_the_machine() {
        assert!(pilot_percent(5.1, true, 100.0).is_err());
        assert!(pilot_percent(-1.0, true, 100.0).is_err());
        assert!(pilot_percent(f64::NAN, true, 100.0).is_err());
        // the machine allows less than the frame does
        assert!(pilot_percent(3.0, true, 2.0).is_err());
        assert_eq!(pilot_percent(2.0, true, 2.0).unwrap(), 2.0);
    }
}
