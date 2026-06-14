use crate::*;

#[test]
fn auto_filter_set_get_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_range_values(
        "Sheet1!A1:C1",
        vec![vec![
            CellValue::String("Region".into()),
            CellValue::String("Units".into()),
            CellValue::String("Rev".into()),
        ]],
    )
    .unwrap();
    wb.set_range_values(
        "Sheet1!A2:C3",
        vec![
            vec![
                CellValue::String("North".into()),
                CellValue::Number(10.0),
                CellValue::Number(99.0),
            ],
            vec![
                CellValue::String("South".into()),
                CellValue::Number(20.0),
                CellValue::Number(199.0),
            ],
        ],
    )
    .unwrap();

    assert!(wb.auto_filter("Sheet1").unwrap().is_none());

    let info = wb.set_auto_filter("Sheet1", "A1:C3").unwrap();
    assert_eq!(info.sheet, "Sheet1");
    assert_eq!(info.reference, "A1:C3");
    assert_eq!(info.start_row, 1);
    assert_eq!(info.end_row, 3);
    assert_eq!(info.end_column, 3);

    let got = wb.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(got.reference, "A1:C3");

    wb.set_auto_filter("Sheet1", "A1:B3").unwrap();
    let replaced = wb.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(replaced.reference, "A1:B3");

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(after.reference, "A1:B3");

    let removed = reopened.remove_auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(removed.reference, "A1:B3");
    assert!(reopened.auto_filter("Sheet1").unwrap().is_none());
    assert!(reopened.remove_auto_filter("Sheet1").unwrap().is_none());

    let err = reopened.set_auto_filter("Ghost", "A1:B2").unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}

#[test]
fn auto_filter_column_criteria_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_range_values(
        "Sheet1!A1:C1",
        vec![vec![
            CellValue::String("Region".into()),
            CellValue::String("Units".into()),
            CellValue::String("Rev".into()),
        ]],
    )
    .unwrap();
    wb.set_auto_filter("Sheet1", "A1:C3").unwrap();

    let info = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Values {
                    values: Vec::new(),
                    blank: Some(true),
                },
            },
        )
        .unwrap();
    assert_eq!(info.column_offset, 0);
    assert!(matches!(info.criteria, AutoFilterCriteria::Values { .. }));

    wb.set_auto_filter_column(
        "Sheet1",
        AutoFilterColumnPatch {
            column_offset: 1,
            hidden_button: None,
            show_button: None,
            criteria: AutoFilterCriteria::Top10 {
                top: Some(true),
                percent: Some(false),
                val: 5.0,
            },
        },
    )
    .unwrap();

    wb.set_auto_filter_column(
        "Sheet1",
        AutoFilterColumnPatch {
            column_offset: 2,
            hidden_button: None,
            show_button: None,
            criteria: AutoFilterCriteria::Custom {
                logical_and: Some(true),
                criteria: vec![
                    AutoFilterCustomCriterion {
                        operator: AutoFilterOperator::GreaterThanOrEqual,
                        value: "50".to_string(),
                    },
                    AutoFilterCustomCriterion {
                        operator: AutoFilterOperator::LessThan,
                        value: "500".to_string(),
                    },
                ],
            },
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(after.columns.len(), 3);
    match &after.columns[0].criteria {
        AutoFilterCriteria::Values { values, blank } => {
            assert!(values.is_empty());
            assert_eq!(*blank, Some(true));
        }
        other => panic!("expected Values, got {other:?}"),
    }
    match &after.columns[1].criteria {
        AutoFilterCriteria::Top10 { top, percent, val } => {
            assert_eq!(*top, Some(true));
            assert_eq!(*percent, Some(false));
            assert_eq!(*val, 5.0);
        }
        other => panic!("expected Top10, got {other:?}"),
    }
    match &after.columns[2].criteria {
        AutoFilterCriteria::Custom {
            logical_and,
            criteria,
        } => {
            assert_eq!(*logical_and, Some(true));
            assert_eq!(criteria.len(), 2);
            assert_eq!(criteria[0].operator, AutoFilterOperator::GreaterThanOrEqual);
            assert_eq!(criteria[0].value, "50");
            assert_eq!(criteria[1].operator, AutoFilterOperator::LessThan);
        }
        other => panic!("expected Custom, got {other:?}"),
    }

    let removed = reopened
        .remove_auto_filter_column("Sheet1", 1)
        .unwrap()
        .unwrap();
    assert!(matches!(removed.criteria, AutoFilterCriteria::Top10 { .. }));
    let after_remove = reopened.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(after_remove.columns.len(), 2);
}

#[test]
fn auto_filter_column_values_multi_value_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "H").unwrap();
    wb.set_value("Sheet1!A2", "alpha").unwrap();
    wb.set_value("Sheet1!A3", "beta").unwrap();
    wb.set_value("Sheet1!A4", "gamma").unwrap();
    wb.set_auto_filter("Sheet1", "A1:A4").unwrap();

    wb.set_auto_filter_column(
        "Sheet1",
        AutoFilterColumnPatch {
            column_offset: 0,
            hidden_button: None,
            show_button: None,
            criteria: AutoFilterCriteria::Values {
                values: vec!["alpha".into(), "gamma".into()],
                blank: Some(true),
            },
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.auto_filter("Sheet1").unwrap().unwrap();
    match &after.columns[0].criteria {
        AutoFilterCriteria::Values { values, blank } => {
            assert_eq!(values, &vec!["alpha".to_string(), "gamma".to_string()]);
            assert_eq!(*blank, Some(true));
        }
        other => panic!("expected Values, got {other:?}"),
    }
}

#[test]
fn auto_filter_values_applies_hidden_rows() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!A3", "South").unwrap();
    wb.set_value("Sheet1!A4", "East").unwrap();
    wb.set_value("Sheet1!A5", "West").unwrap();
    wb.set_auto_filter("Sheet1", "A1:A5").unwrap();

    wb.set_auto_filter_column(
        "Sheet1",
        AutoFilterColumnPatch {
            column_offset: 0,
            hidden_button: None,
            show_button: None,
            criteria: AutoFilterCriteria::Values {
                values: vec!["North".into(), "West".into()],
                blank: None,
            },
        },
    )
    .unwrap();

    let hidden = collect_hidden(&mut wb);
    assert_eq!(hidden, vec![3, 4]);
    assert!(!row_hidden(&mut wb, 1));

    wb.remove_auto_filter_column("Sheet1", 0).unwrap();
    let hidden = collect_hidden(&mut wb);
    assert!(hidden.is_empty());

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    reopened
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Values {
                    values: vec!["North".into(), "West".into()],
                    blank: None,
                },
            },
        )
        .unwrap();
    assert_eq!(collect_hidden(&mut reopened), vec![3, 4]);
}

#[test]
fn auto_filter_sort_reorders_rows_and_authors_state() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Units").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!B2", 30.0).unwrap();
    wb.set_value("Sheet1!A3", "South").unwrap();
    wb.set_value("Sheet1!B3", 10.0).unwrap();
    wb.set_value("Sheet1!A4", "East").unwrap();
    wb.set_value("Sheet1!B4", 20.0).unwrap();
    wb.set_auto_filter("Sheet1", "A1:B4").unwrap();

    wb.set_auto_filter_sort("Sheet1", 1, true).unwrap();

    assert_eq!(cell_text(&mut wb, "B2"), "30");
    assert_eq!(cell_text(&mut wb, "B3"), "20");
    assert_eq!(cell_text(&mut wb, "B4"), "10");
    assert_eq!(cell_text(&mut wb, "A2"), "North");
    assert_eq!(cell_text(&mut wb, "A3"), "East");
    assert_eq!(cell_text(&mut wb, "A4"), "South");
    assert_eq!(cell_text(&mut wb, "A1"), "Region");

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert!(sort_state_present(&mut reopened));
    assert_eq!(cell_text(&mut reopened, "B2"), "30");

    reopened.set_auto_filter_sort("Sheet1", 1, false).unwrap();
    assert_eq!(cell_text(&mut reopened, "B2"), "10");
    assert_eq!(cell_text(&mut reopened, "B4"), "30");

    reopened.remove_auto_filter_sort("Sheet1").unwrap();
    assert!(!sort_state_present(&mut reopened));
    assert_eq!(cell_text(&mut reopened, "B2"), "10");
}

fn cell_text(wb: &mut Workbook, reference: &str) -> String {
    match wb.get_cell(format!("Sheet1!{reference}")).unwrap().value {
        CellValue::String(s) => s,
        CellValue::Number(n) => {
            if n == n.trunc() {
                format!("{}", n as i64)
            } else {
                format!("{n}")
            }
        }
        CellValue::Blank => String::new(),
        other => format!("{other:?}"),
    }
}

fn sort_state_present(wb: &mut Workbook) -> bool {
    let ws_part = wb.worksheet_part_for_sheet("Sheet1").unwrap();
    let ws = ws_part.root_element(&mut wb.doc).unwrap();
    ws.auto_filter
        .as_ref()
        .map(|af| af.sort_state.is_some())
        .unwrap_or(false)
}

fn row_hidden(wb: &mut Workbook, row: u32) -> bool {
    let ws_part = wb.worksheet_part_for_sheet("Sheet1").unwrap();
    let ws = ws_part.root_element(&mut wb.doc).unwrap();
    ws.sheet_data
        .row
        .iter()
        .find(|r| r.row_index == Some(row))
        .and_then(|r| r.hidden.as_ref().map(|b| bool::from(*b)))
        .unwrap_or(false)
}

fn collect_hidden(wb: &mut Workbook) -> Vec<u32> {
    let ws_part = wb.worksheet_part_for_sheet("Sheet1").unwrap();
    let ws = ws_part.root_element(&mut wb.doc).unwrap();
    let mut out: Vec<u32> = ws
        .sheet_data
        .row
        .iter()
        .filter(|r| r.hidden.as_ref().map(|b| bool::from(*b)).unwrap_or(false))
        .filter_map(|r| r.row_index)
        .collect();
    out.sort_unstable();
    out
}

#[test]
fn auto_filter_column_validation_errors() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "H").unwrap();

    let err = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Top10 {
                    top: Some(true),
                    percent: Some(false),
                    val: 5.0,
                },
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidAutoFilter);

    wb.set_auto_filter("Sheet1", "A1:B5").unwrap();

    let err = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 5,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Top10 {
                    top: Some(true),
                    percent: Some(false),
                    val: 5.0,
                },
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidAutoFilter);

    let err = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Values {
                    values: vec!["".into()],
                    blank: Some(false),
                },
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidAutoFilter);

    let err = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Top10 {
                    top: Some(true),
                    percent: Some(false),
                    val: 0.0,
                },
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidAutoFilter);
}
