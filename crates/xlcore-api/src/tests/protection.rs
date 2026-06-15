use crate::*;

#[test]
fn sheet_protection_set_read_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    assert!(wb.sheet_protection("Sheet1").unwrap().is_none());

    let info = wb
        .set_sheet_protection(
            "Sheet1",
            SheetProtectionPatch {
                enabled: Some(true),
                password: Some("CAFE".to_string()),
                format_cells: Some(true),
                insert_rows: Some(true),
                select_locked_cells: Some(false),
                sort: Some(true),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(info.enabled);
    assert_eq!(info.password.as_deref(), Some("CAFE"));
    assert_eq!(info.format_cells, Some(true));
    assert_eq!(info.insert_rows, Some(true));
    assert_eq!(info.select_locked_cells, Some(false));
    assert_eq!(info.sort, Some(true));

    let updated = wb
        .set_sheet_protection(
            "Sheet1",
            SheetProtectionPatch {
                sort: Some(false),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.format_cells, Some(true));
    assert_eq!(updated.sort, Some(false));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.sheet_protection("Sheet1").unwrap().unwrap();
    assert!(after.enabled);
    assert_eq!(after.password.as_deref(), Some("CAFE"));
    assert_eq!(after.sort, Some(false));

    let removed = reopened.remove_sheet_protection("Sheet1").unwrap().unwrap();
    assert!(removed.enabled);
    assert!(reopened.sheet_protection("Sheet1").unwrap().is_none());
    assert!(reopened
        .remove_sheet_protection("Sheet1")
        .unwrap()
        .is_none());

    let err = reopened
        .set_sheet_protection(
            "Sheet1",
            SheetProtectionPatch {
                password: Some("nothex".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidProtection);

    let err = reopened
        .set_sheet_protection("Ghost", SheetProtectionPatch::default())
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}

#[test]
fn workbook_protection_set_read_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    assert!(wb.workbook_protection().unwrap().is_none());

    let info = wb
        .set_workbook_protection(WorkbookProtectionPatch {
            lock_structure: Some(true),
            lock_windows: Some(false),
            workbook_password: Some("ABCD".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(info.lock_structure, Some(true));
    assert_eq!(info.lock_windows, Some(false));
    assert_eq!(info.workbook_password.as_deref(), Some("ABCD"));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.workbook_protection().unwrap().unwrap();
    assert_eq!(after.lock_structure, Some(true));
    assert_eq!(after.workbook_password.as_deref(), Some("ABCD"));

    let removed = reopened.remove_workbook_protection().unwrap().unwrap();
    assert_eq!(removed.lock_structure, Some(true));
    assert!(reopened.workbook_protection().unwrap().is_none());

    let err = reopened
        .set_workbook_protection(WorkbookProtectionPatch {
            workbook_password: Some("zzz".to_string()),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidProtection);
}
