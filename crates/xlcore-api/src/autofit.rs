use std::collections::HashMap;

use ironcalc_base::formatter::format::format_number;
use ironcalc_base::locale::get_locale;
use ooxmlsdk::simple_type::BooleanValue;
use xlcore_types::{ApiCellValue as CellValue, FontPatch};

use crate::errors::sdk_err_to_api;
use crate::rowcols::{ensure_single_column, validate_column};
use crate::xml::{cell_info_from_cell, load_shared_strings};
use crate::{Result, Workbook};

const MAX_COLUMN: u32 = 16_384;
const MDW: f64 = 7.0;
const PADDING_PX: f64 = 9.0;
const DEFAULT_FONT_SIZE: f64 = 11.0;
const WIDTH_FUDGE: f64 = 1.18;
const EXCEL_MAX_WIDTH: f64 = 255.0;

impl Workbook {
    pub fn auto_fit_column(
        &mut self,
        sheet: impl AsRef<str>,
        column: u32,
        min_width: Option<f64>,
        max_width: Option<f64>,
    ) -> Result<f64> {
        let sheet = sheet.as_ref().to_string();
        validate_column(column, &sheet)?;

        let shared_strings = load_shared_strings(&mut self.doc);
        let ws_part = self.worksheet_part_for_sheet(&sheet)?;
        let ws = ws_part
            .root_element(&mut self.doc)
            .map_err(sdk_err_to_api)?;

        let mut cells: Vec<(CellValue, Option<u32>)> = Vec::new();
        for row in &ws.sheet_data.row {
            let row_idx = match row.row_index {
                Some(r) => r,
                None => continue,
            };
            for cell in &row.cell {
                let pos = cell
                    .cell_reference
                    .as_ref()
                    .and_then(|r| xlcore_io::parse_a1(r.as_str()));
                if pos != Some((row_idx, column)) {
                    continue;
                }
                let info =
                    cell_info_from_cell(&sheet, row_idx, column, Some(cell), &shared_strings);
                cells.push((info.value, info.style_index));
            }
        }

        let mut style_cache: HashMap<u32, (FontPatch, Option<String>)> = HashMap::new();
        let locale = get_locale("en").ok();
        let mut max_px = 0.0_f64;

        for (value, style_index) in &cells {
            let (font, num_fmt) = match style_index {
                Some(idx) => style_cache
                    .entry(*idx)
                    .or_insert_with(|| resolve_style(&mut self.doc, *idx))
                    .clone(),
                None => (FontPatch::default(), None),
            };
            let text = display_text(value, num_fmt.as_deref(), locale);
            let px = measure_text_px(&text, &font);
            if px > max_px {
                max_px = px;
            }
        }

        let mut width = if max_px <= 0.0 {
            0.0
        } else {
            (max_px + PADDING_PX) / MDW
        };
        width = (width * 100.0).round() / 100.0;
        if let Some(min) = min_width {
            width = width.max(min);
        }
        width = width.min(max_width.unwrap_or(EXCEL_MAX_WIDTH));

        let ws = ws_part
            .root_element_mut(&mut self.doc)
            .map_err(sdk_err_to_api)?;
        let col = ensure_single_column(ws, column);
        col.width = Some(width);
        col.custom_width = Some(BooleanValue::from_bool(true));
        col.best_fit = Some(BooleanValue::from_bool(true));
        Ok(width)
    }

    pub fn auto_fit_columns(
        &mut self,
        sheet: impl AsRef<str>,
        start: u32,
        end: u32,
        min_width: Option<f64>,
        max_width: Option<f64>,
    ) -> Result<Vec<f64>> {
        let sheet = sheet.as_ref().to_string();
        let (lo, hi) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        let mut widths = Vec::new();
        for column in lo..=hi.min(MAX_COLUMN) {
            widths.push(self.auto_fit_column(&sheet, column, min_width, max_width)?);
        }
        Ok(widths)
    }
}

fn resolve_style(
    doc: &mut xlcore_io::SpreadsheetDocument,
    style_index: u32,
) -> (FontPatch, Option<String>) {
    let patch = crate::styles::xf_to_style_patch(doc, style_index);
    match patch {
        Some(p) => (p.font.unwrap_or_default(), p.number_format),
        None => (FontPatch::default(), None),
    }
}

fn display_text(
    value: &CellValue,
    num_fmt: Option<&str>,
    locale: Option<&ironcalc_base::locale::Locale>,
) -> String {
    match value {
        CellValue::Blank => String::new(),
        CellValue::String(s) => s.clone(),
        CellValue::Boolean(b) => {
            if *b {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        CellValue::Error(e) => e.clone(),
        CellValue::Number(n) => match (num_fmt, locale) {
            (Some(fmt), Some(loc)) => format_number(*n, fmt, loc).text,
            _ => format_general(*n),
        },
    }
}

fn format_general(n: f64) -> String {
    if n == n.trunc() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        let mut s = format!("{n}");
        if let Some(pos) = s.find('.') {
            if s.len() - pos > 11 {
                s = format!("{n:.10}");
                while s.ends_with('0') {
                    s.pop();
                }
                if s.ends_with('.') {
                    s.pop();
                }
            }
        }
        s
    }
}

fn measure_text_px(text: &str, font: &FontPatch) -> f64 {
    if text.is_empty() {
        return 0.0;
    }
    let size = font.size.unwrap_or(DEFAULT_FONT_SIZE);
    let bold = font.bold.unwrap_or(false);
    let scale = size / DEFAULT_FONT_SIZE;
    let bold_factor = if bold { 1.07 } else { 1.0 };
    let mut max_line = 0.0_f64;
    for line in text.split('\n') {
        let mut w = 0.0_f64;
        for c in line.chars() {
            w += char_width_11(c);
        }
        if w > max_line {
            max_line = w;
        }
    }
    max_line * scale * bold_factor * WIDTH_FUDGE
}

fn char_width_11(c: char) -> f64 {
    match c {
        '0'..='9' => 7.0,
        ' ' => 3.6,
        'a' => 6.0,
        'b' | 'd' | 'g' | 'h' | 'n' | 'o' | 'p' | 'q' | 'u' => 6.5,
        'c' | 's' | 'z' => 5.0,
        'e' => 6.2,
        'f' => 3.6,
        'i' | 'j' | 'l' => 2.8,
        'k' | 'v' | 'x' | 'y' => 5.6,
        'm' => 9.8,
        'r' => 4.1,
        't' => 4.0,
        'w' => 8.6,
        'A' => 7.6,
        'B' | 'E' | 'Z' => 6.6,
        'C' | 'K' => 7.1,
        'D' | 'U' | 'V' => 7.9,
        'F' | 'P' => 6.4,
        'G' | 'O' | 'Q' => 8.3,
        'H' | 'N' | 'R' | 'T' => 7.4,
        'I' => 3.0,
        'J' => 4.0,
        'L' => 5.6,
        'M' => 9.7,
        'S' | 'Y' => 6.3,
        'W' => 10.6,
        'X' => 7.0,
        '.' | ',' | '\'' | '`' | '|' | '!' | ':' | ';' => 2.8,
        '-' => 4.4,
        '_' => 7.0,
        '(' | ')' | '[' | ']' | '{' | '}' | '/' | '\\' => 3.6,
        '\t' | '\r' => 0.0,
        _ if (c as u32) >= 0x1100 && is_wide(c) => 11.0,
        _ => 7.0,
    }
}

fn is_wide(c: char) -> bool {
    let cp = c as u32;
    (0x1100..=0x115F).contains(&cp)
        || (0x2E80..=0xA4CF).contains(&cp)
        || (0xAC00..=0xD7A3).contains(&cp)
        || (0xF900..=0xFAFF).contains(&cp)
        || (0xFF00..=0xFF60).contains(&cp)
        || (0xFFE0..=0xFFE6).contains(&cp)
}
