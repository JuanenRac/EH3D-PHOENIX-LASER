// EH3D-PHOENIX-LASER - laser engraving from an image.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
#![forbid(unsafe_code)]
//! An image becomes grey levels, the grey levels become dots (dithering), and the dots become the G-code of a laser that burns line by line with dynamic power. A vector drawing (SVG or DXF) has its contours cut and its shapes engraved with the settings of each layer.

/// The reader of PGM and PNG pictures, kept in EH3D-COMMON because the other programs that work from a picture read them the same way.
pub use eh3d_image as image;
pub mod barcode;
pub mod browse;
pub mod calibrate;
pub mod commonmat;
pub mod contract;
pub mod cut;
pub mod dither;
pub mod dxf;
pub mod eh3dfile;
pub mod energy;
pub mod export;
pub mod flatten;
pub mod frame;
pub mod gcode_out;
pub mod guard;
#[cfg(feature = "gui")]
pub mod gui;
pub mod job;
pub mod lang;
pub mod lbrn2;
pub mod lbrn2_pic;
pub mod linkpane;
pub mod live;
pub mod materials;
pub mod offset;
pub mod order;
pub mod panels;
pub mod presets;
pub mod preview;
pub mod qr;
pub mod raster;
pub mod register;
pub mod report;
pub mod safety;
pub mod server;
pub mod shot;
pub mod sublayers;
// what the window shows and asks about an `.eh3d` project, and its words, are shared by the programs (crate `eh3d-project` of EH3D-COMMON)
pub use eh3d_project::suitetext;
pub use eh3d_project::suiteview as suitecore;
pub mod suitewin;
pub mod svg;
pub mod testfiles;
pub mod text;
pub mod tone;
pub mod trace;
pub mod vartext;
pub mod vconfig;
pub mod vector;
pub mod verify;
pub mod viewport;
pub mod window;
