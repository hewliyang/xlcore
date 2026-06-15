use crate::refs::{parse_cell_reference, parse_range_reference, ParsedCellRef, ParsedRangeRef};
use crate::*;

#[test]
fn parses_cell_references() {
    assert_eq!(
        parse_cell_reference("'Q1 Inputs'!$B$12").unwrap(),
        ParsedCellRef {
            sheet: Some("Q1 Inputs".to_string()),
            row: 12,
            column: 2,
        }
    );
    assert_eq!(
        parse_cell_reference("AA10").unwrap(),
        ParsedCellRef {
            sheet: None,
            row: 10,
            column: 27,
        }
    );
}

#[test]
fn creates_sets_recalculates_saves_and_reopens() {
    let mut workbook = Workbook::new().unwrap();
    assert_eq!(workbook.sheets().unwrap()[0].name, "Sheet1");

    workbook.set_value("Sheet1!A1", "Units").unwrap();
    workbook.set_value("Sheet1!B1", 10.0).unwrap();
    workbook.set_formula("Sheet1!C1", "=B1*2").unwrap();

    let recalc = workbook.recalculate(false).unwrap();
    assert_eq!(
        recalc.cell("Sheet1", "C1").unwrap().value,
        xlcore_engine::CellValue::Number(20.0)
    );

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(
        reopened.get_cell("Sheet1!A1").unwrap().value,
        CellValue::String("Units".to_string())
    );
    assert_eq!(
        reopened.get_cell("Sheet1!C1").unwrap().value,
        CellValue::Number(20.0)
    );
    assert_eq!(
        reopened.get_cell("Sheet1!C1").unwrap().formula.as_deref(),
        Some("B1*2")
    );
}

#[test]
fn save_without_explicit_recalculate_writes_cached_values() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!B1", 7.0).unwrap();
    workbook.set_formula("Sheet1!C1", "=B1*6").unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(
        reopened.get_cell("Sheet1!C1").unwrap().value,
        CellValue::Number(42.0)
    );
}

#[test]
fn engine_produced_errors_populate_fallback() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    workbook
        .set_formula("Sheet1!B1", "=OFFSET(A1,-5,0)")
        .unwrap();
    workbook.set_formula("Sheet1!C1", "=1/0").unwrap();
    workbook.set_formula("Sheet1!D1", "=A1+\"x\"").unwrap();

    let recalc = workbook.recalculate(false).unwrap();
    let b1 = recalc.cell("Sheet1", "B1").unwrap();
    assert_eq!(
        b1.value,
        xlcore_engine::CellValue::Error("#REF!".to_string()),
        "B1 value: {:?}",
        b1
    );
    assert_eq!(b1.fallback, None, "B1 fallback: {:?}", b1);
    let c1 = recalc.cell("Sheet1", "C1").unwrap();
    assert_eq!(
        c1.value,
        xlcore_engine::CellValue::Error("#DIV/0!".to_string()),
        "C1 value: {:?}",
        c1
    );
    assert_eq!(c1.fallback, None, "C1 fallback: {:?}", c1);
    let d1 = recalc.cell("Sheet1", "D1").unwrap();
    assert_eq!(
        d1.value,
        xlcore_engine::CellValue::Error("#VALUE!".to_string()),
        "D1 value: {:?}",
        d1
    );
    assert_eq!(d1.fallback, None, "D1 fallback: {:?}", d1);
}

#[test]
fn creates_and_renames_sheets() {
    let mut workbook = Workbook::new().unwrap();
    workbook.create_sheet("Scenario").unwrap();
    workbook.rename_sheet("Scenario", "Inputs").unwrap();
    workbook.set_value("Inputs!A1", "ok").unwrap();

    let sheets = workbook.sheets().unwrap();
    assert_eq!(
        sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Sheet1", "Inputs"]
    );

    workbook.delete_sheet("Sheet1").unwrap();
    assert_eq!(workbook.sheets().unwrap()[0].name, "Inputs");
    assert_eq!(
        workbook.get_cell("Inputs!A1").unwrap().value,
        CellValue::String("ok".to_string())
    );
}

#[test]
fn rename_sheet_rewrites_cross_sheet_formula_refs() {
    let mut workbook = Workbook::new().unwrap();
    workbook.create_sheet("Data").unwrap();
    workbook.create_sheet("Other Sheet").unwrap();
    workbook.set_value("Data!A1", 10.0).unwrap();
    workbook.set_value("Data!A2", 15.0).unwrap();
    workbook.set_value("Other Sheet!B1", 7.0).unwrap();
    workbook
        .set_formula("Sheet1!A1", "=Data!A1+Data!A2")
        .unwrap();
    workbook
        .set_formula("Sheet1!A2", "=SUM(Data!A1:A2)")
        .unwrap();
    workbook
        .set_formula("Sheet1!A3", "='Other Sheet'!B1*2")
        .unwrap();
    workbook
        .set_defined_name(crate::DefinedNamePatch {
            name: "Total".to_string(),
            reference: "Data!$A$1:$A$2".to_string(),
            scope: None,
            comment: None,
            hidden: None,
        })
        .unwrap();

    workbook.rename_sheet("Data", "Inputs").unwrap();
    workbook.rename_sheet("Other Sheet", "Refs").unwrap();

    let recalc = workbook.recalculate(false).unwrap();
    assert_eq!(
        recalc.cell("Sheet1", "A1").unwrap().value,
        xlcore_engine::CellValue::Number(25.0)
    );
    assert_eq!(
        recalc.cell("Sheet1", "A2").unwrap().value,
        xlcore_engine::CellValue::Number(25.0)
    );
    assert_eq!(
        recalc.cell("Sheet1", "A3").unwrap().value,
        xlcore_engine::CellValue::Number(14.0)
    );
    let dn = workbook
        .defined_names()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "Total")
        .unwrap();
    assert_eq!(dn.reference, "Inputs!$A$1:$A$2");
}

#[test]
fn move_visibility_and_active_sheet_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook.create_sheet("Inputs").unwrap();
    workbook.create_sheet("Outputs").unwrap();

    let names = |w: &mut Workbook| {
        w.sheets()
            .unwrap()
            .into_iter()
            .map(|s| s.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(names(&mut workbook), ["Sheet1", "Inputs", "Outputs"]);

    workbook.set_active_sheet("Outputs").unwrap();
    let outputs = workbook.move_sheet("Outputs", 0).unwrap();
    assert_eq!(outputs.index, 0);
    assert!(outputs.active);
    assert_eq!(names(&mut workbook), ["Outputs", "Sheet1", "Inputs"]);

    let hidden = workbook
        .set_sheet_visibility("Sheet1", SheetVisibility::Hidden)
        .unwrap();
    assert_eq!(hidden.state.as_deref(), Some("hidden"));
    let very = workbook
        .set_sheet_visibility("Inputs", SheetVisibility::VeryHidden)
        .unwrap();
    assert_eq!(very.state.as_deref(), Some("veryHidden"));

    let err = workbook
        .set_sheet_visibility("Outputs", SheetVisibility::Hidden)
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::Other);

    let err = workbook.set_active_sheet("Sheet1").unwrap_err();
    assert_eq!(err.code, ApiErrorCode::Other);

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let sheets = reopened.sheets().unwrap();
    assert_eq!(
        sheets.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Outputs", "Sheet1", "Inputs"]
    );
    assert!(sheets[0].active);
    assert_eq!(sheets[1].state.as_deref(), Some("hidden"));
    assert_eq!(sheets[2].state.as_deref(), Some("veryHidden"));

    reopened
        .set_sheet_visibility("Sheet1", SheetVisibility::Visible)
        .unwrap();
    reopened.set_active_sheet("Sheet1").unwrap();
    let after = reopened.sheets().unwrap();
    assert!(after.iter().find(|s| s.name == "Sheet1").unwrap().active);
}

#[test]
fn move_sheet_clamps_to_range_and_missing_errors() {
    let mut workbook = Workbook::new().unwrap();
    workbook.create_sheet("B").unwrap();
    workbook.create_sheet("C").unwrap();
    let moved = workbook.move_sheet("Sheet1", 99).unwrap();
    assert_eq!(moved.index, 2);
    let err = workbook.move_sheet("Missing", 0).unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}

#[test]
fn parses_range_references() {
    let plain = parse_range_reference("A1:B3").unwrap();
    assert_eq!(
        plain,
        ParsedRangeRef {
            sheet: None,
            start_row: 1,
            start_column: 1,
            end_row: 3,
            end_column: 2,
        }
    );
    let qualified = parse_range_reference("'Q1 Inputs'!$B$2:$C$4").unwrap();
    assert_eq!(
        qualified,
        ParsedRangeRef {
            sheet: Some("Q1 Inputs".to_string()),
            start_row: 2,
            start_column: 2,
            end_row: 4,
            end_column: 3,
        }
    );
    let single = parse_range_reference("Sheet1!C5").unwrap();
    assert_eq!(single.start_row, 5);
    assert_eq!(single.end_row, 5);
    assert_eq!(single.start_column, 3);
    assert_eq!(single.end_column, 3);
    let reversed = parse_range_reference("B3:A1").unwrap();
    assert_eq!(reversed.start_row, 1);
    assert_eq!(reversed.end_row, 3);
    assert_eq!(reversed.start_column, 1);
    assert_eq!(reversed.end_column, 2);

    assert!(parse_range_reference("").is_err());
    assert!(parse_range_reference("A1:").is_err());
    assert!(parse_range_reference(":B2").is_err());
    assert!(parse_range_reference("NOT_A_REF").is_err());
}
