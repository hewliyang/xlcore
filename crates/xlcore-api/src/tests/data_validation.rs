use crate::*;

#[test]
fn data_validation_add_list_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "pick").unwrap();

    let info = wb
        .set_data_validation(
            "Sheet1!A2:A10",
            DataValidationPatch {
                rule_type: DataValidationType::List,
                formula1: Some("\"red,green,blue\"".to_string()),
                show_input_message: Some(true),
                prompt: Some("Choose a color".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(info.reference, "A2:A10");
    assert_eq!(info.rule_type, DataValidationType::List);

    wb.set_data_validation(
        "Sheet1!B1:B5",
        DataValidationPatch {
            rule_type: DataValidationType::Whole,
            operator: Some(DataValidationOperator::Between),
            formula1: Some("1".to_string()),
            formula2: Some("100".to_string()),
            show_error_message: Some(true),
            error_style: Some(DataValidationErrorStyle::Stop),
            error: Some("1-100 only".to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    let list = wb.data_validations("Sheet1").unwrap();
    assert_eq!(list.len(), 2);

    let missing_f1 = wb
        .set_data_validation(
            "Sheet1!C1",
            DataValidationPatch {
                rule_type: DataValidationType::List,
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(missing_f1.code, ApiErrorCode::InvalidDataValidation);

    let missing_op = wb
        .set_data_validation(
            "Sheet1!C1",
            DataValidationPatch {
                rule_type: DataValidationType::Whole,
                formula1: Some("1".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(missing_op.code, ApiErrorCode::InvalidDataValidation);

    let missing_f2 = wb
        .set_data_validation(
            "Sheet1!C1",
            DataValidationPatch {
                rule_type: DataValidationType::Whole,
                operator: Some(DataValidationOperator::Between),
                formula1: Some("1".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(missing_f2.code, ApiErrorCode::InvalidDataValidation);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.data_validations("Sheet1").unwrap();
    assert_eq!(after.len(), 2);

    let list_rule = after
        .iter()
        .find(|d| d.rule_type == DataValidationType::List)
        .unwrap();
    assert_eq!(list_rule.formula1.as_deref(), Some("\"red,green,blue\""));
    assert_eq!(list_rule.reference, "A2:A10");
    assert_eq!(list_rule.prompt.as_deref(), Some("Choose a color"));
    assert!(list_rule.show_input_message);

    let whole_rule = after
        .iter()
        .find(|d| d.rule_type == DataValidationType::Whole)
        .unwrap();
    assert_eq!(whole_rule.operator, Some(DataValidationOperator::Between));
    assert_eq!(whole_rule.formula1.as_deref(), Some("1"));
    assert_eq!(whole_rule.formula2.as_deref(), Some("100"));
    assert_eq!(whole_rule.error_style, Some(DataValidationErrorStyle::Stop));

    let replaced = reopened
        .set_data_validation(
            "Sheet1!A2:A5",
            DataValidationPatch {
                rule_type: DataValidationType::List,
                formula1: Some("\"alpha,beta\"".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(replaced.reference, "A2:A5");
    let after_replace = reopened.data_validations("Sheet1").unwrap();
    assert_eq!(after_replace.len(), 2);

    let removed = reopened.remove_data_validation("Sheet1!B1:B100").unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].rule_type, DataValidationType::Whole);
    assert_eq!(reopened.data_validations("Sheet1").unwrap().len(), 1);

    let removed_all = reopened.remove_data_validation("Sheet1!A1:Z1000").unwrap();
    assert_eq!(removed_all.len(), 1);
    assert!(reopened.data_validations("Sheet1").unwrap().is_empty());

    let bytes = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes).unwrap();
    assert!(reopened2.data_validations("Sheet1").unwrap().is_empty());
}
