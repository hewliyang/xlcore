use crate::pivot_engine::PivotStyleIndices;
use crate::schema::{Cell, CellRef, Merge, Pivot, Styles};
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use xlcore_io::{parse_range, SpreadsheetDocument};

pub fn extract(
    doc: &mut SpreadsheetDocument,
    ws_part: &WorksheetPart,
    styles: &mut Styles,
    style_memo: &mut Option<PivotStyleIndices>,
) -> (Vec<Pivot>, Vec<Cell>) {
    let mut out = Vec::new();
    let mut cells = Vec::new();
    let pivot_parts: Vec<_> = ws_part.pivot_table_parts(doc).collect();

    for pp in &pivot_parts {
        let pt = match pp.root_element(doc) {
            Ok(pt) => pt.clone(),
            Err(_) => continue,
        };

        let cache = pp
            .pivot_table_cache_definition_part(doc)
            .and_then(|def_part| {
                let rec_part = def_part.pivot_table_cache_records_part(doc)?;
                let cache_def = def_part.root_element(doc).ok()?.clone();
                let records = rec_part.root_element(doc).ok()?.clone();
                Some((cache_def, records))
            });
        if let Some((cache_def, records)) = cache {
            cells.extend(crate::pivot_engine::compute_cells(
                &pt, &cache_def, &records, styles, style_memo,
            ));
        }

        let Some(((r1, c1), (r2, c2))) = parse_range(pt.location.reference.as_str()) else {
            continue;
        };

        let first_header_row = pt.location.first_header_row;
        let first_data_row = pt.location.first_data_row;
        let first_data_col = pt.location.first_data_column;
        let has_row_fields = pt
            .row_fields
            .as_ref()
            .map(|rf| !rf.field.is_empty())
            .unwrap_or(false);
        let has_col_fields = pt
            .column_fields
            .as_ref()
            .map(|cf| !cf.field.is_empty())
            .unwrap_or(false);

        let mut filter_arrow_cells = Vec::new();

        if has_row_fields && first_data_row >= 1 && first_data_col >= 1 {
            let r = r1 + first_data_row - 1;
            let c = c1 + (first_data_col - 1);
            if r >= r1 && r <= r2 && c >= c1 && c <= c2 {
                filter_arrow_cells.push(CellRef { r, c });
            }
        }

        if has_col_fields && first_header_row >= 1 {
            let r = r1 + (first_header_row - 1);
            let c = c1 + first_data_col;
            if r >= r1 && r <= r2 && c >= c1 && c <= c2 {
                filter_arrow_cells.push(CellRef { r, c });
            }
        }

        out.push(Pivot {
            name: pt.name.as_str().to_string(),
            range: Merge { r1, c1, r2, c2 },
            filter_arrow_cells,
        });
    }
    (out, cells)
}
