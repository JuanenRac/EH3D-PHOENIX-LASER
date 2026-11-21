// EH3D-PHOENIX-LASER - the live adjustment of the power and of the speed, held by the limits.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! While a job runs a person may want a little less or a little more power or speed. Here that is only a calculation: from the value the program has and the percent that is asked, it gives
//! the value that may be used, and it never gives one above a limit. Four limits hold, and the lowest wins: the range of the adjustment itself (`[live]` of the settings file, which does not
//! go above 100 % unless the file says so), the limit of power or of speed of the machine, the limit of power sent to the server (`max_power_percent` of `[send]`) and, for the low power that
//! shows the frame or moves the head, the limit of the pilot (`max_pilot_percent` of `[frame]`). A value that was held is never held in silence: the answer says by which limit, in the
//! eight languages. Nothing here talks to a machine or to the server; a program that sends the result owns the sending and its guards.

use crate::lang::{fill, Lang, Msg};
use crate::vconfig::Settings;

/// The `[live]` section of the settings file: the range of the adjustment, in percent of the value the program has.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveSettings {
    pub power_min_percent: f64,
    pub power_max_percent: f64,
    pub speed_min_percent: f64,
    pub speed_max_percent: f64,
}

impl Default for LiveSettings {
    /// The conservative range: the adjustment can lower a value to a tenth and never raise it.
    fn default() -> Self {
        LiveSettings {
            power_min_percent: 10.0,
            power_max_percent: 100.0,
            speed_min_percent: 10.0,
            speed_max_percent: 100.0,
        }
    }
}

impl LiveSettings {
    pub fn validate(&self) -> Result<(), String> {
        for (name, lo, hi) in [
            ("power", self.power_min_percent, self.power_max_percent),
            ("speed", self.speed_min_percent, self.speed_max_percent),
        ] {
            for (kind, v) in [("min", lo), ("max", hi)] {
                if !(v.is_finite() && (1.0..=200.0).contains(&v)) {
                    return Err(format!(
                        "live {name}_{kind}_percent is {v}: it must be from 1 to 200"
                    ));
                }
            }
            if lo > hi {
                return Err(format!(
                    "live {name}_min_percent is {lo}, more than {name}_max_percent, which is {hi}"
                ));
            }
        }
        Ok(())
    }
}

/// What the adjustment is for: the power of the program, or the low power of the pilot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Program,
    Pilot,
}

/// The limit that held a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    /// The range of the adjustment (`[live]`).
    Range,
    /// The limit of power or of speed of the machine (`[machine]`).
    Machine,
    /// The limit of power sent to the server (`[send]`).
    Server,
    /// The limit of the pilot power (`[frame]`).
    Pilot,
}

impl Held {
    fn msg(self) -> Msg {
        match self {
            Held::Range => Msg::LiveByRange,
            Held::Machine => Msg::LiveByMachine,
            Held::Server => Msg::LiveByServer,
            Held::Pilot => Msg::LiveByPilot,
        }
    }
}

/// What is adjusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    Power,
    Speed,
}

/// The result of an adjustment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Adjusted {
    pub quantity: Quantity,
    /// The value the program has (a percent of full power, or mm/min).
    pub programmed: f64,
    /// The percent that was asked.
    pub asked_percent: f64,
    /// The percent that is applied to the programmed value.
    pub applied_percent: f64,
    /// The value to use: the programmed value times the applied percent.
    pub value: f64,
    /// The limit that held the value, when one did.
    pub held: Option<Held>,
}

impl Adjusted {
    /// What a person is told when the value was held, in a language; `None` when it was used as asked.
    pub fn warning(&self, lang: Lang) -> Option<String> {
        let by = self.held?;
        let what = match self.quantity {
            Quantity::Power => Msg::LivePower,
            Quantity::Speed => Msg::LiveSpeed,
        };
        Some(fill(
            Msg::LiveHeld.tr(lang),
            &[
                what.tr(lang),
                &format!("{:.1}", self.asked_percent),
                &format!("{:.1}", self.applied_percent),
                by.msg().tr(lang),
            ],
        ))
    }
}

fn percent_ok(name: &str, v: f64) -> Result<(), String> {
    if v.is_finite() && v > 0.0 && v <= 1_000.0 {
        Ok(())
    } else {
        Err(format!(
            "the {name} adjustment is {v} %: it must be more than 0 and at most 1000"
        ))
    }
}

/// The power to use when the person asks for `asked_percent` of the power the program has (`programmed_percent`, a percent of full power). The range of `[live]` is applied first, then the
/// lowest of the limits that hold for the target: the machine, the server and, for the pilot, the pilot.
pub fn power(
    s: &Settings,
    target: Target,
    programmed_percent: f64,
    asked_percent: f64,
) -> Result<Adjusted, String> {
    s.live.validate()?;
    percent_ok("power", asked_percent)?;
    if !(programmed_percent.is_finite() && (0.0..=100.0).contains(&programmed_percent)) {
        return Err(format!(
            "the power of the program is {programmed_percent} %: it must be from 0 to 100"
        ));
    }
    let l = &s.live;
    let mut applied = asked_percent.clamp(l.power_min_percent, l.power_max_percent);
    let mut held = (applied != asked_percent).then_some(Held::Range);
    let limits = if target == Target::Pilot {
        [
            (s.frame.max_pilot_percent, Held::Pilot),
            (s.machine.max_power_percent, Held::Machine),
        ]
    } else {
        [
            (s.machine.max_power_percent, Held::Machine),
            (s.send.max_power_percent, Held::Server),
        ]
    };
    let mut value = programmed_percent * applied / 100.0;
    for (limit, by) in limits {
        if value > limit {
            value = limit;
            applied = if programmed_percent > 0.0 {
                limit / programmed_percent * 100.0
            } else {
                applied
            };
            held = Some(by);
        }
    }
    Ok(Adjusted {
        quantity: Quantity::Power,
        programmed: programmed_percent,
        asked_percent,
        applied_percent: applied,
        value,
        held,
    })
}

/// The speed (mm/min) to use when the person asks for `asked_percent` of the speed the program has: the range of `[live]`, then the limit of speed of the machine.
pub fn speed(s: &Settings, programmed_mm_min: f64, asked_percent: f64) -> Result<Adjusted, String> {
    s.live.validate()?;
    percent_ok("speed", asked_percent)?;
    if !(programmed_mm_min.is_finite() && programmed_mm_min > 0.0) {
        return Err(format!(
            "the speed of the program is {programmed_mm_min} mm/min: it must be more than 0"
        ));
    }
    let l = &s.live;
    let mut applied = asked_percent.clamp(l.speed_min_percent, l.speed_max_percent);
    let mut held = (applied != asked_percent).then_some(Held::Range);
    let mut value = programmed_mm_min * applied / 100.0;
    if value > s.machine.max_speed_mm_min {
        value = s.machine.max_speed_mm_min;
        applied = value / programmed_mm_min * 100.0;
        held = Some(Held::Machine);
    }
    Ok(Adjusted {
        quantity: Quantity::Speed,
        programmed: programmed_mm_min,
        asked_percent,
        applied_percent: applied,
        value,
        held,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_range_lowers_a_value_and_never_raises_it() {
        let s = Settings::default();
        let a = power(&s, Target::Program, 30.0, 50.0).unwrap();
        assert_eq!((a.value, a.held), (15.0, None));
        let up = power(&s, Target::Program, 30.0, 150.0).unwrap();
        assert_eq!((up.value, up.held), (30.0, Some(Held::Range)));
        let low = speed(&s, 1000.0, 1.0).unwrap();
        assert_eq!((low.value, low.held), (100.0, Some(Held::Range)));
    }

    #[test]
    fn the_range_is_checked() {
        assert!(LiveSettings::default().validate().is_ok());
        for (lo, hi) in [(0.5, 100.0), (10.0, 201.0), (50.0, 40.0), (f64::NAN, 100.0)] {
            let l = LiveSettings {
                power_min_percent: lo,
                power_max_percent: hi,
                ..LiveSettings::default()
            };
            assert!(l.validate().is_err(), "{lo} {hi}");
        }
    }
}
