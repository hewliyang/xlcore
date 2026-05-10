//! Pivot table chrome extraction (cheap path).
//!
//! We do **not** rebuild pivots from the cache — the materialized result
//! cells are already in `<sheetData>` with the right cell xfs (header
//! band fill, white bold text, banded rows, "Grand Total" bold). All
//! this module produces is the bounding `<location>` plus the cells
//! that should get a filter-dropdown chevron painted on them, so the
//! renderer can finish the visual to match Excel/SpreadJS.

use crate::schema::{CellRef, Merge, Pivot};
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use xlcore_io::{parse_range, SpreadsheetDocument};

pub fn extract(doc: &mut SpreadsheetDocument, ws_part: &WorksheetPart) -> Vec<Pivot> {
    let mut out = Vec::new();
    let pivot_parts: Vec<_> = ws_part.pivot_table_parts(doc).map(|p| p.clone()).collect();

    for pp in &pivot_parts {
        let Ok(pt) = pp.root_element(doc) else { continue; };
        let Some(((r1, c1), (r2, c2))) = parse_range(pt.location.reference.as_str()) else {
            continue;
        };

        // Location attrs are 0-based offsets relative to (r1,c1).
        // For our minimal layout (no page fields, no compound fields):
        //   - `firstDataCol = N` ⇒ the row-field axis label sits in
        //     column `c1 + (N-1)` of the row directly above the data
        //     (i.e. row `r1 + firstDataRow - 1`).
        //   - `firstHeaderRow = M` + non-empty `<colFields>` ⇒ the
        //     column-field axis label sits at row `r1 + (M-1)`,
        //     column `c1 + firstDataCol`.
        let first_header_row = pt.location.first_header_row;
        let first_data_row = pt.location.first_data_row;
        let first_data_col = pt.location.first_data_column;
        let has_row_fields = pt
            .row_fields
            .as_ref()
            .map(|rf| !rf.x_field.is_empty())
            .unwrap_or(false);
        let has_col_fields = pt
            .column_fields
            .as_ref()
            .map(|cf| !cf.x_field.is_empty())
            .unwrap_or(false);

        let mut filter_arrow_cells = Vec::new();
        // Row-field axis label (e.g. "Region"). Lives one column inside
        // from the left edge when `firstDataCol >= 1`, in the row
        // directly above the first data row.
        if has_row_fields && first_data_row >= 1 && first_data_col >= 1 {
            let r = r1 + first_data_row - 1;
            let c = c1 + (first_data_col - 1);
            if r >= r1 && r <= r2 && c >= c1 && c <= c2 {
                filter_arrow_cells.push(CellRef { r, c });
            }
        }
        // Column-field axis label (e.g. "Product"). Sits at the top of
        // the data block when `firstHeaderRow >= 1`.
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
    out
}
