use crate::*;

#[test]
fn properties_default_blank_workbook_has_no_core_part() {
    let mut wb = Workbook::new().unwrap();
    let props = wb.properties().unwrap();
    assert_eq!(props, WorkbookProperties::default());
}

#[test]
fn properties_set_and_round_trip_through_save() {
    let mut wb = Workbook::new().unwrap();
    let returned = wb
        .set_properties(WorkbookPropertiesPatch {
            title: Some("Quarterly Plan".to_string()),
            creator: Some("Agent".to_string()),
            keywords: Some("finance,plan".to_string()),
            description: Some("Q1 outputs".to_string()),
            last_modified_by: Some("Agent".to_string()),
            category: Some("Reports".to_string()),
            created: Some("2024-01-01T00:00:00Z".to_string()),
            modified: Some("2024-02-15T12:30:00Z".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(returned.title.as_deref(), Some("Quarterly Plan"));
    assert_eq!(returned.keywords.as_deref(), Some("finance,plan"));
    assert_eq!(returned.created.as_deref(), Some("2024-01-01T00:00:00Z"));
    assert_eq!(returned.modified.as_deref(), Some("2024-02-15T12:30:00Z"));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.properties().unwrap();
    assert_eq!(after, returned);
}

#[test]
fn properties_partial_patch_preserves_unchanged_fields() {
    let mut wb = Workbook::new().unwrap();
    wb.set_properties(WorkbookPropertiesPatch {
        title: Some("First".to_string()),
        creator: Some("Alice".to_string()),
        ..Default::default()
    })
    .unwrap();
    let after = wb
        .set_properties(WorkbookPropertiesPatch {
            creator: Some("Bob".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(after.title.as_deref(), Some("First"));
    assert_eq!(after.creator.as_deref(), Some("Bob"));
}

#[test]
fn properties_invalid_created_timestamp_diagnosed() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_properties(WorkbookPropertiesPatch {
            created: Some("yesterday".to_string()),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidProperty);
}

#[test]
fn calc_properties_default_blank_workbook() {
    let mut wb = Workbook::new().unwrap();
    let calc = wb.calc_properties().unwrap();
    assert_eq!(calc.calc_mode, Some(CalcMode::Auto));
    assert_eq!(calc.full_calc_on_load, Some(true));
    assert_eq!(calc.force_full_calc, Some(true));
}

#[test]
fn calc_properties_patch_round_trips_through_save() {
    let mut wb = Workbook::new().unwrap();
    let updated = wb
        .set_calc_properties(CalcPropertiesPatch {
            calc_mode: Some(CalcMode::Manual),
            iterate: Some(true),
            iterate_count: Some(50),
            iterate_delta: Some(0.001),
            full_precision: Some(false),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(updated.calc_mode, Some(CalcMode::Manual));
    assert_eq!(updated.iterate, Some(true));
    assert_eq!(updated.iterate_count, Some(50));
    assert_eq!(updated.iterate_delta, Some(0.001));
    assert_eq!(updated.full_precision, Some(false));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.calc_properties().unwrap();
    assert_eq!(after.calc_mode, Some(CalcMode::Manual));
    assert_eq!(after.iterate, Some(true));
    assert_eq!(after.iterate_count, Some(50));
    assert_eq!(after.iterate_delta, Some(0.001));
    assert_eq!(after.full_precision, Some(false));
}
