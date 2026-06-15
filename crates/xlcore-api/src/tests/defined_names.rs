use crate::*;

#[test]
fn defined_names_create_update_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.create_sheet("Inputs").unwrap();

    let info = wb
        .set_defined_name(DefinedNamePatch {
            name: "TaxRate".to_string(),
            reference: "Sheet1!$B$1".to_string(),
            comment: Some("effective rate".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(info.name, "TaxRate");
    assert!(info.scope.is_none());

    wb.set_defined_name(DefinedNamePatch {
        name: "LocalRange".to_string(),
        reference: "$A$1:$B$10".to_string(),
        scope: Some("Inputs".to_string()),
        hidden: Some(true),
        ..Default::default()
    })
    .unwrap();

    let list = wb.defined_names().unwrap();
    assert_eq!(list.len(), 2);
    let local = list.iter().find(|d| d.name == "LocalRange").unwrap();
    assert_eq!(local.scope.as_deref(), Some("Inputs"));
    assert!(local.hidden);

    wb.set_defined_name(DefinedNamePatch {
        name: "TaxRate".to_string(),
        reference: "Sheet1!$C$1".to_string(),
        ..Default::default()
    })
    .unwrap();
    let updated = wb
        .defined_names()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "TaxRate")
        .unwrap();
    assert_eq!(updated.reference, "Sheet1!$C$1");
    assert_eq!(updated.comment.as_deref(), None);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.defined_names().unwrap();
    assert_eq!(after.len(), 2);

    let removed = reopened
        .remove_defined_name("LocalRange", Some("Inputs"))
        .unwrap()
        .unwrap();
    assert_eq!(removed.name, "LocalRange");
    assert_eq!(reopened.defined_names().unwrap().len(), 1);

    assert!(reopened
        .remove_defined_name("TaxRate", Some("Inputs"))
        .unwrap()
        .is_none());
    assert!(reopened
        .remove_defined_name("TaxRate", None)
        .unwrap()
        .is_some());
    assert!(reopened.defined_names().unwrap().is_empty());
}

#[test]
fn defined_name_patch_accepts_legacy_formula_alias() {
    let patch: DefinedNamePatch =
        serde_json::from_str(r#"{"name":"Legacy","formula":"Sheet1!$A$1"}"#).unwrap();
    assert_eq!(patch.reference, "Sheet1!$A$1");

    let canonical: DefinedNamePatch =
        serde_json::from_str(r#"{"name":"Modern","reference":"Sheet1!$B$2"}"#).unwrap();
    assert_eq!(canonical.reference, "Sheet1!$B$2");
}

#[test]
fn defined_names_validation_errors() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "".to_string(),
            reference: "Sheet1!$A$1".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidDefinedName);

    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "A1".to_string(),
            reference: "Sheet1!$A$1".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidDefinedName);

    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "has space".to_string(),
            reference: "Sheet1!$A$1".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidDefinedName);

    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "OK".to_string(),
            reference: "   ".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidDefinedName);

    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "Scoped".to_string(),
            reference: "$A$1".to_string(),
            scope: Some("Ghost".to_string()),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}

#[test]
fn traces_formula_precedents_and_dependents() {
    let mut wb = Workbook::new().unwrap();
    wb.create_sheet("Inputs").unwrap();
    wb.set_value("Sheet1!A1", 5.0).unwrap();
    wb.set_value("Inputs!A1", 10.0).unwrap();
    wb.set_value("Inputs!A2", 15.0).unwrap();
    wb.set_formula("Sheet1!B2", "=SUM(Inputs!A1:A2)+A1")
        .unwrap();
    wb.set_formula("Sheet1!C2", "=B2*2").unwrap();

    let precedents = wb.precedents("Sheet1!B2").unwrap();
    assert_eq!(
        precedents
            .iter()
            .map(|item| (item.sheet.as_str(), item.reference.as_str()))
            .collect::<Vec<_>>(),
        [("Sheet1", "A1"), ("Inputs", "A1:A2")]
    );

    let dependents = wb.dependents("Inputs!A2").unwrap();
    assert_eq!(
        dependents
            .iter()
            .map(|item| (item.sheet.as_str(), item.reference.as_str()))
            .collect::<Vec<_>>(),
        [("Sheet1", "B2")]
    );

    let info = wb.dependencies("Sheet1!B2").unwrap();
    assert_eq!(info.reference, "B2");
    assert_eq!(info.precedents.len(), 2);
    assert_eq!(info.dependents[0].reference, "C2");
}

#[test]
fn traces_defined_name_dependencies() {
    let mut wb = Workbook::new().unwrap();
    wb.create_sheet("Inputs").unwrap();
    wb.set_value("Inputs!B1", 0.08).unwrap();
    wb.set_defined_name(DefinedNamePatch {
        name: "TaxRate".to_string(),
        reference: "Inputs!$B$1".to_string(),
        ..Default::default()
    })
    .unwrap();
    wb.set_formula("Sheet1!A1", "=TaxRate*100").unwrap();

    let precedents = wb.precedents("Sheet1!A1").unwrap();
    assert_eq!(precedents[0].sheet, "Inputs");
    assert_eq!(precedents[0].reference, "B1");

    let dependents = wb.dependents("Inputs!B1").unwrap();
    assert_eq!(dependents[0].sheet, "Sheet1");
    assert_eq!(dependents[0].reference, "A1");
}

#[test]
fn defined_names_resolve_in_recalculate() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", 10.0).unwrap();
    wb.set_value("Sheet1!A2", 20.0).unwrap();
    wb.set_value("Sheet1!A3", 30.0).unwrap();
    wb.set_value("Sheet1!D1", 5.0).unwrap();

    wb.set_defined_name(DefinedNamePatch {
        name: "MyRange".to_string(),
        reference: "Sheet1!$A$1:$A$3".to_string(),
        ..Default::default()
    })
    .unwrap();
    wb.set_defined_name(DefinedNamePatch {
        name: "Pivot".to_string(),
        reference: "Sheet1!$D$1".to_string(),
        ..Default::default()
    })
    .unwrap();

    wb.set_formula("Sheet1!B1", "=SUM(MyRange)").unwrap();
    wb.set_formula("Sheet1!B2", "=SUM(MyRange)+Pivot").unwrap();

    let recalc = wb.recalculate(false).unwrap();
    let b1 = recalc.cell("Sheet1", "B1").unwrap();
    assert!(b1.fallback.is_none(), "B1 fallback: {:?}", b1.fallback);
    assert_eq!(b1.value, xlcore_engine::CellValue::Number(60.0));
    let b2 = recalc.cell("Sheet1", "B2").unwrap();
    assert!(b2.fallback.is_none(), "B2 fallback: {:?}", b2.fallback);
    assert_eq!(b2.value, xlcore_engine::CellValue::Number(65.0));
}

#[test]
fn defined_names_non_reference_formula_emits_warning() {
    let mut wb = Workbook::new().unwrap();
    wb.take_warnings();
    wb.set_defined_name(DefinedNamePatch {
        name: "TaxRate".to_string(),
        reference: "0.1".to_string(),
        ..Default::default()
    })
    .unwrap();
    let warnings = wb.take_warnings();
    assert_eq!(warnings.len(), 1, "warnings: {:?}", warnings);
    assert!(
        warnings[0].message.contains("TaxRate"),
        "msg: {}",
        warnings[0].message
    );

    wb.set_defined_name(DefinedNamePatch {
        name: "Pivot".to_string(),
        reference: "Sheet1!$A$1".to_string(),
        ..Default::default()
    })
    .unwrap();
    assert!(wb.take_warnings().is_empty());
}
