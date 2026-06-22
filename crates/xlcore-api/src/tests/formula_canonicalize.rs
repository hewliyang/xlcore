use crate::*;

fn formula_xml(wb: &mut Workbook, sheet: &str, reference: &str) -> Option<String> {
    wb.get_cell_in(sheet, reference).unwrap().formula
}

#[test]
fn set_formula_canonicalizes_modern_functions_with_xlfn() {
    let mut wb = Workbook::new().unwrap();
    let sheet = wb.sheets().unwrap()[0].name.clone();

    wb.set_formula_in(&sheet, "A1", "=MAXIFS(B1:B3,C1:C3,1)")
        .unwrap();
    wb.set_formula_in(&sheet, "A2", "=TEXTJOIN(\",\",TRUE,B1:B3)")
        .unwrap();
    wb.set_formula_in(&sheet, "A3", "=SUM(B1:B3)").unwrap();

    assert_eq!(
        formula_xml(&mut wb, &sheet, "A1").as_deref(),
        Some("_xlfn.MAXIFS(B1:B3,C1:C3,1)")
    );
    assert_eq!(
        formula_xml(&mut wb, &sheet, "A2").as_deref(),
        Some("_xlfn.TEXTJOIN(\",\",TRUE,B1:B3)")
    );
    assert_eq!(
        formula_xml(&mut wb, &sheet, "A3").as_deref(),
        Some("SUM(B1:B3)")
    );
}

#[test]
fn set_formula_decorates_let_binding_names_with_xlpm() {
    let mut wb = Workbook::new().unwrap();
    let sheet = wb.sheets().unwrap()[0].name.clone();

    wb.set_formula_in(&sheet, "A1", "=LET(pay,B1,cohort,B2,pay*cohort)")
        .unwrap();
    wb.set_formula_in(&sheet, "A2", "=LAMBDA(x,y,x+y)(1,2)")
        .unwrap();

    assert_eq!(
        formula_xml(&mut wb, &sheet, "A1").as_deref(),
        Some("_xlfn.LET(_xlpm.pay,B1,_xlpm.cohort,B2,_xlpm.pay*_xlpm.cohort)")
    );
    assert_eq!(
        formula_xml(&mut wb, &sheet, "A2").as_deref(),
        Some("_xlfn.LAMBDA(_xlpm.x,_xlpm.y,_xlpm.x+_xlpm.y)(1,2)")
    );

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(
        formula_xml(&mut reopened, &sheet, "A1").as_deref(),
        Some("_xlfn.LET(_xlpm.pay,B1,_xlpm.cohort,B2,_xlpm.pay*_xlpm.cohort)")
    );
}

#[test]
fn set_formula_canonicalization_is_idempotent() {
    let mut wb = Workbook::new().unwrap();
    let sheet = wb.sheets().unwrap()[0].name.clone();

    let first = wb
        .set_formula_in(&sheet, "A1", "=MAXIFS(B1:B3,C1:C3,1)")
        .unwrap()
        .formula
        .unwrap();
    let second = wb
        .set_formula_in(&sheet, "A1", &format!("={first}"))
        .unwrap()
        .formula
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(first, "_xlfn.MAXIFS(B1:B3,C1:C3,1)");
}

#[test]
fn set_defined_name_canonicalizes_modern_functions_but_leaves_references() {
    let mut wb = Workbook::new().unwrap();

    wb.set_defined_name(DefinedNamePatch {
        name: "PlainRef".to_string(),
        reference: "Sheet1!$A$1:$A$10".to_string(),
        ..Default::default()
    })
    .unwrap();
    wb.set_defined_name(DefinedNamePatch {
        name: "MaxTier1".to_string(),
        reference: "=MAXIFS(Sheet1!B:B,Sheet1!C:C,1)".to_string(),
        ..Default::default()
    })
    .unwrap();

    let list = wb.defined_names().unwrap();
    let plain = list.iter().find(|d| d.name == "PlainRef").unwrap();
    assert_eq!(plain.reference, "Sheet1!$A$1:$A$10");
    let expr = list.iter().find(|d| d.name == "MaxTier1").unwrap();
    assert_eq!(expr.reference, "_xlfn.MAXIFS(Sheet1!B:B,Sheet1!C:C,1)");
}

#[test]
fn set_range_formulas_canonicalizes_modern_functions() {
    let mut wb = Workbook::new().unwrap();
    let sheet = wb.sheets().unwrap()[0].name.clone();

    wb.set_range_formulas_in(
        &sheet,
        "A1:A2",
        vec![
            vec![Some("=XLOOKUP(1,B:B,C:C)".to_string())],
            vec![Some("=SUM(B1:B5)".to_string())],
        ],
    )
    .unwrap();

    assert_eq!(
        formula_xml(&mut wb, &sheet, "A1").as_deref(),
        Some("_xlfn.XLOOKUP(1,B:B,C:C)")
    );
    assert_eq!(
        formula_xml(&mut wb, &sheet, "A2").as_deref(),
        Some("SUM(B1:B5)")
    );
}

#[test]
fn let_canonicalization_is_idempotent() {
    let mut wb = Workbook::new().unwrap();
    let sheet = wb.sheets().unwrap()[0].name.clone();

    let first = wb
        .set_formula_in(&sheet, "A1", "=LET(pay,B1,cohort,B2,pay*cohort)")
        .unwrap()
        .formula
        .unwrap();
    let second = wb
        .set_formula_in(&sheet, "A1", &format!("={first}"))
        .unwrap()
        .formula
        .unwrap();
    assert_eq!(
        first, second,
        "re-canonicalizing a decorated LET must be stable"
    );
}
