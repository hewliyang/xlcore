use crate::schema::{CellRef, Merge, Pivot};
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use xlcore_io::{parse_range, SpreadsheetDocument};

pub fn extract(doc: &mut SpreadsheetDocument, ws_part: &WorksheetPart) -> Vec<Pivot> {
    let mut out = Vec::new();
    let pivot_parts: Vec<_> = ws_part.pivot_table_parts(doc).collect();

    for pp in &pivot_parts {
        let Ok(pt) = pp.root_element(doc) else {
            continue;
        };
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
    out
}
