# Security policy

EH3D-PHOENIX-LASER reads files that other people may send. It never panics on them and refuses a value that makes no sense, but it has had no independent review.

What it does **not** do: it does not know the limits of a machine and it does not decide whether what it produces is safe to run. A result that was produced without errors is not a verified result; a person must check it before anything is run on a machine.

What it writes: only the file that is named as the output of a command (the G-code program of `engrave`, `calibrate`, `cut`, `vector`, `trace`, `text`, `qr`, `frame` and `focus`, or the SVG picture of `preview`), written beside and renamed so that a file is never left half written, and never over a file it is made from (the picture, the drawing, the settings or the library). `text` and `qr` write only strokes of their own: the words or the data of the person are never written into a comment of the program, and a letter that the font has not, a text or data over the limits of characters, bytes, version, size, height, power or speed, or work outside of the bed is refused with nothing written.

The only network connection it makes is the one of `phoenix-laser server` to the address in `[server]` of the settings file (plain HTTP, one request at a time, to the EH3D server of that address): the token is sent in the clear, so use it on a network you trust or through a tunnel, and it is never printed. No other command opens a connection. A program is checked against the limits of the machine before it is sent, and the stop of a job is an emergency stop that latches the machine until an administrator releases it.

Please report a vulnerability privately to **electrohobby3d@gmail.com**, with the file or the steps that show it. Do not open a public issue for it.
