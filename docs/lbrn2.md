# LBRN2 projects

`phoenix-laser` reads `.lbrn2` project files, the project format of a laser program of another maker (its names are trademarks of their owners). `cut`, `vector`, `layers` and `convert` take one in place of an SVG or a DXF, and the window opens it like a drawing. There is no published specification of the format: the reader was written from the behaviour of real files, and what it does with each part is below. Nothing is copied from another program.

## What is read

The file is XML. The root is a `LightBurnProject` element; its attributes `AppVersion`, `DeviceName`, `FormatVersion`, `MirrorX` and `MirrorY` are kept and shown, nothing is done with them.

| Part | What the reader does |
|---|---|
| `CutSetting` and `CutSetting_Img` (a layer) | the `type` (`Cut`, `Scan`, `Image`; any other word is kept as written), and one child per value with a `Value` attribute. Read: `index`, `name`, `maxPower` (percent), `minPower`, `speed` (mm/s), `numPasses`, `interval` (mm between the lines of a fill), `priority` (the order of the layers) and `doOutput` (a layer that is not sent to the laser is skipped). Every value is kept as written. |
| `Shape` with `CutIndex` | the layer of the shape is the layer with that number (its name, or `C` and the number with two digits when it has none). |
| `XForm` | six numbers `a b c d e f`: `x' = a x + c y + e`, `y' = b x + d y + f`, from the space of the shape to the space of its parent. A shape whose transform is not six numbers has no place and is left out. |
| `Rect` | `W` and `H`, centred on its origin; `Cr` is a corner radius (it was zero in every real example, so the rounded corner is the part least checked). |
| `Path` | `VertList`: vertices written `V x y`, each followed by the optional handles `c0x c0y c1x c1y`; a handle given with only its x (the value 1) is no handle. `PrimList`: `L a b` is a line from vertex `a` to vertex `b`, `B a b` a cubic Bezier that leaves `a` through its `c0` and arrives at `b` through its `c1`; the word `LineClosed` is a closed polygon through all the vertices. A new outline starts where a piece does not begin at the end of the one before; an outline that returns to its first vertex is closed. |
| `VertID`, `PrimID` | a path may leave its lists out when an earlier path wrote them under the same number; a number nobody wrote skips the path with a note. |
| `Group` with `Children` | the matrix of the child acts first, the matrix of the group second, at any depth. |
| `Text` | drawn from its `BackupPath`, the outline of the letters, whose `XForm` is the one that places the text (it is applied once). A text with no outline is left out. |

The curves are cut into straight pieces that are never farther than `tolerance_mm` of `[read]` from the curve. The limits of `[limits]` hold: the size of the file, the depth of the elements, their number, the shapes, the points and the coordinates; a declared entity is an error.

The y axis of the file is taken as the one of an SVG (it points down), so the drawing is turned over when it is cut. That was not confirmed on a machine: if a cut comes out as a mirror image, this is where to look.

## What the layers give the table

A layer of the project becomes a row of the table of layers with: the mode (`cut` for a `Cut`, `engrave` for the rest, `skip` for a layer that is not sent to the laser), the power, the speed in mm/min (the file has mm/s, so the value is multiplied by 60), the passes, the distance between the lines of a fill and the order. They are hints: the limits of the machine hold over them when the work is made (a power over the limit is an error that names the layer, never lowered), a layer that the settings file names keeps what the settings file says, and the window notes that the values are the project's and have to be checked for the machine.

## What is left out

Said in the notes of the drawing and counted:

* a picture inside the project (`Bitmap`);
* a shape of a type that was not seen in the real examples (any `Type` other than `Rect`, `Path`, `Group`, `Text` and `Bitmap`);
* a text with no outline;
* a path whose lists are missing or damaged, and a rectangle without a size.

The rest of the file (the preview picture, the variable text, the options of the optimiser, the tabs and the settings of picture layers, the notes) is not used.

## How it was measured

The reader was run over twelve real projects made by a laser maker, of versions 1.5 to 2.0 of the program that wrote them, from 40 kB to 10 MB, kept outside of this repository and read only (they are not copied here and no test uses them; the tests use projects written for them with the same structure). All twelve are read, in 34 ms or less each; 4,300 paths, 47 groups, two rectangles and one text are drawn; one picture (the largest file) is left out. Two details were confirmed against the files themselves: the speeds and the powers of the layers agree with the values the maker wrote in the notes that come with them (the speed is in mm/s), and the drawing of the plate with a text measures exactly the size of its rectangle when the transform of the text is applied once. The direction of the handles (`c0` leaves the first vertex, `c1` arrives at the second) holds for 98 % of the 16,600 Bezier pieces that give both handles (the handle points towards the other end of the piece); in the rest it points the other way, as it does at a cusp.

`examples/lbrn2_survey.rs` is the tool: `cargo run --release --example lbrn2_survey -- FOLDER [--out REPORT.txt]` reads every `.lbrn2` file under a folder and says which are read, which are not and why, what each holds, and which elements and attributes the files have that the reader does not use. The exit code is 2 when a file is not read. It writes nothing but its report.

## Not tried

No program made from a project has been cut on a machine. The orientation of the y axis, the corner radius of a rectangle and the shapes that are not in the examples (ellipses, polygons, arrays, pictures) are not confirmed.
