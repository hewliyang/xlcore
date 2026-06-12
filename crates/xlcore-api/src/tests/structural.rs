use crate::*;

#[test]
fn insert_rows_shifts_cells_formulas_and_merges() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", "Header").unwrap();
    workbook.set_value("Sheet1!A2", 1.0).unwrap();
    workbook.set_value("Sheet1!A3", 2.0).unwrap();
    workbook.set_formula("Sheet1!B3", "=SUM(A2:A3)").unwrap();
    workbook.add_merge("Sheet1!C2:D3").unwrap();

    workbook.insert_rows("Sheet1", 2, 2).unwrap();

    assert_eq!(
        workbook.get_cell("Sheet1!A1").unwrap().value,
        CellValue::String("Header".to_string())
    );
    assert_eq!(
        workbook.get_cell("Sheet1!A2").unwrap().value,
        CellValue::Blank
    );
    assert_eq!(
        workbook.get_cell("Sheet1!A4").unwrap().value,
        CellValue::Number(1.0)
    );
    assert_eq!(
        workbook.get_cell("Sheet1!A5").unwrap().value,
        CellValue::Number(2.0)
    );
    assert_eq!(
        workbook.get_cell("Sheet1!B5").unwrap().formula.as_deref(),
        Some("SUM(A4:A5)")
    );
    let merges = workbook.merges("Sheet1").unwrap();
    assert_eq!(merges[0].reference, "C4:D5");

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let recalc = reopened.recalculate(false).unwrap();
    assert_eq!(
        recalc.cell("Sheet1", "B5").unwrap().value,
        xlcore_engine::CellValue::Number(3.0)
    );
}

#[test]
fn delete_rows_collapses_refs_and_drops_cells() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    workbook.set_value("Sheet1!A2", 2.0).unwrap();
    workbook.set_value("Sheet1!A3", 3.0).unwrap();
    workbook.set_value("Sheet1!A4", 4.0).unwrap();
    workbook.set_formula("Sheet1!B1", "=A2+A3").unwrap();
    workbook.set_formula("Sheet1!B4", "=SUM(A1:A4)").unwrap();

    workbook.delete_rows("Sheet1", 2, 2).unwrap();

    assert_eq!(
        workbook.get_cell("Sheet1!A1").unwrap().value,
        CellValue::Number(1.0)
    );
    assert_eq!(
        workbook.get_cell("Sheet1!A2").unwrap().value,
        CellValue::Number(4.0)
    );
    assert_eq!(
        workbook.get_cell("Sheet1!A3").unwrap().value,
        CellValue::Blank
    );
    assert_eq!(
        workbook.get_cell("Sheet1!B1").unwrap().formula.as_deref(),
        Some("#REF!+#REF!")
    );
    assert_eq!(
        workbook.get_cell("Sheet1!B2").unwrap().formula.as_deref(),
        Some("SUM(A1:A2)")
    );
}

#[test]
fn insert_and_delete_columns_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 10.0).unwrap();
    workbook.set_value("Sheet1!B1", 20.0).unwrap();
    workbook.set_value("Sheet1!C1", 30.0).unwrap();
    workbook.set_formula("Sheet1!D1", "=A1+B1+C1").unwrap();
    workbook.set_column_width("Sheet1", 2, 40.0).unwrap();

    workbook.insert_columns("Sheet1", 2, 1).unwrap();
    assert_eq!(
        workbook.get_cell("Sheet1!A1").unwrap().value,
        CellValue::Number(10.0)
    );
    assert_eq!(
        workbook.get_cell("Sheet1!C1").unwrap().value,
        CellValue::Number(20.0)
    );
    assert_eq!(
        workbook.get_cell("Sheet1!E1").unwrap().formula.as_deref(),
        Some("A1+C1+D1")
    );

    workbook.delete_columns("Sheet1", 1, 1).unwrap();
    assert_eq!(
        workbook.get_cell("Sheet1!A1").unwrap().value,
        CellValue::Blank
    );
    assert_eq!(
        workbook.get_cell("Sheet1!B1").unwrap().value,
        CellValue::Number(20.0)
    );
    assert_eq!(
        workbook.get_cell("Sheet1!D1").unwrap().formula.as_deref(),
        Some("#REF!+B1+C1")
    );

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(
        reopened.get_cell("Sheet1!B1").unwrap().value,
        CellValue::Number(20.0)
    );
}

#[test]
fn cross_sheet_formulas_update_when_target_changes() {
    let mut workbook = Workbook::new().unwrap();
    workbook.create_sheet("Data").unwrap();
    workbook.set_value("Data!A1", 5.0).unwrap();
    workbook.set_value("Data!A2", 7.0).unwrap();
    workbook
        .set_formula("Sheet1!A1", "=Data!A1+Data!A2")
        .unwrap();

    workbook.insert_rows("Data", 1, 1).unwrap();
    assert_eq!(
        workbook.get_cell("Sheet1!A1").unwrap().formula.as_deref(),
        Some("Data!A2+Data!A3")
    );
}

#[test]
fn structural_invalid_args_diagnosed() {
    let mut workbook = Workbook::new().unwrap();
    assert_eq!(
        workbook.insert_rows("Sheet1", 0, 1).unwrap_err().code,
        ApiErrorCode::InvalidRef,
    );
    assert_eq!(
        workbook.insert_rows("Sheet1", 1, 0).unwrap_err().code,
        ApiErrorCode::InvalidRef,
    );
    assert_eq!(
        workbook.delete_columns("Ghost", 1, 1).unwrap_err().code,
        ApiErrorCode::MissingSheet,
    );
}

#[test]
fn structural_shifts_defined_names() {
    use crate::errors::sdk_err_to_api;
    use xlcore_io::spreadsheetml as x;

    let mut workbook = Workbook::new().unwrap();
    workbook.create_sheet("Data").unwrap();
    {
        let wb_part = workbook
            .doc
            .workbook_part()
            .map_err(sdk_err_to_api)
            .unwrap()
            .clone();
        let wb = wb_part
            .root_element_mut(&mut workbook.doc)
            .map_err(sdk_err_to_api)
            .unwrap();
        wb.defined_names = Some(x::DefinedNames {
            defined_name: vec![
                x::DefinedName {
                    name: "Global".to_string(),
                    xml_content: Some("Sheet1!$A$1:$B$10".to_string()),
                    ..Default::default()
                },
                x::DefinedName {
                    name: "LocalScoped".to_string(),
                    local_sheet_id: Some(0),
                    xml_content: Some("$A$1:$B$10".to_string()),
                    ..Default::default()
                },
                x::DefinedName {
                    name: "OtherSheet".to_string(),
                    xml_content: Some("Data!$C$5".to_string()),
                    ..Default::default()
                },
            ],
        });
    }

    workbook.insert_rows("Sheet1", 2, 3).unwrap();

    let wb_part = workbook
        .doc
        .workbook_part()
        .map_err(sdk_err_to_api)
        .unwrap()
        .clone();
    let wb = wb_part
        .root_element(&mut workbook.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    let names = &wb.defined_names.as_ref().unwrap().defined_name;
    assert_eq!(names[0].xml_content.as_deref(), Some("Sheet1!$A$1:$B$13"));
    assert_eq!(names[1].xml_content.as_deref(), Some("$A$1:$B$13"));
    assert_eq!(names[2].xml_content.as_deref(), Some("Data!$C$5"));

    workbook.delete_rows("Sheet1", 1, 1000).unwrap();
    let wb_part = workbook
        .doc
        .workbook_part()
        .map_err(sdk_err_to_api)
        .unwrap()
        .clone();
    let wb = wb_part
        .root_element(&mut workbook.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    let names = &wb.defined_names.as_ref().unwrap().defined_name;
    assert_eq!(names[0].xml_content.as_deref(), Some("Sheet1!#REF!"));
}

#[test]
fn structural_shifts_conditional_formatting() {
    use crate::errors::sdk_err_to_api;
    use xlcore_io::spreadsheetml as x;

    let mut workbook = Workbook::new().unwrap();
    let ws_part = workbook.worksheet_part_for_sheet("Sheet1").unwrap();
    {
        let ws = ws_part
            .root_element_mut(&mut workbook.doc)
            .map_err(sdk_err_to_api)
            .unwrap();
        ws.conditional_formatting.push(x::ConditionalFormatting {
            sequence_of_references: Some(vec!["A1:B5".to_string(), "D10".to_string()]),
            conditional_formatting_rule: vec![x::ConditionalFormattingRule {
                formula: vec![x::Formula(x::XstringType {
                    xml_content: Some("A1>0".to_string()),
                    ..Default::default()
                })],
                ..Default::default()
            }],
            ..Default::default()
        });
    }

    workbook.insert_rows("Sheet1", 1, 2).unwrap();

    let ws_part = workbook.worksheet_part_for_sheet("Sheet1").unwrap();
    let ws = ws_part
        .root_element(&mut workbook.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    let cf = &ws.conditional_formatting[0];
    assert_eq!(
        cf.sequence_of_references.as_deref(),
        Some(&vec!["A3:B7".to_string(), "D12".to_string()][..])
    );
    assert_eq!(
        cf.conditional_formatting_rule[0].formula[0]
            .0
            .xml_content
            .as_deref(),
        Some("A3>0"),
    );
}

#[test]
fn structural_shifts_tables() {
    use crate::errors::sdk_err_to_api;
    use ooxmlsdk::parts::table_definition_part::TableDefinitionPart;
    use xlcore_io::spreadsheetml as x;

    let mut workbook = Workbook::new().unwrap();
    let ws_part = workbook.worksheet_part_for_sheet("Sheet1").unwrap();
    let table_part: TableDefinitionPart = ws_part
        .add_new_part_auto_id(&mut workbook.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    table_part
        .set_root_element(
            &mut workbook.doc,
            x::Table {
                id: 1,
                display_name: "Table1".to_string(),
                reference: "A1:C10".to_string(),
                auto_filter: Some(Box::new(x::AutoFilter {
                    reference: Some("A1:C10".to_string()),
                    ..Default::default()
                })),
                table_columns: Box::new(x::TableColumns::default()),
                ..Default::default()
            },
        )
        .map_err(sdk_err_to_api)
        .unwrap();

    workbook.insert_rows("Sheet1", 2, 5).unwrap();

    let ws_part = workbook.worksheet_part_for_sheet("Sheet1").unwrap();
    let parts: Vec<_> = ws_part.table_definition_parts(&workbook.doc).collect();
    let table = parts[0]
        .root_element(&mut workbook.doc)
        .map_err(sdk_err_to_api)
        .unwrap();
    assert_eq!(table.reference, "A1:C15");
    assert_eq!(
        table.auto_filter.as_ref().unwrap().reference.as_deref(),
        Some("A1:C15"),
    );
}

#[test]
fn layout_reflects_mutated_cells() {
    let mut workbook = Workbook::new().unwrap();
    let outcome = workbook.batch(|tx| {
        tx.set_value("Sheet1!A1", "Label")?;
        tx.set_value("Sheet1!B1", 42.0)?;
        Ok(())
    });
    assert!(outcome.is_ok());
    assert!(outcome.warnings.is_empty());

    let layout = workbook.layout(LayoutOptions::default()).unwrap();
    let sheet = &layout.sheets[0];
    assert_eq!(sheet.max_row, 1);
    assert_eq!(sheet.max_col, 2);
    assert_eq!(sheet.cells.count, 2);
    assert!(sheet.value_pool.iter().any(|value| value == "Label"));
    assert!(sheet.value_pool.iter().any(|value| value == "42"));
}

#[test]
fn copy_range_translates_relative_formulas() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    workbook.set_value("Sheet1!A2", 2.0).unwrap();
    workbook.set_value("Sheet1!A3", 3.0).unwrap();
    workbook.set_formula("Sheet1!B1", "=A1*$C$1").unwrap();
    workbook.set_value("Sheet1!C1", 10.0).unwrap();

    let info = workbook.copy_range("Sheet1!B1", "Sheet1!B2:B3").unwrap();
    assert_eq!(info.formulas[0][0].as_deref(), Some("A2*$C$1"));
    assert_eq!(info.formulas[1][0].as_deref(), Some("A3*$C$1"));
}

#[test]
fn copy_range_to_single_cell_uses_source_shape() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_range_values(
            "Sheet1!A1:B2",
            vec![
                vec![CellValue::Number(1.0), CellValue::Number(2.0)],
                vec![CellValue::Number(3.0), CellValue::Number(4.0)],
            ],
        )
        .unwrap();
    workbook.set_formula("Sheet1!A1", "=B1+1").unwrap();

    let info = workbook.copy_range("Sheet1!A1:B2", "Sheet1!D5").unwrap();
    assert_eq!(info.reference, "D5:E6");
    assert_eq!(info.formulas[0][0].as_deref(), Some("E5+1"));
    assert_eq!(info.values[1][0], CellValue::Number(3.0));
    assert_eq!(info.values[1][1], CellValue::Number(4.0));
}

#[test]
fn copy_range_shape_mismatch() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    let err = workbook
        .copy_range("Sheet1!A1:A2", "Sheet1!C1:D3")
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::ShapeMismatch);
}

#[test]
fn copy_range_across_sheets_keeps_sheet_qualifier() {
    let mut workbook = Workbook::new().unwrap();
    workbook.create_sheet("Other").unwrap();
    workbook.set_value("Sheet1!A1", 5.0).unwrap();
    workbook.set_formula("Sheet1!B1", "=A1+Other!A1").unwrap();

    let info = workbook.copy_range("Sheet1!B1", "Sheet1!B2").unwrap();
    assert_eq!(info.formulas[0][0].as_deref(), Some("A2+Other!A2"));
}

#[test]
fn fill_range_tiles_source_with_translation() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    workbook.set_formula("Sheet1!B1", "=A1*2").unwrap();

    let info = workbook.fill_range("Sheet1!A1:B1", "Sheet1!A1:B4").unwrap();
    assert_eq!(info.values[0][0], CellValue::Number(1.0));
    for r in 1..4 {
        assert_eq!(info.values[r][0], CellValue::Number(1.0));
        assert_eq!(
            info.formulas[r][0].as_deref(),
            None,
            "row {r} col 0 should be value"
        );
        assert_eq!(
            info.formulas[r][1].as_deref(),
            Some(format!("A{}*2", r + 1).as_str())
        );
    }
}

#[test]
fn fill_range_requires_multiple() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    let err = workbook
        .fill_range("Sheet1!A1:A2", "Sheet1!A1:A5")
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::ShapeMismatch);
}

#[test]
fn copy_range_round_trips_through_save() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 1.0).unwrap();
    workbook.set_value("Sheet1!A2", 2.0).unwrap();
    workbook.set_formula("Sheet1!B1", "=A1+10").unwrap();
    workbook.copy_range("Sheet1!B1", "Sheet1!B2").unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(
        reopened.get_cell("Sheet1!B2").unwrap().formula.as_deref(),
        Some("A2+10")
    );
}
