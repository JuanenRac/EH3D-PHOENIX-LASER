//! What every G-code file of the program shares: the one way a number is written (a number that is not finite is refused, never written as `NaN` or `inf` for the controller to
//! choke on), the SHA-256 of the source file and the standard header with the generator, its version, the units, the file the work came from and the tools. They are the ones of
//! `eh3d-parser-gcode` (EH3D-CORE-LIBS), the same for every program of the suite, and are re-exported here under the names the program has always used.

pub use eh3d_parser_gcode::emit::{header, number, sha256_hex, Header};
