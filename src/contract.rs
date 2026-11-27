// EH3D-PHOENIX-LASER - the contract of the settings of the program.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! Every setting of the program, in one list that another program can read: the section and the key of the settings file (or of a library of materials), the kind of value it takes, its
//! default and whether it is a safety setting (a limit, a zone, an interlock, a token). The list is what `phoenix-laser settings` prints, as text or as JSON, so that a server, a studio or
//! a test can show or check every switch and number of this program without reading its code. Tests keep the list equal to the keys the settings reader takes and to the defaults of
//! `docs/vector-settings.md`, so a key added to the reader and not to the list, or the other way round, fails.

/// The kind of value a setting takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A number.
    Number,
    /// A whole number.
    Whole,
    /// `on` or `off`.
    Switch,
    /// One of a few words.
    Word,
    /// Text.
    Text,
}

impl Kind {
    /// The word that names the kind in the list.
    pub fn word(self) -> &'static str {
        match self {
            Kind::Number => "number",
            Kind::Whole => "whole",
            Kind::Switch => "switch",
            Kind::Word => "word",
            Kind::Text => "text",
        }
    }
}

/// One setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The section of the file (`machine`, `zone`...); `layer` stands for `[defaults]`, `[layer NAME]` and `[sublayer LAYER/LABEL]`, `profile` for `[profile NAME]`, `zone` for `[zone NAME]` and `material` for the
    /// sections of a library of materials.
    pub section: &'static str,
    pub key: &'static str,
    pub kind: Kind,
    /// The value the program has when the file does not say, as `docs/vector-settings.md` writes it (`none` when there is no value, `not given` for what a profile leaves to the machine).
    pub default: &'static str,
    /// A setting that holds the machine safe or keeps a secret: a limit, a zone, an interlock, the token of the server.
    pub safety: bool,
}

/// Every setting, in the order of the sections of the documentation.
pub static ENTRIES: &[Entry] = &[
    Entry {
        section: "layer",
        key: "mode",
        kind: Kind::Word,
        default: "cut",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "power",
        kind: Kind::Number,
        default: "50",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "speed",
        kind: Kind::Number,
        default: "600",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "passes",
        kind: Kind::Whole,
        default: "1",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "focus_step",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "kerf",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "kerf_side",
        kind: Kind::Word,
        default: "none",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "tabs",
        kind: Kind::Whole,
        default: "0",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "tab_width",
        kind: Kind::Number,
        default: "1",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "perf_cut",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "perf_gap",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "hatch_angle",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "hatch_spacing",
        kind: Kind::Number,
        default: "0.1",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "hatch_cross",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "layer",
        key: "order",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_file_bytes",
        kind: Kind::Whole,
        default: "33554432",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_depth",
        kind: Kind::Whole,
        default: "64",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_nodes",
        kind: Kind::Whole,
        default: "200000",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_shapes",
        kind: Kind::Whole,
        default: "100000",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_points",
        kind: Kind::Whole,
        default: "2000000",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_curve_segments",
        kind: Kind::Whole,
        default: "2000",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_coordinate_mm",
        kind: Kind::Number,
        default: "100000",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_bitmaps",
        kind: Kind::Whole,
        default: "16",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_bitmap_bytes",
        kind: Kind::Whole,
        default: "16777216",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_bitmap_pixels",
        kind: Kind::Whole,
        default: "20000000",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_csv_bytes",
        kind: Kind::Whole,
        default: "1048576",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_csv_rows",
        kind: Kind::Whole,
        default: "10000",
        safety: false,
    },
    Entry {
        section: "limits",
        key: "max_csv_columns",
        kind: Kind::Whole,
        default: "64",
        safety: false,
    },
    Entry {
        section: "read",
        key: "dpi",
        kind: Kind::Number,
        default: "96",
        safety: false,
    },
    Entry {
        section: "read",
        key: "tolerance_mm",
        kind: Kind::Number,
        default: "0.01",
        safety: false,
    },
    Entry {
        section: "read",
        key: "dxf_unit_mm",
        kind: Kind::Number,
        default: "1",
        safety: false,
    },
    Entry {
        section: "read",
        key: "lbrn2_flip_y",
        kind: Kind::Switch,
        default: "on",
        safety: false,
    },
    Entry {
        section: "read",
        key: "group_by",
        kind: Kind::Word,
        default: "auto",
        safety: false,
    },
    Entry {
        section: "read",
        key: "align",
        kind: Kind::Word,
        default: "bbox",
        safety: false,
    },
    Entry {
        section: "order",
        key: "join_tolerance_mm",
        kind: Kind::Number,
        default: "0.01",
        safety: false,
    },
    Entry {
        section: "order",
        key: "join_max_paths",
        kind: Kind::Whole,
        default: "5000",
        safety: false,
    },
    Entry {
        section: "order",
        key: "order_max_items",
        kind: Kind::Whole,
        default: "20000",
        safety: false,
    },
    Entry {
        section: "order",
        key: "containment_max_items",
        kind: Kind::Whole,
        default: "1500",
        safety: false,
    },
    Entry {
        section: "order",
        key: "two_opt",
        kind: Kind::Switch,
        default: "on",
        safety: false,
    },
    Entry {
        section: "order",
        key: "merge_doubled",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "order",
        key: "two_opt_max_items",
        kind: Kind::Whole,
        default: "2000",
        safety: false,
    },
    Entry {
        section: "order",
        key: "two_opt_budget",
        kind: Kind::Whole,
        default: "20000000",
        safety: false,
    },
    Entry {
        section: "offset",
        key: "max_points",
        kind: Kind::Whole,
        default: "5000",
        safety: false,
    },
    Entry {
        section: "hatch",
        key: "max_lines",
        kind: Kind::Whole,
        default: "100000",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "s_max",
        kind: Kind::Whole,
        default: "1000",
        safety: true,
    },
    Entry {
        section: "machine",
        key: "overscan_mm",
        kind: Kind::Number,
        default: "2",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "rapid_mm_min",
        kind: Kind::Number,
        default: "6000",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "laser_watts",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "origin_x",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "origin_y",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "laser_mode",
        kind: Kind::Word,
        default: "m4",
        safety: true,
    },
    Entry {
        section: "machine",
        key: "focus_z_mm",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "bed_x_mm",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "bed_y_mm",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "focus",
        key: "z_start_mm",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "focus",
        key: "z_step_mm",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "focus",
        key: "steps",
        kind: Kind::Whole,
        default: "9",
        safety: false,
    },
    Entry {
        section: "focus",
        key: "length_mm",
        kind: Kind::Number,
        default: "10",
        safety: false,
    },
    Entry {
        section: "focus",
        key: "spacing_mm",
        kind: Kind::Number,
        default: "2",
        safety: false,
    },
    Entry {
        section: "focus",
        key: "feed_mm_min",
        kind: Kind::Number,
        default: "1000",
        safety: false,
    },
    Entry {
        section: "focus",
        key: "power_percent",
        kind: Kind::Number,
        default: "30",
        safety: false,
    },
    Entry {
        section: "tone",
        key: "gamma",
        kind: Kind::Number,
        default: "1",
        safety: false,
    },
    Entry {
        section: "tone",
        key: "contrast",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "tone",
        key: "brightness",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "tone",
        key: "invert",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "tone",
        key: "auto_level",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "send",
        key: "max_power_percent",
        kind: Kind::Number,
        default: "40",
        safety: true,
    },
    Entry {
        section: "interlocks",
        key: "estop",
        kind: Kind::Word,
        default: "released",
        safety: true,
    },
    Entry {
        section: "interlocks",
        key: "enclosure",
        kind: Kind::Word,
        default: "unknown",
        safety: true,
    },
    Entry {
        section: "interlocks",
        key: "air_assist",
        kind: Kind::Word,
        default: "unknown",
        safety: true,
    },
    Entry {
        section: "interlocks",
        key: "require_enclosure",
        kind: Kind::Switch,
        default: "off",
        safety: true,
    },
    Entry {
        section: "interlocks",
        key: "require_air_assist",
        kind: Kind::Switch,
        default: "off",
        safety: true,
    },
    Entry {
        section: "verify",
        key: "overscan_mm",
        kind: Kind::Number,
        default: "0",
        safety: true,
    },
    Entry {
        section: "verify",
        key: "arc_tolerance_mm",
        kind: Kind::Number,
        default: "0.01",
        safety: true,
    },
    Entry {
        section: "verify",
        key: "s_max_mismatch_warns",
        kind: Kind::Switch,
        default: "off",
        safety: true,
    },
    Entry {
        section: "calibrate",
        key: "steps",
        kind: Kind::Whole,
        default: "5",
        safety: false,
    },
    Entry {
        section: "calibrate",
        key: "spread_percent",
        kind: Kind::Number,
        default: "40",
        safety: false,
    },
    Entry {
        section: "calibrate",
        key: "labels",
        kind: Kind::Switch,
        default: "on",
        safety: false,
    },
    Entry {
        section: "calibrate",
        key: "label_mm",
        kind: Kind::Number,
        default: "3",
        safety: false,
    },
    Entry {
        section: "calibrate",
        key: "min_power_percent",
        kind: Kind::Number,
        default: "1",
        safety: false,
    },
    Entry {
        section: "calibrate",
        key: "min_feed_mm_min",
        kind: Kind::Number,
        default: "10",
        safety: false,
    },
    Entry {
        section: "text",
        key: "height_mm",
        kind: Kind::Number,
        default: "10",
        safety: false,
    },
    Entry {
        section: "text",
        key: "max_height_mm",
        kind: Kind::Number,
        default: "100",
        safety: false,
    },
    Entry {
        section: "text",
        key: "spacing_percent",
        kind: Kind::Number,
        default: "25",
        safety: false,
    },
    Entry {
        section: "text",
        key: "line_spacing_percent",
        kind: Kind::Number,
        default: "50",
        safety: false,
    },
    Entry {
        section: "text",
        key: "power_percent",
        kind: Kind::Number,
        default: "20",
        safety: false,
    },
    Entry {
        section: "text",
        key: "speed_mm_min",
        kind: Kind::Number,
        default: "1000",
        safety: false,
    },
    Entry {
        section: "text",
        key: "passes",
        kind: Kind::Whole,
        default: "1",
        safety: false,
    },
    Entry {
        section: "text",
        key: "max_chars",
        kind: Kind::Whole,
        default: "200",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "module_mm",
        kind: Kind::Number,
        default: "1",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "line_mm",
        kind: Kind::Number,
        default: "0.1",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "power_percent",
        kind: Kind::Number,
        default: "20",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "speed_mm_min",
        kind: Kind::Number,
        default: "1000",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "passes",
        kind: Kind::Whole,
        default: "1",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "max_bytes",
        kind: Kind::Whole,
        default: "40",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "max_version",
        kind: Kind::Whole,
        default: "4",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "ecc",
        kind: Kind::Word,
        default: "M",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "max_size_mm",
        kind: Kind::Number,
        default: "100",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "max_lines",
        kind: Kind::Whole,
        default: "100000",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "invert",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "overscan_mm",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "mirror",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "max_minutes",
        kind: Kind::Number,
        default: "240",
        safety: false,
    },
    Entry {
        section: "qr",
        key: "border_modules",
        kind: Kind::Whole,
        default: "0",
        safety: false,
    },
    Entry {
        section: "marks",
        key: "arm_mm",
        kind: Kind::Number,
        default: "5",
        safety: false,
    },
    Entry {
        section: "marks",
        key: "margin_mm",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "marks",
        key: "power_percent",
        kind: Kind::Number,
        default: "10",
        safety: false,
    },
    Entry {
        section: "marks",
        key: "speed_mm_min",
        kind: Kind::Number,
        default: "1000",
        safety: false,
    },
    Entry {
        section: "marks",
        key: "passes",
        kind: Kind::Whole,
        default: "1",
        safety: false,
    },
    Entry {
        section: "marks",
        key: "centre_cross",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "module_mm",
        kind: Kind::Number,
        default: "0.4",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "height_mm",
        kind: Kind::Number,
        default: "10",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "line_mm",
        kind: Kind::Number,
        default: "0.1",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "power_percent",
        kind: Kind::Number,
        default: "20",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "speed_mm_min",
        kind: Kind::Number,
        default: "1000",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "passes",
        kind: Kind::Whole,
        default: "1",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "max_chars",
        kind: Kind::Whole,
        default: "20",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "max_width_mm",
        kind: Kind::Number,
        default: "150",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "max_lines",
        kind: Kind::Whole,
        default: "100000",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "max_minutes",
        kind: Kind::Number,
        default: "240",
        safety: false,
    },
    Entry {
        section: "barcode",
        key: "mirror",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "server",
        key: "server_address",
        kind: Kind::Text,
        default: "empty",
        safety: false,
    },
    Entry {
        section: "server",
        key: "server_token",
        kind: Kind::Text,
        default: "empty",
        safety: true,
    },
    Entry {
        section: "server",
        key: "server_machine",
        kind: Kind::Text,
        default: "empty",
        safety: false,
    },
    Entry {
        section: "server",
        key: "server_timeout_s",
        kind: Kind::Whole,
        default: "15",
        safety: false,
    },
    Entry {
        section: "server",
        key: "server_retries",
        kind: Kind::Whole,
        default: "2",
        safety: false,
    },
    Entry {
        section: "server",
        key: "server_mdns",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "text",
        key: "max_minutes",
        kind: Kind::Number,
        default: "240",
        safety: false,
    },
    Entry {
        section: "text",
        key: "mirror",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "text",
        key: "align",
        kind: Kind::Word,
        default: "left",
        safety: false,
    },
    Entry {
        section: "trace",
        key: "threshold",
        kind: Kind::Whole,
        default: "128",
        safety: false,
    },
    Entry {
        section: "trace",
        key: "invert",
        kind: Kind::Switch,
        default: "off",
        safety: false,
    },
    Entry {
        section: "trace",
        key: "simplify_mm",
        kind: Kind::Number,
        default: "0.02",
        safety: false,
    },
    Entry {
        section: "trace",
        key: "min_area_mm2",
        kind: Kind::Number,
        default: "0.05",
        safety: false,
    },
    Entry {
        section: "trace",
        key: "max_pixels",
        kind: Kind::Whole,
        default: "4000000",
        safety: false,
    },
    Entry {
        section: "trace",
        key: "max_contours",
        kind: Kind::Whole,
        default: "20000",
        safety: false,
    },
    Entry {
        section: "trace",
        key: "max_segments",
        kind: Kind::Whole,
        default: "2000000",
        safety: false,
    },
    Entry {
        section: "frame",
        key: "feed_mm_min",
        kind: Kind::Number,
        default: "3000",
        safety: false,
    },
    Entry {
        section: "frame",
        key: "laps",
        kind: Kind::Whole,
        default: "1",
        safety: false,
    },
    Entry {
        section: "frame",
        key: "margin_mm",
        kind: Kind::Number,
        default: "0",
        safety: false,
    },
    Entry {
        section: "frame",
        key: "pilot_percent",
        kind: Kind::Number,
        default: "0",
        safety: true,
    },
    Entry {
        section: "frame",
        key: "max_pilot_percent",
        kind: Kind::Number,
        default: "5",
        safety: true,
    },
    Entry {
        section: "job",
        key: "anchor",
        kind: Kind::Word,
        default: "bottom_left",
        safety: false,
    },
    Entry {
        section: "job",
        key: "accel_mm_s2",
        kind: Kind::Number,
        default: "1000",
        safety: false,
    },
    Entry {
        section: "live",
        key: "power_min_percent",
        kind: Kind::Number,
        default: "10",
        safety: false,
    },
    Entry {
        section: "live",
        key: "power_max_percent",
        kind: Kind::Number,
        default: "100",
        safety: true,
    },
    Entry {
        section: "live",
        key: "speed_min_percent",
        kind: Kind::Number,
        default: "10",
        safety: false,
    },
    Entry {
        section: "live",
        key: "speed_max_percent",
        kind: Kind::Number,
        default: "100",
        safety: true,
    },
    Entry {
        section: "machine",
        key: "max_power_percent",
        kind: Kind::Number,
        default: "100",
        safety: true,
    },
    Entry {
        section: "machine",
        key: "max_speed_mm_min",
        kind: Kind::Number,
        default: "30000",
        safety: true,
    },
    Entry {
        section: "machine",
        key: "profile",
        kind: Kind::Text,
        default: "none",
        safety: false,
    },
    Entry {
        section: "machine",
        key: "max_passes",
        kind: Kind::Whole,
        default: "50",
        safety: false,
    },
    Entry {
        section: "profile",
        key: "travel_x_mm",
        kind: Kind::Number,
        default: "not given",
        safety: false,
    },
    Entry {
        section: "profile",
        key: "travel_y_mm",
        kind: Kind::Number,
        default: "not given",
        safety: false,
    },
    Entry {
        section: "profile",
        key: "max_power_percent",
        kind: Kind::Number,
        default: "not given",
        safety: true,
    },
    Entry {
        section: "profile",
        key: "max_speed_mm_min",
        kind: Kind::Number,
        default: "not given",
        safety: true,
    },
    Entry {
        section: "profile",
        key: "s_max",
        kind: Kind::Whole,
        default: "not given",
        safety: true,
    },
    Entry {
        section: "profile",
        key: "laser_watts",
        kind: Kind::Number,
        default: "not given",
        safety: false,
    },
    Entry {
        section: "profile",
        key: "laser_mode",
        kind: Kind::Word,
        default: "not given",
        safety: true,
    },
    Entry {
        section: "zone",
        key: "x_mm",
        kind: Kind::Number,
        default: "0",
        safety: true,
    },
    Entry {
        section: "zone",
        key: "y_mm",
        kind: Kind::Number,
        default: "0",
        safety: true,
    },
    Entry {
        section: "zone",
        key: "width_mm",
        kind: Kind::Number,
        default: "none",
        safety: true,
    },
    Entry {
        section: "zone",
        key: "height_mm",
        kind: Kind::Number,
        default: "none",
        safety: true,
    },
    Entry {
        section: "zone",
        key: "kind",
        kind: Kind::Word,
        default: "no_go",
        safety: true,
    },
    Entry {
        section: "material",
        key: "note",
        kind: Kind::Text,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "verified",
        kind: Kind::Switch,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "power",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "speed",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "passes",
        kind: Kind::Whole,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "focus_step",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "kerf",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "hatch_spacing",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "hatch_angle",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "hatch_cross",
        kind: Kind::Switch,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "dot",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "overscan",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "dither",
        kind: Kind::Word,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "threshold",
        kind: Kind::Whole,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "gamma",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "contrast",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "brightness",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "invert",
        kind: Kind::Switch,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "auto_level",
        kind: Kind::Switch,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "max_power",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
    Entry {
        section: "material",
        key: "max_speed",
        kind: Kind::Number,
        default: "none",
        safety: false,
    },
];

/// The entry of a key, when there is one.
pub fn find(section: &str, key: &str) -> Option<&'static Entry> {
    ENTRIES
        .iter()
        .find(|e| e.section == section && e.key == key)
}

/// The list as lines of text, one setting to a line: `section.key  kind  default` and `  (safety)` for a safety setting.
pub fn text() -> String {
    let mut out = String::new();
    for e in ENTRIES {
        out.push_str(&format!(
            "{}.{}  {}  {}{}\n",
            e.section,
            e.key,
            e.kind.word(),
            e.default,
            if e.safety { "  (safety)" } else { "" }
        ));
    }
    out.push_str(&format!(
        "{} settings, {} of them safety settings\n",
        ENTRIES.len(),
        ENTRIES.iter().filter(|e| e.safety).count()
    ));
    out
}

/// A text for a JSON string.
fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The list as JSON: the program, its version and one object for each setting.
pub fn json() -> String {
    let rows: Vec<String> = ENTRIES
        .iter()
        .map(|e| {
            format!(
                "    {{\"section\": {}, \"key\": {}, \"kind\": {}, \"default\": {}, \"safety\": {}}}",
                json_string(e.section),
                json_string(e.key),
                json_string(e.kind.word()),
                json_string(e.default),
                e.safety
            )
        })
        .collect();
    format!(
        "{{\n  \"program\": \"phoenix-laser\",\n  \"version\": {},\n  \"settings\": [\n{}\n  ]\n}}\n",
        json_string(env!("CARGO_PKG_VERSION")),
        rows.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials;
    use crate::vconfig;
    use std::collections::BTreeSet;

    const SOURCE: &str = include_str!("vconfig.rs");

    /// The text of the function `name` of the reader, up to the next function.
    fn function(name: &str) -> &'static str {
        let at = SOURCE
            .find(&format!("\nfn {name}("))
            .unwrap_or_else(|| panic!("no fn {name}"));
        let rest = &SOURCE[at + 1..];
        let end = rest[3..]
            .find("\nfn ")
            .into_iter()
            .chain(rest[3..].find("\npub fn "))
            .min()
            .map_or(rest.len(), |e| e + 3);
        &rest[..end]
    }

    /// The keys of the arms `"key" => ...` of a function.
    fn single(name: &str) -> BTreeSet<String> {
        function(name)
            .lines()
            .filter_map(|l| {
                let (key, after) = l.trim().strip_prefix('"')?.split_once('"')?;
                after
                    .trim_start()
                    .starts_with("=>")
                    .then(|| key.to_string())
            })
            .collect()
    }

    /// The pairs of the arms `("section", "key") => ...` of `set_global`.
    fn pairs() -> BTreeSet<(String, String)> {
        function("set_global")
            .lines()
            .filter_map(|l| {
                let rest = l.trim().strip_prefix("(\"")?;
                let parts: Vec<&str> = rest.split('"').collect();
                (parts.len() > 3 && parts[1] == ", ")
                    .then(|| (parts[0].to_string(), parts[2].to_string()))
            })
            .collect()
    }

    fn table(section: &str) -> BTreeSet<String> {
        ENTRIES
            .iter()
            .filter(|e| e.section == section)
            .map(|e| e.key.to_string())
            .collect()
    }

    #[test]
    fn no_setting_is_listed_twice_and_every_field_is_filled() {
        let mut seen = BTreeSet::new();
        for e in ENTRIES {
            assert!(
                seen.insert((e.section, e.key)),
                "{}.{} twice",
                e.section,
                e.key
            );
            assert!(!e.section.is_empty() && !e.key.is_empty() && !e.default.is_empty());
            assert!(e
                .key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'));
        }
        assert!(ENTRIES.len() > 150);
    }

    #[test]
    fn the_list_has_every_key_the_reader_takes_and_no_other() {
        // the keys of a layer, of a profile and of a zone
        assert_eq!(table("layer"), single("set_layer"));
        assert_eq!(table("profile"), single("set_profile"));
        assert_eq!(table("zone"), single("set_zone"));
        // the keys of the other sections
        let from_reader = pairs();
        let from_list: BTreeSet<(String, String)> = ENTRIES
            .iter()
            .filter(|e| !matches!(e.section, "layer" | "profile" | "zone" | "material"))
            .map(|e| (e.section.to_string(), e.key.to_string()))
            .collect();
        assert_eq!(
            from_list.difference(&from_reader).collect::<Vec<_>>(),
            Vec::<&(String, String)>::new(),
            "listed but not read"
        );
        assert_eq!(
            from_reader.difference(&from_list).collect::<Vec<_>>(),
            Vec::<&(String, String)>::new(),
            "read but not listed"
        );
    }

    #[test]
    fn the_materials_of_a_library_are_listed_with_every_key_of_the_editor_and_the_marks() {
        let mut want: BTreeSet<String> = materials::KEYS.iter().map(|k| k.to_string()).collect();
        want.extend(["note", "verified", "max_power", "max_speed"].map(String::from));
        assert_eq!(table("material"), want);
        // each is a key the reader of a library takes
        for key in &want {
            let sample = match find("material", key).unwrap().kind {
                Kind::Number => "1",
                Kind::Whole => "1",
                Kind::Switch => "on",
                Kind::Word => "fs",
                Kind::Text => "x",
            };
            let text = format!("[material m]\n{key} = {sample}\n");
            assert!(materials::parse(&text).is_ok(), "{key}");
        }
    }

    #[test]
    fn every_default_that_is_a_value_is_accepted_by_the_reader() {
        let mut checked = 0;
        for e in ENTRIES {
            if matches!(e.section, "material" | "zone")
                || !matches!(e.kind, Kind::Number | Kind::Whole | Kind::Switch)
            {
                continue;
            }
            let value_ok = e.default.parse::<f64>().is_ok() || matches!(e.default, "on" | "off");
            if !value_ok {
                continue;
            }
            let header = match e.section {
                "layer" => "defaults".to_string(),
                "profile" => "profile p".to_string(),
                s => s.to_string(),
            };
            let text = format!("[{header}]\n{} = {}\n", e.key, e.default);
            if let Err(error) = vconfig::parse(&text) {
                panic!("{}.{} = {}: {error}", e.section, e.key, e.default);
            }
            checked += 1;
        }
        assert!(checked > 100, "{checked}");
        // and a file that says nothing is the defaults
        assert_eq!(vconfig::parse("").unwrap(), vconfig::Settings::default());
    }

    #[test]
    fn the_defaults_are_the_ones_of_the_documentation() {
        let doc = include_str!("../docs/vector-settings.md");
        for e in ENTRIES
            .iter()
            .filter(|e| e.section != "material" && e.default != "none")
        {
            let row = doc
                .lines()
                .filter(|l| l.starts_with("| `") && l.contains(&format!("`{}`", e.key)))
                .any(|l| {
                    l.split('|')
                        .nth(2)
                        .is_some_and(|c| c.trim().trim_matches('`') == e.default)
                });
            assert!(
                row,
                "{}.{} = {} is not in a row of the documentation",
                e.section, e.key, e.default
            );
        }
    }

    #[test]
    fn the_safety_settings_are_the_limits_the_zones_the_interlocks_and_the_token() {
        let safety: BTreeSet<String> = ENTRIES
            .iter()
            .filter(|e| e.safety)
            .map(|e| format!("{}.{}", e.section, e.key))
            .collect();
        for must in [
            "machine.max_power_percent",
            "machine.max_speed_mm_min",
            "frame.max_pilot_percent",
            "send.max_power_percent",
            "interlocks.estop",
            "interlocks.require_enclosure",
            "zone.width_mm",
            "server.server_token",
            "verify.overscan_mm",
        ] {
            assert!(safety.contains(must), "{must}");
        }
        // an ordinary setting of the look of the work is not
        for not in ["layer.power", "qr.module_mm", "read.dpi"] {
            assert!(!safety.contains(not), "{not}");
        }
        assert!(safety.len() >= 20);
        assert!(find("machine", "nothing").is_none());
        assert_eq!(find("send", "max_power_percent").unwrap().default, "40");
    }

    #[test]
    fn the_text_has_a_line_for_each_setting_and_the_json_reads_back() {
        let t = text();
        assert_eq!(t.lines().count(), ENTRIES.len() + 1);
        assert!(
            t.contains("send.max_power_percent  number  40  (safety)\n"),
            "{t}"
        );
        assert!(t.contains("layer.mode  word  cut\n"), "{t}");
        let j = json();
        let v = eh3d_serverclient::Json::parse(&j).unwrap();
        assert_eq!(
            v.get("program").and_then(eh3d_serverclient::Json::as_str),
            Some("phoenix-laser")
        );
        let rows = v.get("settings").unwrap();
        let count = match rows {
            eh3d_serverclient::Json::Arr(a) => a.len(),
            other => panic!("{other:?}"),
        };
        assert_eq!(count, ENTRIES.len());
        assert!(j.contains("\"safety\": true") && j.contains("\"safety\": false"));
        // a text with a quote or a backslash is escaped
        assert_eq!(json_string("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
    }
}
