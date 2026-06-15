use crate::errors::sdk_err_to_api;
use crate::*;

fn outline_levels(workbook: &mut Workbook, sheet: &str) -> (Vec<(u32, u8, bool)>, Option<u8>) {
    let part = workbook.worksheet_part_for_sheet(sheet).unwrap();
    let ws = part
        .root_element(&mut workbook.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    let rows = ws
        .sheet_data
        .row
        .iter()
        .filter_map(|r| {
            r.outline_level.map(|lvl| {
                (
                    r.row_index.unwrap_or(0),
                    u8::from(lvl),
                    r.hidden.map(bool::from).unwrap_or(false),
                )
            })
        })
        .collect();
    let max = ws
        .sheet_format_properties
        .as_ref()
        .and_then(|f| f.outline_level_row.map(u8::from));
    (rows, max)
}

#[test]
fn group_rows_sets_outline_and_roundtrips() {
    let mut workbook = Workbook::new().unwrap();
    workbook.group_rows("Sheet1", 2, 4, 1, false).unwrap();

    let (rows, max) = outline_levels(&mut workbook, "Sheet1");
    assert_eq!(rows, vec![(2, 1, false), (3, 1, false), (4, 1, false)]);
    assert_eq!(max, Some(1));

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let (rows, max) = outline_levels(&mut reopened, "Sheet1");
    assert_eq!(rows, vec![(2, 1, false), (3, 1, false), (4, 1, false)]);
    assert_eq!(max, Some(1));
}

#[test]
fn group_rows_collapsed_hides_and_marks_summary() {
    let mut workbook = Workbook::new().unwrap();
    workbook.group_rows("Sheet1", 2, 4, 1, true).unwrap();

    let part = workbook.worksheet_part_for_sheet("Sheet1").unwrap();
    let ws = part
        .root_element(&mut workbook.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    for row in 2..=4 {
        let entry = ws
            .sheet_data
            .row
            .iter()
            .find(|r| r.row_index == Some(row))
            .unwrap();
        assert!(
            entry.hidden.map(bool::from).unwrap_or(false),
            "row {row} hidden"
        );
    }
    let summary = ws
        .sheet_data
        .row
        .iter()
        .find(|r| r.row_index == Some(5))
        .unwrap();
    assert!(summary.collapsed.map(bool::from).unwrap_or(false));
}

#[test]
fn ungroup_rows_clears_outline() {
    let mut workbook = Workbook::new().unwrap();
    workbook.group_rows("Sheet1", 2, 4, 1, false).unwrap();
    workbook.group_rows("Sheet1", 2, 4, 0, false).unwrap();
    let (rows, max) = outline_levels(&mut workbook, "Sheet1");
    assert!(rows.is_empty());
    assert_eq!(max, None);
}

#[test]
fn group_columns_sets_outline_and_roundtrips() {
    let mut workbook = Workbook::new().unwrap();
    workbook.group_columns("Sheet1", 2, 3, 1, false).unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let part = reopened.worksheet_part_for_sheet("Sheet1").unwrap();
    let ws = part
        .root_element(&mut reopened.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    let levels: Vec<(u32, u32, u8)> = ws
        .columns
        .first()
        .map(|cols| {
            cols.column
                .iter()
                .filter_map(|c| c.outline_level.map(|l| (c.min, c.max, u8::from(l))))
                .collect()
        })
        .unwrap_or_default();
    assert!(levels
        .iter()
        .any(|&(min, max, lvl)| min <= 2 && 2 <= max && lvl == 1));
    assert!(levels
        .iter()
        .any(|&(min, max, lvl)| min <= 3 && 3 <= max && lvl == 1));
    let max = ws
        .sheet_format_properties
        .as_ref()
        .and_then(|f| f.outline_level_column.map(u8::from));
    assert_eq!(max, Some(1));
}

#[test]
fn group_rows_rejects_bad_range() {
    let mut workbook = Workbook::new().unwrap();
    assert!(workbook.group_rows("Sheet1", 4, 2, 1, false).is_err());
    assert!(workbook.group_rows("Sheet1", 2, 4, 8, false).is_err());
}

fn col_width(workbook: &mut Workbook, sheet: &str, column: u32) -> Option<f64> {
    let part = workbook.worksheet_part_for_sheet(sheet).unwrap();
    let ws = part
        .root_element(&mut workbook.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    ws.columns.first().and_then(|cols| {
        cols.column
            .iter()
            .find(|c| c.min <= column && column <= c.max)
            .and_then(|c| c.width)
    })
}

#[test]
fn auto_fit_column_widens_for_long_text() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value_in("Sheet1", "A1", "Hi").unwrap();
    workbook.set_value_in("Sheet1", "A2", "A").unwrap();
    let narrow = workbook.auto_fit_column("Sheet1", 1, None, None).unwrap();

    let mut workbook2 = Workbook::new().unwrap();
    workbook2
        .set_value_in("Sheet1", "A1", "A considerably longer label here")
        .unwrap();
    let wide = workbook2.auto_fit_column("Sheet1", 1, None, None).unwrap();

    assert!(wide > narrow, "wide={wide} narrow={narrow}");
    assert!(narrow > 0.0);
}

#[test]
fn auto_fit_column_sets_best_fit_and_roundtrips() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_value_in("Sheet1", "B3", "Hello World")
        .unwrap();
    let width = workbook.auto_fit_column("Sheet1", 2, None, None).unwrap();
    assert!(col_width(&mut workbook, "Sheet1", 2).is_some());

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let rt = col_width(&mut reopened, "Sheet1", 2).unwrap();
    assert!((rt - width).abs() < 1e-9);
}

#[test]
fn auto_fit_column_respects_min_and_max() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value_in("Sheet1", "A1", "x").unwrap();
    let w = workbook
        .auto_fit_column("Sheet1", 1, Some(20.0), None)
        .unwrap();
    assert!(w >= 20.0);

    let mut workbook2 = Workbook::new().unwrap();
    workbook2
        .set_value_in("Sheet1", "A1", "this is a very very long string value")
        .unwrap();
    let w2 = workbook2
        .auto_fit_column("Sheet1", 1, None, Some(8.0))
        .unwrap();
    assert!(w2 <= 8.0);
}

#[test]
fn auto_fit_column_uses_number_format() {
    let mut plain = Workbook::new().unwrap();
    plain.set_value_in("Sheet1", "A1", 1234.5).unwrap();
    let plain_w = plain.auto_fit_column("Sheet1", 1, None, None).unwrap();

    let mut fmt = Workbook::new().unwrap();
    fmt.set_value_in("Sheet1", "A1", 1234.5).unwrap();
    fmt.set_style_in(
        "Sheet1",
        "A1",
        StylePatch {
            number_format: Some("$#,##0.00".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    let fmt_w = fmt.auto_fit_column("Sheet1", 1, None, None).unwrap();
    assert!(fmt_w > plain_w, "fmt={fmt_w} plain={plain_w}");
}
