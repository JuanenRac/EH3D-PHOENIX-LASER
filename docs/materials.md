# The library of materials

What suits a material is found once with the test grid (`phoenix-laser calibrate`) and then used again and again. A library is a plain text file with one `[material NAME]` section for each material **and way of working** (the cut of 3 mm birch plywood and its engraving are two entries). `examples/materials.cfg` shows every key. The settings file of the drawings is described in [vector-settings.md](vector-settings.md). The file is chosen with `--materials FILE` or with the variable `PHOENIX_LASER_MATERIALS`, and one material of it with `--material NAME` (capitals and the spaces around the name do not matter) on `engrave`, `cut`, `vector` and `layers`; `phoenix-laser materials FILE` lists what a library holds.

Every key is optional, and a command takes from a material only what it says. The order is: the settings file, then the material (for a drawing, over `[defaults]` and every layer, like an option of the command line), then the options of the command line, which win. The limit of power (`max_power_percent`, `--max-power`) holds over all of them: a material whose power is over it is an error, never lowered.

| Key | Range | Used by | Meaning |
|---|---|---|---|
| `note` | up to 300 characters | all | a reminder shown in the list |
| `power` | 0.001 to 100 | all | percent of full power |
| `speed` | 1 to 30000 | all | mm/min (the feed of an engraving) |
| `passes` | 1 to 1000 | drawings | how many times |
| `focus_step` | -20 to 20 | drawings | mm the focus moves at each pass |
| `kerf` | 0 to 5 | drawings | the width of the beam, mm |
| `hatch_spacing`, `hatch_angle` | 0.01 to 5, -360 to 360 | drawings | the hatch of the shapes that are engraved |
| `hatch_cross` | `on` / `off` | drawings | fill the engraved shapes a second time at right angles to the first lines |
| `dot` | 0.01 to 5 | pictures | the size of a dot, mm |
| `overscan` | 0 to 50 | pictures | the run past the work at each end of a line, mm |
| `dither` | `fs`, `atkinson`, `threshold`, `bayer` | pictures | how the grey levels become dots |
| `threshold` | 0 to 255 | pictures | the level of `dither = threshold` (128 when it is not said) |
| `gamma`, `contrast`, `brightness` | 0.1 to 10, -100 to 100, -100 to 100 | pictures | the tone curve (see `phoenix-laser help`) |
| `invert`, `auto_level` | `on` / `off` | pictures | the tone curve |
| `verified` | `on` / `off` | all | whether a person tried the values on their machine: `off` says they are only a place to start from, and using such a material prints a note; a material that does not say is left unmarked |
| `max_power`, `max_speed` | 0.001 to 100, 1 to 30000 | all | the ceiling of this material: a library whose `power` or `speed` is over it is refused. The window keeps `verified` and these two when a material is edited, though it has no box for them |

`examples/materials-starter.cfg` is a library of seven places to start from for a weak diode laser, every one marked `verified = off` and with a ceiling of its own: the values are round numbers from general experience, not copied from any table, and nobody has tried them on a real machine. Test each on a scrap with `calibrate`, write what works for you in your own library and mark it `verified = on`.

The library is read as a whole, so a line that is not understood, a key that is not known, a value out of its range, a name used twice or a file larger than 1 MiB is an error that names the line. The values are the ones the person found for their own machine: the program knows no material.

In the window the material section edits a library without a text editor: a material can be edited, copied, made from the boxes of the window and deleted, and a new library can be created. One box for each key of the table, named with a word in the language of the window and the key of the file beside it, holds its value (an empty box leaves it out), and saving rewrites the section of that material in its place, or adds it at the end when it is new, and changes nothing else in the file: the comments, the blank lines, the order, the line ends and the other materials stay as they were. A value out of the range of the table, a name that is empty, longer than 80 characters or already used, a note over 300 characters, a library that would pass 1 MiB or 500 materials, and a file that is not UTF-8 text or not a valid library are refused with the key named and the file untouched; nothing is cut down. Deleting needs a second press of the button, and a new library is never written over a file that is there.

## Presets: assign and link

A preset is a power, a speed and a number of passes for a material and, when it is cut, a thickness. It is given to a layer, or to a sub-layer, in one of two ways. **Assign** copies its values into the layer and the layer is then its own: a later change of the preset does not reach it. **Link** keeps the layer tied to the preset: when the values of the preset change, every linked layer changes with them, the change says which layers it reached and from what to what, and the book of presets shows each layer as `[linked]` or `[assigned]`. Cutting the link of a layer leaves it with the values it has.

The values come from a person or from the common database of materials (feature `common-machining`), exactly as the database gives them. The database's seed was never tried on a machine, so every preset made from it is marked as not tried, and every place that shows it carries the warning in the language of the window; a preset that a person changes is marked as not tried again, because the values that were tried are no longer the ones in use. A second figure that another source gives for the same case (an alternative of the database) is kept apart: it is listed on request and is never given to a layer unless the person asks for it.

A value over a limit of the machine (`max_power_percent`, `max_speed_mm_min`, `max_passes`) is an error that names the preset and the layer, never lowered in silence, and a change of a preset that any linked layer cannot take is refused as a whole: the preset and every layer stay as they were. The links are kept in memory; the settings file and the window do not hold them yet.
