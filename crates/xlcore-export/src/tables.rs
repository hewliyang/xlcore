use crate::schema::{Merge, Table, TableColumn, TableStyle};
use ooxmlsdk::parts::worksheet_part::WorksheetPart;
use xlcore_io::{parse_range, SpreadsheetDocument};

pub fn extract(doc: &mut SpreadsheetDocument, ws_part: &WorksheetPart) -> Vec<Table> {
    let mut out = Vec::new();

    let table_parts: Vec<_> = ws_part.table_definition_parts(doc).collect();

    for tp in &table_parts {
        let Ok(t) = tp.root_element(doc) else {
            continue;
        };
        let Some(((r1, c1), (r2, c2))) = parse_range(t.reference.as_str()) else {
            continue;
        };
        let display_name = t.display_name.as_str().to_string();
        let name = t
            .name
            .as_ref()
            .map(|s| s.as_str().to_string())
            .unwrap_or_else(|| display_name.clone());

        let header_row_count = t.header_row_count.unwrap_or(1);
        let totals_row_count = t.totals_row_count.unwrap_or(0);

        let columns: Vec<TableColumn> = t
            .table_columns
            .table_column
            .iter()
            .map(|c| TableColumn {
                name: c.name.as_str().to_string(),
                totals_row_function: c
                    .totals_row_function
                    .as_ref()
                    .map(|v| format!("{:?}", v).to_lowercase()),
                totals_row_label: c.totals_row_label.as_ref().map(|s| s.as_str().to_string()),
            })
            .collect();

        let style = t.table_style_info.as_ref().map(|s| TableStyle {
            name: s
                .name
                .as_ref()
                .map(|n| n.as_str().to_string())
                .unwrap_or_default(),
            show_first_column: s.show_first_column.unwrap_or(false.into()).into(),
            show_last_column: s.show_last_column.unwrap_or(false.into()).into(),
            show_row_stripes: s.show_row_stripes.unwrap_or(false.into()).into(),
            show_column_stripes: s.show_column_stripes.unwrap_or(false.into()).into(),
        });

        out.push(Table {
            name,
            display_name,
            range: Merge { r1, c1, r2, c2 },
            header_row_count,
            totals_row_count,
            columns,
            style,
            has_auto_filter: t.auto_filter.is_some(),
        });
    }
    out
}
