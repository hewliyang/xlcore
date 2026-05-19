use crate::schema::{ColumnarCells, RowMetaBlob, Sheet, TextRun, WorkbookLayout};
use base64::Engine;
use std::collections::HashMap;

pub fn compactify(layout: &mut WorkbookLayout) {
    for sheet in layout.sheets.iter_mut() {
        compactify_sheet(sheet);
    }
}

fn compactify_sheet(sheet: &mut Sheet) {
    let rows = std::mem::take(&mut sheet.rows);

    let mut value_pool: Vec<String> = Vec::new();
    let mut value_index: HashMap<String, i32> = HashMap::new();
    let mut formula_pool: Vec<String> = Vec::new();
    let mut formula_index: HashMap<String, i32> = HashMap::new();
    let mut inline_runs: Vec<Vec<TextRun>> = Vec::new();

    let total_cells: usize = rows.iter().map(|r| r.cells.len()).sum();
    let mut col_r: Vec<u32> = Vec::with_capacity(total_cells);
    let mut col_c: Vec<u32> = Vec::with_capacity(total_cells);
    let mut col_kind: Vec<u8> = Vec::with_capacity(total_cells);
    let mut col_value: Vec<i32> = Vec::with_capacity(total_cells);
    let mut col_formula: Vec<i32> = Vec::with_capacity(total_cells);
    let mut col_style: Vec<i32> = Vec::with_capacity(total_cells);
    let mut col_runs: Vec<i32> = Vec::with_capacity(total_cells);

    let row_count = rows.len();
    let mut row_index: Vec<u32> = Vec::with_capacity(row_count);
    let mut row_height: Vec<f32> = Vec::with_capacity(row_count);
    let mut row_style: Vec<i32> = Vec::with_capacity(row_count);
    let mut row_hidden: Vec<u8> = Vec::with_capacity(row_count);

    let mut row_outline: Vec<u8> = Vec::with_capacity(row_count);
    let mut any_outline = false;
    let mut row_ptr: Vec<u32> = Vec::with_capacity(row_count + 1);
    row_ptr.push(0);

    let mut rows = rows;
    rows.sort_by_key(|r| r.index);

    for row in rows.into_iter() {
        row_index.push(row.index);
        row_height.push(row.height_px.unwrap_or(f32::NAN));
        row_style.push(row.style_index.map(|x| x as i32).unwrap_or(-1));
        row_hidden.push(if row.hidden { 1 } else { 0 });
        if row.outline_level != 0 {
            any_outline = true;
        }
        row_outline.push(row.outline_level);

        let mut cells = row.cells;
        cells.sort_by_key(|c| c.c);

        for cell in cells.into_iter() {
            col_r.push(cell.r);
            col_c.push(cell.c);
            col_kind.push(kind_to_u8(&cell.kind));

            col_value.push(match cell.value {
                Some(v) => intern(&mut value_pool, &mut value_index, v),
                None => -1,
            });
            col_formula.push(match cell.formula {
                Some(v) => intern(&mut formula_pool, &mut formula_index, v),
                None => -1,
            });
            col_style.push(cell.style_index.map(|x| x as i32).unwrap_or(-1));
            col_runs.push(if cell.runs.is_empty() {
                -1
            } else {
                let idx = inline_runs.len() as i32;
                inline_runs.push(cell.runs);
                idx
            });
        }
        row_ptr.push(col_r.len() as u32);
    }

    sheet.cells = ColumnarCells {
        count: col_r.len() as u32,
        r: encode_u32(&col_r),
        c: encode_u32(&col_c),
        kind: encode_u8(&col_kind),
        value_idx: encode_i32(&col_value),
        formula_idx: encode_i32(&col_formula),
        style_idx: encode_i32(&col_style),
        runs_idx: encode_i32(&col_runs),
        row_ptr: encode_u32(&row_ptr),
    };
    sheet.row_meta = RowMetaBlob {
        count: row_count as u32,
        index: encode_u32(&row_index),
        height_px: encode_f32(&row_height),
        style_idx: encode_i32(&row_style),
        hidden: encode_u8(&row_hidden),

        outline_level: if any_outline {
            encode_u8(&row_outline)
        } else {
            String::new()
        },
    };
    sheet.value_pool = value_pool;
    sheet.formula_pool = formula_pool;
    sheet.inline_runs = inline_runs;
}

fn intern(pool: &mut Vec<String>, index: &mut HashMap<String, i32>, s: String) -> i32 {
    if let Some(&i) = index.get(&s) {
        return i;
    }
    let i = pool.len() as i32;
    index.insert(s.clone(), i);
    pool.push(s);
    i
}

fn kind_to_u8(k: &str) -> u8 {
    match k {
        "n" => 0,
        "s" => 1,
        "inline" => 2,
        "b" => 3,
        "e" => 4,
        "str" => 5,
        "f" => 6,
        _ => 0,
    }
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}
fn encode_u8(v: &[u8]) -> String {
    b64(v)
}
fn encode_u32(v: &[u32]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    b64(&bytes)
}
fn encode_i32(v: &[i32]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    b64(&bytes)
}
fn encode_f32(v: &[f32]) -> String {
    let mut bytes = Vec::with_capacity(v.len() * 4);
    for x in v {
        bytes.extend_from_slice(&x.to_le_bytes());
    }
    b64(&bytes)
}
