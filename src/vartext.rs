// EH3D-PHOENIX-LASER - the variable text: a counter and the fields of a CSV file.
// Copyright (C) 2026 JuanenRac (Electro Hobby 3D). GPL-3.0-or-later.
//! The variable text of a series: one text with places for a counter and for the columns of a CSV file becomes one text for each row, so that a series of plates, each with its own
//! number or name, is burnt from one file. Nothing is ever executed: a place is `{counter}` (or `{counter:5}` for five figures with zeros in front), `{row}` (the number of the row,
//! from 1) or `{NAME}` (the column of the CSV file that has that name in its first row); `{{` and `}}` are the braces themselves. The CSV file is held to the limits of the settings (its
//! size, its rows and its columns), a row of another width than the first is an error, and a place that names nothing is an error: nothing is guessed, and the text of each row can be
//! looked at (`preview`) before any program is written.

use crate::vector::Limits;

/// The error of a CSV file that cannot be used.
pub const CODE_CSV: &str = "PHOENIX-LASER-VARTEXT-CSV";
/// The error of a place of the text that names nothing or is badly written.
pub const CODE_FIELD: &str = "PHOENIX-LASER-VARTEXT-FIELD";
/// The error of a counter that cannot be made.
pub const CODE_COUNTER: &str = "PHOENIX-LASER-VARTEXT-COUNTER";
/// The error of a series that is too long or empty.
pub const CODE_SERIES: &str = "PHOENIX-LASER-VARTEXT-SERIES";

/// The longest text one row may become, in characters.
pub const MAX_TEXT_CHARS: usize = 4096;
/// The most figures a counter is written with.
pub const MAX_COUNTER_WIDTH: usize = 12;
/// The longest a field of a CSV file may be, in characters.
pub const MAX_FIELD_CHARS: usize = 1024;

/// A CSV file read: the names of the columns (the first row) and the rows that follow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Csv {
    /// The names of the columns.
    pub header: Vec<String>,
    /// The rows, each with as many fields as the header.
    pub rows: Vec<Vec<String>>,
}

impl Csv {
    /// The index of a column by its name.
    pub fn column(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }
}

/// Reads a CSV file: fields separated by a comma (or by a semicolon when the first row has no comma), a field in double quotes may hold the separator, new lines and `""` for a quote;
/// blank lines are skipped. Nothing in it is ever run.
pub fn parse_csv(text: &str, lim: &Limits) -> Result<Csv, String> {
    if text.len() > lim.max_csv_bytes {
        return Err(format!(
            "{CODE_CSV}: the file is larger than {} bytes (max_csv_bytes)",
            lim.max_csv_bytes
        ));
    }
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let first_line = text.lines().next().unwrap_or("");
    let sep = if first_line.contains(',') || !first_line.contains(';') {
        ','
    } else {
        ';'
    };
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut was_quoted = false;
    let mut chars = text.chars().peekable();
    let mut field_chars = 0usize;
    while let Some(c) = chars.next() {
        field_chars += 1;
        if field_chars > MAX_FIELD_CHARS {
            return Err(format!(
                "{CODE_CSV}: a field is longer than {MAX_FIELD_CHARS} characters"
            ));
        }
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
            continue;
        }
        match c {
            '"' if field.is_empty() && !was_quoted => {
                quoted = true;
                was_quoted = true;
            }
            c if c == sep => {
                row.push(std::mem::take(&mut field));
                was_quoted = false;
                field_chars = 0;
            }
            '\r' => {}
            '\n' => {
                finish_row(&mut row, &mut field, &mut was_quoted, &mut records);
                field_chars = 0;
                if records.len() > lim.max_csv_rows.saturating_add(1) {
                    return Err(format!(
                        "{CODE_CSV}: more than {} rows (max_csv_rows)",
                        lim.max_csv_rows
                    ));
                }
            }
            c => field.push(c),
        }
        if row.len() > lim.max_csv_columns {
            return Err(format!(
                "{CODE_CSV}: more than {} columns (max_csv_columns)",
                lim.max_csv_columns
            ));
        }
    }
    if quoted {
        return Err(format!("{CODE_CSV}: a field in quotes is not closed"));
    }
    finish_row(&mut row, &mut field, &mut was_quoted, &mut records);
    let mut it = records.into_iter();
    let header: Vec<String> = it
        .next()
        .ok_or_else(|| format!("{CODE_CSV}: the file is empty"))?
        .into_iter()
        .map(|h| h.trim().to_string())
        .collect();
    if header.len() > lim.max_csv_columns {
        return Err(format!(
            "{CODE_CSV}: more than {} columns (max_csv_columns)",
            lim.max_csv_columns
        ));
    }
    for (i, h) in header.iter().enumerate() {
        if h.is_empty() {
            return Err(format!("{CODE_CSV}: column {} has no name", i + 1));
        }
        if header[..i].contains(h) {
            return Err(format!("{CODE_CSV}: the column {h} is named twice"));
        }
    }
    let rows: Vec<Vec<String>> = it.collect();
    if rows.len() > lim.max_csv_rows {
        return Err(format!(
            "{CODE_CSV}: more than {} rows (max_csv_rows)",
            lim.max_csv_rows
        ));
    }
    for (i, r) in rows.iter().enumerate() {
        if r.len() != header.len() {
            return Err(format!(
                "{CODE_CSV}: row {} has {} fields and the first row has {}",
                i + 1,
                r.len(),
                header.len()
            ));
        }
    }
    Ok(Csv { header, rows })
}

/// Ends a row of the CSV file; a row with nothing in it (a blank line) is dropped.
fn finish_row(
    row: &mut Vec<String>,
    field: &mut String,
    was_quoted: &mut bool,
    records: &mut Vec<Vec<String>>,
) {
    let blank = row.is_empty() && field.is_empty() && !*was_quoted;
    if !blank {
        row.push(std::mem::take(field));
        records.push(std::mem::take(row));
    }
    field.clear();
    *was_quoted = false;
}

/// A counter: `count` values from `start`, each `step` after the one before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counter {
    /// The first value.
    pub start: i64,
    /// The step (never 0).
    pub step: i64,
    /// How many values the series has.
    pub count: usize,
}

const COUNTER_RANGE: i64 = 1_000_000_000_000;

impl Counter {
    /// A counter from its three numbers.
    pub fn new(start: i64, step: i64, count: usize) -> Result<Counter, String> {
        if step == 0 {
            return Err(format!("{CODE_COUNTER}: the step of a counter is not 0"));
        }
        if start.abs() > COUNTER_RANGE || step.abs() > COUNTER_RANGE {
            return Err(format!(
                "{CODE_COUNTER}: the start and the step are from -{COUNTER_RANGE} to {COUNTER_RANGE}"
            ));
        }
        Ok(Counter { start, step, count })
    }

    /// The counter of the variable text of an LBRN2 project: from its current value towards its end, one `increment` at a time. A project that never ends before it starts has no series.
    pub fn from_range(current: i64, end: i64, increment: i64) -> Result<Counter, String> {
        if increment == 0 {
            return Err(format!(
                "{CODE_COUNTER}: the increment of a counter is not 0"
            ));
        }
        let span = if increment > 0 {
            end.checked_sub(current)
        } else {
            current.checked_sub(end)
        };
        let Some(span) = span.filter(|s| *s >= 0) else {
            return Err(format!("{CODE_COUNTER}: the counter ends before it starts"));
        };
        let count = usize::try_from(span / increment.abs() + 1)
            .map_err(|_| format!("{CODE_COUNTER}: the series is too long"))?;
        Counter::new(current, increment, count)
    }

    /// The value at a place of the series (0 is the first).
    pub fn value(&self, index: usize) -> Result<i64, String> {
        i64::try_from(index)
            .ok()
            .and_then(|i| i.checked_mul(self.step))
            .and_then(|d| self.start.checked_add(d))
            .filter(|v| v.abs() <= COUNTER_RANGE * 1000)
            .ok_or_else(|| format!("{CODE_COUNTER}: the value is out of range"))
    }
}

fn write_counter(value: i64, width: usize) -> String {
    let digits = value.unsigned_abs().to_string();
    let sign = if value < 0 { "-" } else { "" };
    let pad = width.saturating_sub(digits.len());
    format!("{sign}{}{digits}", "0".repeat(pad))
}

/// The text of one row: the places of `template` filled from the fields of the row (when there is a CSV file), the value of the counter and the number of the row (from 1).
pub fn render(
    template: &str,
    csv: Option<(&Csv, &[String])>,
    counter: Option<i64>,
    row_number: usize,
) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                out.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                out.push('}');
            }
            '}' => {
                return Err(format!(
                    "{CODE_FIELD}: a }} closes nothing (write }}}} for a brace)"
                ));
            }
            '{' => {
                let mut name = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(ch) if name.chars().count() < 64 => name.push(ch),
                        Some(_) => {
                            return Err(format!("{CODE_FIELD}: the name of a place is too long"))
                        }
                        None => {
                            return Err(format!(
                                "{CODE_FIELD}: a place is not closed (write {{{{ for a brace)"
                            ))
                        }
                    }
                }
                let name = name.trim();
                let (base, width) = match name.split_once(':') {
                    Some((b, w)) => (b.trim(), Some(w.trim())),
                    None => (name, None),
                };
                match base {
                    "counter" => {
                        let value = counter.ok_or_else(|| {
                            format!("{CODE_FIELD}: {{counter}} is used but no counter was given")
                        })?;
                        let width = match width {
                            None => 0,
                            Some(w) => w
                                .parse::<usize>()
                                .ok()
                                .filter(|n| (1..=MAX_COUNTER_WIDTH).contains(n))
                                .ok_or_else(|| {
                                    format!(
                                        "{CODE_FIELD}: the figures of a counter are from 1 to {MAX_COUNTER_WIDTH}"
                                    )
                                })?,
                        };
                        out.push_str(&write_counter(value, width));
                    }
                    "row" if width.is_none() => out.push_str(&row_number.to_string()),
                    _ if width.is_some() => {
                        return Err(format!("{CODE_FIELD}: only {{counter}} takes a width"));
                    }
                    column => {
                        let (csv, fields) = csv.ok_or_else(|| {
                            format!("{CODE_FIELD}: {{{column}}} names a column but no CSV file was given")
                        })?;
                        let at = csv.column(column).ok_or_else(|| {
                            format!("{CODE_FIELD}: the CSV file has no column {column}")
                        })?;
                        out.push_str(fields.get(at).map_or("", String::as_str));
                    }
                }
            }
            c => out.push(c),
        }
        if out.chars().count() > MAX_TEXT_CHARS {
            return Err(format!(
                "{CODE_FIELD}: a text is longer than {MAX_TEXT_CHARS} characters"
            ));
        }
    }
    Ok(out)
}

/// The text of every row of a series: one for each row of the CSV file, or one for each value of the counter when there is no CSV file. The series is held to `max_csv_rows`.
pub fn plan(
    template: &str,
    csv: Option<&Csv>,
    counter: Option<&Counter>,
    lim: &Limits,
) -> Result<Vec<String>, String> {
    let n = match (csv, counter) {
        (Some(c), _) => c.rows.len(),
        (None, Some(k)) => k.count,
        (None, None) => {
            return Err(format!(
                "{CODE_SERIES}: a series needs a CSV file or a counter"
            ));
        }
    };
    if n == 0 {
        return Err(format!("{CODE_SERIES}: the series has no row"));
    }
    if n > lim.max_csv_rows {
        return Err(format!(
            "{CODE_SERIES}: {n} rows are more than {} (max_csv_rows)",
            lim.max_csv_rows
        ));
    }
    (0..n)
        .map(|i| {
            let value = counter.map(|k| k.value(i)).transpose()?;
            let fields = csv.map(|c| (c, c.rows[i].as_slice()));
            render(template, fields, value, i + 1).map_err(|e| format!("row {}: {e}", i + 1))
        })
        .collect()
}
