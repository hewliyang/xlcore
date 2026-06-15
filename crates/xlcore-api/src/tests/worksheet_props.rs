use crate::*;

#[test]
fn sheet_properties_set_read_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    let initial = wb.sheet_properties("Sheet1").unwrap();
    assert_eq!(initial.sheet, "Sheet1");
    assert!(initial.tab_color.is_none());
    assert!(initial.zoom.is_none());

    let info = wb
        .set_sheet_properties(
            "Sheet1",
            SheetPropertiesPatch {
                tab_color: Some("FF0000".to_string()),
                zoom: Some(150),
                show_zeros: Some(false),
                right_to_left: Some(true),
                default_row_height: Some(21.0),
                default_col_width: Some(12.5),
            },
        )
        .unwrap();
    assert_eq!(info.tab_color.as_deref(), Some("FFFF0000"));
    assert_eq!(info.zoom, Some(150));
    assert_eq!(info.show_zeros, Some(false));
    assert_eq!(info.right_to_left, Some(true));
    assert_eq!(info.default_row_height, Some(21.0));
    assert_eq!(info.default_col_width, Some(12.5));

    let updated = wb
        .set_sheet_properties(
            "Sheet1",
            SheetPropertiesPatch {
                zoom: Some(75),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(updated.zoom, Some(75));
    assert_eq!(updated.tab_color.as_deref(), Some("FFFF0000"));
    assert_eq!(updated.right_to_left, Some(true));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.sheet_properties("Sheet1").unwrap();
    assert_eq!(after.tab_color.as_deref(), Some("FFFF0000"));
    assert_eq!(after.zoom, Some(75));
    assert_eq!(after.show_zeros, Some(false));
    assert_eq!(after.right_to_left, Some(true));
    assert_eq!(after.default_row_height, Some(21.0));
    assert_eq!(after.default_col_width, Some(12.5));
}

#[test]
fn sheet_properties_validates_zoom_and_dimensions() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_sheet_properties(
            "Sheet1",
            SheetPropertiesPatch {
                zoom: Some(5),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::Other);

    let err = wb
        .set_sheet_properties(
            "Sheet1",
            SheetPropertiesPatch {
                default_row_height: Some(-1.0),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::Other);
}
