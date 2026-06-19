use crate::*;

#[test]
fn routed_edit_recalcs_dependents_and_reuses_engine() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 10.0).unwrap();
    workbook.set_value("Sheet1!B1", 20.0).unwrap();
    workbook.set_formula("Sheet1!C1", "=A1+B1").unwrap();

    let first = workbook.recalculate(false).unwrap();
    assert_eq!(
        first.cell("Sheet1", "C1").map(|cell| &cell.value),
        Some(&xlcore_types::EngineCellValue::Number(30.0))
    );
    assert!(workbook.engine.is_some());

    workbook.set_value("Sheet1!A1", 100.0).unwrap();
    assert!(
        workbook.engine.is_some(),
        "value edit must stay routed into the resident engine, not invalidate it"
    );

    let second = workbook.recalculate(false).unwrap();
    assert_eq!(
        second.cell("Sheet1", "C1").map(|cell| &cell.value),
        Some(&xlcore_types::EngineCellValue::Number(120.0))
    );
}

#[test]
fn structural_mutation_invalidates_engine() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    workbook.set_formula("Sheet1!B1", "=A1+1").unwrap();
    workbook.recalculate(false).unwrap();
    assert!(workbook.engine.is_some());

    workbook.insert_rows("Sheet1", 1, 1).unwrap();
    assert!(workbook.engine.is_none());

    let report = workbook.recalculate(false).unwrap();
    assert_eq!(
        report.cell("Sheet1", "B2").map(|cell| &cell.value),
        Some(&xlcore_types::EngineCellValue::Number(2.0))
    );
}
