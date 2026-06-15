use crate::*;

#[test]
fn tables_create_resize_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Units").unwrap();
    wb.set_value("Sheet1!C1", "Price").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!C2", 1.5).unwrap();
    wb.set_value("Sheet1!A3", "South").unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();
    wb.set_value("Sheet1!C3", 2.5).unwrap();

    let info = wb
        .set_table(
            "Sheet1",
            TablePatch {
                name: "Sales".to_string(),
                reference: Some("Sheet1!A1:C3".to_string()),
                style: Some(TableStylePatch {
                    name: Some("TableStyleMedium2".to_string()),
                    show_row_stripes: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(info.name, "Sales");
    assert_eq!(info.display_name, "Sales");
    assert_eq!(info.reference, "A1:C3");
    assert_eq!(info.header_row_count, 1);
    assert_eq!(info.totals_row_count, 0);
    assert!(info.has_auto_filter);
    assert_eq!(info.columns.len(), 3);
    assert_eq!(info.columns[0].name, "Region");
    assert_eq!(info.columns[1].name, "Units");
    assert_eq!(info.columns[2].name, "Price");
    let style = info.style.as_ref().unwrap();
    assert_eq!(style.name.as_deref(), Some("TableStyleMedium2"));
    assert!(style.show_row_stripes);

    let resized = wb
        .set_table(
            "Sheet1",
            TablePatch {
                name: "Sales".to_string(),
                reference: Some("Sheet1!A1:C5".to_string()),
                totals_row_count: Some(1),
                columns: Some(vec![
                    TableColumnPatch {
                        name: Some("Region".to_string()),
                        totals_label: Some("Total".to_string()),
                        ..Default::default()
                    },
                    TableColumnPatch {
                        name: Some("Units".to_string()),
                        totals_function: Some(TableTotalsFunction::Sum),
                        ..Default::default()
                    },
                    TableColumnPatch {
                        name: Some("Price".to_string()),
                        totals_function: Some(TableTotalsFunction::Average),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(resized.reference, "A1:C5");
    assert_eq!(resized.totals_row_count, 1);
    assert_eq!(resized.columns[1].totals_function, TableTotalsFunction::Sum);
    assert_eq!(
        resized.columns[2].totals_function,
        TableTotalsFunction::Average
    );

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let tables = reopened.tables(None).unwrap();
    assert_eq!(tables.len(), 1);
    let t = &tables[0];
    assert_eq!(t.name, "Sales");
    assert_eq!(t.sheet, "Sheet1");
    assert_eq!(t.reference, "A1:C5");
    assert_eq!(t.totals_row_count, 1);
    assert_eq!(t.columns[0].totals_label.as_deref(), Some("Total"));
    assert_eq!(t.columns[1].totals_function, TableTotalsFunction::Sum);
    assert!(t.has_auto_filter);

    let removed = reopened.remove_table("Sales").unwrap().unwrap();
    assert_eq!(removed.name, "Sales");
    assert!(reopened.tables(None).unwrap().is_empty());

    let bytes2 = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes2).unwrap();
    assert!(reopened2.tables(None).unwrap().is_empty());
}

#[test]
fn tables_reject_overlap_and_invalid_names() {
    let mut wb = Workbook::new().unwrap();
    wb.set_table(
        "Sheet1",
        TablePatch {
            name: "T1".to_string(),
            reference: Some("Sheet1!A1:B5".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    let err = wb
        .set_table(
            "Sheet1",
            TablePatch {
                name: "T2".to_string(),
                reference: Some("Sheet1!B3:D8".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidTable);

    let err = wb
        .set_table(
            "Sheet1",
            TablePatch {
                name: "Bad Name".to_string(),
                reference: Some("Sheet1!E1:F2".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidTable);

    let err = wb
        .set_table(
            "Sheet1",
            TablePatch {
                name: "Other".to_string(),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidTable);
}

#[test]
fn tables_shift_through_structural_edits() {
    let mut wb = Workbook::new().unwrap();
    wb.set_table(
        "Sheet1",
        TablePatch {
            name: "T".to_string(),
            reference: Some("Sheet1!B2:D6".to_string()),
            ..Default::default()
        },
    )
    .unwrap();
    wb.insert_rows("Sheet1", 1, 2).unwrap();
    let t = &wb.tables(None).unwrap()[0];
    assert_eq!(t.reference, "B4:D8");
}
