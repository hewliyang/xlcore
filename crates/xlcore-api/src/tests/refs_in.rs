use crate::*;

const TRICKY: &str = "Q1 'Final' Inputs";

fn workbook_with_tricky_sheet() -> Workbook {
    let mut workbook = Workbook::new().unwrap();
    workbook.create_sheet(TRICKY).unwrap();
    workbook
}

#[test]
fn cell_in_variants_round_trip_quoted_sheet() {
    let mut workbook = workbook_with_tricky_sheet();

    let info = workbook.set_value_in(TRICKY, "B2", 41.0).unwrap();
    assert_eq!(info.sheet, TRICKY);
    assert_eq!(info.reference, "B2");

    workbook.set_formula_in(TRICKY, "C2", "=B2+1").unwrap();
    let cell = workbook.get_cell_in(TRICKY, "C2").unwrap();
    assert_eq!(cell.sheet, TRICKY);
    assert_eq!(cell.formula.as_deref(), Some("B2+1"));

    workbook.clear_in(TRICKY, "B2").unwrap();
    assert_eq!(
        workbook.get_cell_in(TRICKY, "B2").unwrap().value,
        CellValue::Blank
    );
}

#[test]
fn cell_in_honors_embedded_sheet_prefix() {
    let mut workbook = workbook_with_tricky_sheet();
    workbook
        .set_value_in("Sheet1", "'Q1 ''Final'' Inputs'!A1", 7.0)
        .unwrap();
    assert_eq!(
        workbook.get_cell_in("Sheet1", "'Q1 ''Final'' Inputs'!A1").unwrap().value,
        CellValue::Number(7.0)
    );
    assert_eq!(
        workbook.get_cell_in(TRICKY, "A1").unwrap().value,
        CellValue::Number(7.0)
    );
}

#[test]
fn range_in_variants_round_trip_quoted_sheet() {
    let mut workbook = workbook_with_tricky_sheet();
    workbook
        .set_range_values_in(TRICKY, "A1:B1", vec![vec![1.0.into(), 2.0.into()]])
        .unwrap();
    let info = workbook.get_range_in(TRICKY, "A1:B1").unwrap();
    assert_eq!(info.sheet, TRICKY);
    assert_eq!(info.values[0][0], CellValue::Number(1.0));

    workbook
        .set_range_formulas_in(TRICKY, "C1:C1", vec![vec![Some("=A1+B1".to_string())]])
        .unwrap();
    workbook
        .set_style_in(TRICKY, "A1:B1", StylePatch::default())
        .unwrap();
    workbook.clear_range_in(TRICKY, "A1:B1").unwrap();
    assert_eq!(
        workbook.get_range_in(TRICKY, "A1:B1").unwrap().values[0][0],
        CellValue::Blank
    );
}

#[test]
fn copy_and_fill_in_across_quoted_sheets() {
    let mut workbook = workbook_with_tricky_sheet();
    workbook.set_value_in(TRICKY, "A1", 5.0).unwrap();

    let copied = workbook
        .copy_range_in(TRICKY, "A1", "Sheet1", "D4")
        .unwrap();
    assert_eq!(copied.sheet, "Sheet1");
    assert_eq!(copied.reference, "D4:D4");
    assert_eq!(
        workbook.get_cell_in("Sheet1", "D4").unwrap().value,
        CellValue::Number(5.0)
    );

    workbook
        .set_range_values_in(TRICKY, "A1:A1", vec![vec![9.0.into()]])
        .unwrap();
    let filled = workbook.fill_range_in(TRICKY, "A1", TRICKY, "A1:A3").unwrap();
    assert_eq!(filled.sheet, TRICKY);
    assert_eq!(
        workbook.get_cell_in(TRICKY, "A3").unwrap().value,
        CellValue::Number(9.0)
    );
}

#[test]
fn dependencies_in_resolve_quoted_sheet() {
    let mut workbook = workbook_with_tricky_sheet();
    workbook.set_value_in(TRICKY, "A1", 2.0).unwrap();
    workbook.set_formula_in(TRICKY, "B1", "=A1*3").unwrap();

    let precedents = workbook.precedents_in(TRICKY, "B1").unwrap();
    assert_eq!(precedents.len(), 1);
    assert_eq!(precedents[0].sheet, TRICKY);
    assert_eq!(precedents[0].reference, "A1");

    let dependents = workbook.dependents_in(TRICKY, "A1").unwrap();
    assert_eq!(dependents.len(), 1);
    assert_eq!(dependents[0].reference, "B1");

    let info = workbook.dependencies_in(TRICKY, "B1").unwrap();
    assert_eq!(info.sheet, TRICKY);
    assert_eq!(info.precedents.len(), 1);
}
