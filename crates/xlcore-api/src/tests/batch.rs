use crate::*;

#[test]
fn batch_outcome_collects_warnings_on_success() {
    let mut workbook = Workbook::new().unwrap();
    let outcome = workbook.batch(|tx| {
        tx.set_value("Sheet1!A1", 1.0)?;
        tx.push_warning(
            ApiWarning::new(ApiErrorCode::LossyOperation, "normalized something")
                .with_sheet("Sheet1")
                .with_ref("A1"),
        );
        Ok(42_u32)
    });
    assert!(outcome.is_ok());
    assert_eq!(outcome.value, Some(42));
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(outcome.warnings[0].code, ApiErrorCode::LossyOperation);
    assert_eq!(outcome.warnings[0].sheet.as_deref(), Some("Sheet1"));
    assert!(workbook.warnings().is_empty());
}

#[test]
fn batch_outcome_reports_error_with_prior_warnings() {
    let mut workbook = Workbook::new().unwrap();
    let outcome = workbook.batch(|tx| {
        tx.push_warning(ApiWarning::new(ApiErrorCode::LossyOperation, "first"));
        tx.set_value("Bogus!A1", 1.0)?;
        Ok(())
    });
    assert!(!outcome.is_ok());
    let err = outcome.error.expect("error captured");
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(outcome.warnings[0].code, ApiErrorCode::LossyOperation);
    assert!(outcome.value.is_none());
}

#[test]
fn warnings_outside_batch_are_drainable() {
    let mut workbook = Workbook::new().unwrap();
    workbook.push_warning(ApiWarning::new(ApiErrorCode::LossyOperation, "ambient"));
    assert_eq!(workbook.warnings().len(), 1);
    let drained = workbook.take_warnings();
    assert_eq!(drained.len(), 1);
    assert!(workbook.warnings().is_empty());
}

#[test]
fn batch_restores_prior_warnings_buffer() {
    let mut workbook = Workbook::new().unwrap();
    workbook.push_warning(ApiWarning::new(ApiErrorCode::LossyOperation, "outer"));
    let outcome = workbook.batch(|tx| {
        tx.push_warning(ApiWarning::new(ApiErrorCode::LossyOperation, "inner"));
        Ok(())
    });
    assert_eq!(outcome.warnings.len(), 1);
    assert_eq!(outcome.warnings[0].message, "inner");
    assert_eq!(workbook.warnings().len(), 1);
    assert_eq!(workbook.warnings()[0].message, "outer");
}
