# Security policy

EH3D-PHOENIX-LASER reads files that other people may send. It never panics on them and refuses a value that makes no sense, but it has had no independent review.

What it does **not** do: it does not know the limits of a machine and it does not decide whether what it produces is safe to run. A result that was produced without errors is not a verified result; a person must check it before anything is run on a machine.

What it writes: only the file that is named as the output of a command (the G-code program of `engrave`, `calibrate`, `cut`, `vector`, `trace`, `text`, `frame` and `focus`, or the SVG picture of `preview`), written beside and renamed so that a file is never left half written, and never over a file it is made from (the picture, the drawing, the settings or the library). `text` writes only the strokes of its own font: the words of the person are never written into a comment of the program, and a letter that the font has not, a text over the limits of characters, height, power or speed, or work outside of the bed is refused with nothing written.

Please report a vulnerability privately to **electrohobby3d@gmail.com**, with the file or the steps that show it. Do not open a public issue for it.
