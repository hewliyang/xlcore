
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

    let recalc = workbook.recalculate().unwrap();
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

#[test]
fn range_round_trip_values_formulas_and_clear() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_range_values(
            "Sheet1!A1:B2",
            vec![
                vec![
                    CellValue::String("Region".into()),
                    CellValue::String("Units".into()),
                ],
                vec![CellValue::String("North".into()), CellValue::Number(10.0)],
            ],
        )
        .unwrap();
    workbook
        .set_range_formulas(
            "Sheet1!C1:C2",
            vec![vec![None], vec![Some("=B2*2".to_string())]],
        )
        .unwrap();

    let range = workbook.get_range("Sheet1!A1:C2").unwrap();
    assert_eq!(range.rows, 2);
    assert_eq!(range.columns, 3);
    assert_eq!(range.reference, "A1:C2");
    assert_eq!(range.values[0][0], CellValue::String("Region".to_string()));
    assert_eq!(range.values[1][1], CellValue::Number(10.0));
    assert_eq!(range.formulas[0][2], None);
    assert_eq!(range.formulas[1][2].as_deref(), Some("B2*2"));

    let recalc = workbook.recalculate().unwrap();
    assert_eq!(
        recalc.cell("Sheet1", "C2").unwrap().value,
        xlcore_engine::CellValue::Number(20.0)
    );

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let reread = reopened.get_range("Sheet1!A1:C2").unwrap();
    assert_eq!(reread.values[1][1], CellValue::Number(10.0));
    assert_eq!(reread.formulas[1][2].as_deref(), Some("B2*2"));
    assert_eq!(reread.values[1][2], CellValue::Number(20.0));

    let cleared = reopened.clear_range("Sheet1!A1:C2").unwrap();
    assert!(cleared
        .values
        .iter()
        .flatten()
        .all(|v| matches!(v, CellValue::Blank)));
    assert!(cleared.formulas.iter().flatten().all(|f| f.is_none()));
}

#[test]
fn clear_modes_respect_target() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 10.0).unwrap();
    workbook.set_formula("Sheet1!B1", "=A1*2").unwrap();
    workbook
        .set_style(
            "Sheet1!A1:B1",
            StylePatch {
                font: Some(FontPatch {
                    bold: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    workbook.recalculate().unwrap();

    let styled_a = workbook.get_cell("Sheet1!A1").unwrap().style_index;
    assert!(styled_a.is_some());

    let only_values = workbook.clear_with("Sheet1!B1", ClearMode::Values).unwrap();
    assert_eq!(only_values.value, CellValue::Blank);
    assert_eq!(only_values.formula.as_deref(), Some("A1*2"));
    assert!(only_values.style_index.is_some());

    let only_formulas = workbook
        .clear_with("Sheet1!B1", ClearMode::Formulas)
        .unwrap();
    assert!(only_formulas.formula.is_none());
    assert!(only_formulas.style_index.is_some());

    let only_styles = workbook.clear_with("Sheet1!A1", ClearMode::Styles).unwrap();
    assert_eq!(only_styles.value, CellValue::Number(10.0));
    assert!(only_styles.style_index.is_none());

    let all = workbook.clear_with("Sheet1!A1", ClearMode::All).unwrap();
    assert_eq!(all.value, CellValue::Blank);
    assert!(all.formula.is_none());
    assert!(all.style_index.is_none());
}

#[test]
fn clear_range_modes_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_range_values(
            "Sheet1!A1:B1",
            vec![vec![CellValue::Number(1.0), CellValue::Number(2.0)]],
        )
        .unwrap();
    workbook
        .set_range_formulas(
            "Sheet1!A2:B2",
            vec![vec![Some("=A1+1".into()), Some("=B1+1".into())]],
        )
        .unwrap();
    workbook
        .set_style(
            "Sheet1!A1:B2",
            StylePatch {
                font: Some(FontPatch {
                    italic: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let cleared_formulas = workbook
        .clear_range_with("Sheet1!A2:B2", ClearMode::Formulas)
        .unwrap();
    assert!(cleared_formulas
        .formulas
        .iter()
        .flatten()
        .all(|f| f.is_none()));

    let cleared_styles = workbook
        .clear_range_with("Sheet1!A1:B1", ClearMode::Styles)
        .unwrap();
    assert!(matches!(
        cleared_styles.values[0][0],
        CellValue::Number(1.0)
    ));

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert!(reopened.get_cell("Sheet1!A2").unwrap().formula.is_none());
    assert!(reopened
        .get_cell("Sheet1!A1")
        .unwrap()
        .style_index
        .is_none());
}

#[test]
fn range_shape_mismatch_is_diagnosed() {
    let mut workbook = Workbook::new().unwrap();
    let err = workbook
        .set_range_values(
            "Sheet1!A1:B2",
            vec![vec![CellValue::Number(1.0), CellValue::Number(2.0)]],
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::ShapeMismatch);
    assert_eq!(err.reference.as_deref(), Some("A1:B2"));
    assert_eq!(err.sheet.as_deref(), Some("Sheet1"));
}

#[test]
fn set_style_applies_font_fill_border_align_and_numfmt() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", "Hello").unwrap();
    workbook.set_value("Sheet1!B1", 1234.5).unwrap();

    let patch = StylePatch {
        font: Some(FontPatch {
            bold: Some(true),
            color: Some("#FF0000".to_string()),
            size: Some(14.0),
            ..Default::default()
        }),
        fill: Some(FillPatch {
            color: Some("E2F0D9".to_string()),
        }),
        border: Some(BorderPatch {
            all: Some(BorderLinePatch {
                style: BorderLineStyle::Thin,
                color: Some("000000".to_string()),
            }),
            ..Default::default()
        }),
        alignment: Some(AlignmentPatch {
            horizontal: Some(HorizontalAlign::Center),
            wrap: Some(true),
            ..Default::default()
        }),
        number_format: Some("#,##0.00".to_string()),
    };
    workbook.set_style("Sheet1!A1:B1", patch).unwrap();
    let a1 = workbook.get_cell("Sheet1!A1").unwrap();
    let b1 = workbook.get_cell("Sheet1!B1").unwrap();
    let idx_a = a1.style_index.unwrap();
    let idx_b = b1.style_index.unwrap();
    assert!(idx_a > 0);
    assert_eq!(idx_a, idx_b);

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(
        reopened.get_cell("Sheet1!A1").unwrap().style_index,
        Some(idx_a)
    );

    let layout = reopened.layout(LayoutOptions::default()).unwrap();
    let xf = &layout.styles.cell_xfs[idx_a as usize];
    let font = &layout.styles.fonts[xf.font_id.unwrap() as usize];
    assert!(font.bold);
    assert_eq!(font.size, Some(14.0));
    assert_eq!(
        font.color.as_ref().and_then(|c| c.rgb.as_deref()),
        Some("FFFF0000")
    );
    let fill = &layout.styles.fills[xf.fill_id.unwrap() as usize];
    assert_eq!(fill.pattern_type.as_deref(), Some("solid"));
    assert_eq!(
        fill.fg_color.as_ref().and_then(|c| c.rgb.as_deref()),
        Some("FFE2F0D9")
    );
    assert!(xf.wrap_text);
    assert_eq!(xf.horizontal_alignment.as_deref(), Some("center"));
    let num_fmt_id = xf.num_fmt_id.unwrap();
    assert_eq!(num_fmt_id, 4);
}

#[test]
fn set_style_dedupes_across_cells_and_invalid_color_errors() {
    let mut workbook = Workbook::new().unwrap();
    let bold = StylePatch {
        font: Some(FontPatch {
            bold: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    workbook.set_style("Sheet1!A1", bold.clone()).unwrap();
    workbook.set_style("Sheet1!B1", bold).unwrap();
    assert_eq!(
        workbook.get_cell("Sheet1!A1").unwrap().style_index,
        workbook.get_cell("Sheet1!B1").unwrap().style_index
    );

    let err = workbook
        .set_style(
            "Sheet1!A1",
            StylePatch {
                fill: Some(FillPatch {
                    color: Some("notacolor".into()),
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::UnsupportedStyle);
}

#[test]
fn merges_add_list_remove_and_overlap_diagnosed() {
    let mut workbook = Workbook::new().unwrap();
    let info = workbook.add_merge("Sheet1!A1:B2").unwrap();
    assert_eq!(info.reference, "A1:B2");
    assert_eq!(info.rows, 2);
    assert_eq!(info.columns, 2);

    workbook.add_merge("Sheet1!C1:D2").unwrap();
    let list = workbook.merges("Sheet1").unwrap();
    assert_eq!(list.len(), 2);

    let err = workbook.add_merge("Sheet1!B2:C3").unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MergeOverlap);
    assert_eq!(err.sheet.as_deref(), Some("Sheet1"));

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.merges("Sheet1").unwrap();
    assert_eq!(
        after
            .iter()
            .map(|m| m.reference.as_str())
            .collect::<Vec<_>>(),
        ["A1:B2", "C1:D2"]
    );

    let removed = reopened.remove_merge("Sheet1!B1").unwrap().unwrap();
    assert_eq!(removed.reference, "A1:B2");
    let removed_exact = reopened.remove_merge("Sheet1!C1:D2").unwrap().unwrap();
    assert_eq!(removed_exact.reference, "C1:D2");
    assert!(reopened.merges("Sheet1").unwrap().is_empty());
    assert!(reopened.remove_merge("Sheet1!A1").unwrap().is_none());
}

#[test]
fn row_and_column_size_visibility_and_freeze_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", "keep").unwrap();
    workbook.set_row_height("Sheet1", 2, 33.0).unwrap();
    workbook.set_row_visible("Sheet1", 3, false).unwrap();
    workbook.set_column_width("Sheet1", 2, 24.5).unwrap();
    workbook.set_column_visible("Sheet1", 4, false).unwrap();
    let freeze = workbook.set_freeze("Sheet1", 1, 2).unwrap();
    assert_eq!(freeze.frozen_rows, 1);
    assert_eq!(freeze.frozen_columns, 2);

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(
        reopened.get_cell("Sheet1!A1").unwrap().value,
        CellValue::String("keep".to_string())
    );
    let got = reopened.get_freeze("Sheet1").unwrap();
    assert_eq!(got.frozen_rows, 1);
    assert_eq!(got.frozen_columns, 2);

    reopened.set_freeze("Sheet1", 0, 0).unwrap();
    let cleared = reopened.get_freeze("Sheet1").unwrap();
    assert_eq!(cleared.frozen_rows, 0);
    assert_eq!(cleared.frozen_columns, 0);

    reopened.set_row_visible("Sheet1", 3, true).unwrap();
    reopened.set_column_visible("Sheet1", 4, true).unwrap();
}

#[test]
fn row_and_column_invalid_indices_diagnosed() {
    let mut workbook = Workbook::new().unwrap();
    assert_eq!(
        workbook.set_row_height("Sheet1", 0, 20.0).unwrap_err().code,
        ApiErrorCode::InvalidRef,
    );
    assert_eq!(
        workbook
            .set_column_width("Sheet1", 0, 10.0)
            .unwrap_err()
            .code,
        ApiErrorCode::InvalidRef,
    );
    assert_eq!(
        workbook
            .set_row_height("Sheet1", 1, f64::NAN)
            .unwrap_err()
            .code,
        ApiErrorCode::InvalidRef,
    );
    assert_eq!(
        workbook.set_row_height("Ghost", 1, 10.0).unwrap_err().code,
        ApiErrorCode::MissingSheet,
    );
}

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
    let recalc = reopened.recalculate().unwrap();
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
    assert_eq!(cf.sequence_of_references.as_deref(), Some(&vec!["A3:B7".to_string(), "D12".to_string()][..]));
    assert_eq!(
        cf.conditional_formatting_rule[0].formula[0].0.xml_content.as_deref(),
        Some("A3>0"),
    );
}

#[test]
fn structural_shifts_tables() {
    use crate::errors::sdk_err_to_api;
    use ooxmlsdk::parts::table_definition_part::TableDefinitionPart;
    use ooxmlsdk::sdk::SdkPart;
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

fn search_fixture() -> Workbook {
    let mut wb = Workbook::new().unwrap();
    wb.create_sheet("Inputs").unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!A3", "NORTHEAST").unwrap();
    wb.set_value("Sheet1!A4", 42.0).unwrap();
    wb.set_value("Sheet1!A5", true).unwrap();
    wb.set_formula("Sheet1!B2", "=SUM(A4:A4)").unwrap();
    wb.set_value("Inputs!A1", "north pole").unwrap();
    wb.set_formula("Inputs!B1", "=AVERAGE(Sheet1!A4:A4)")
        .unwrap();
    wb
}

#[test]
fn search_substring_default_case_insensitive_across_sheets() {
    let mut wb = search_fixture();
    let hits = wb.search("north", SearchOptions::default()).unwrap();
    let refs: Vec<_> = hits
        .iter()
        .map(|m| (m.sheet.as_str(), m.reference.as_str(), m.hit))
        .collect();
    assert_eq!(
        refs,
        vec![
            ("Sheet1", "Sheet1!A2", SearchHit::Value),
            ("Sheet1", "Sheet1!A3", SearchHit::Value),
            ("Inputs", "Inputs!A1", SearchHit::Value),
        ],
    );
    assert_eq!(hits[0].matched, "North");
}

#[test]
fn search_case_sensitive_narrows_results() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "NORTH",
            SearchOptions {
                case_sensitive: true,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A3");
}

#[test]
fn search_exact_mode_requires_full_cell() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "North",
            SearchOptions {
                mode: SearchMode::Exact,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A2");
}

#[test]
fn search_formulas_target_only_matches_formula_text() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "SUM",
            SearchOptions {
                target: SearchTarget::Formulas,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!B2");
    assert_eq!(hits[0].hit, SearchHit::Formula);
    assert_eq!(hits[0].formula.as_deref(), Some("SUM(A4:A4)"));
}

#[test]
fn search_both_target_returns_separate_hits_per_cell() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "total").unwrap();
    wb.set_formula("Sheet1!A2", "=total").unwrap();
    let hits = wb
        .search(
            "total",
            SearchOptions {
                target: SearchTarget::Both,
                ..Default::default()
            },
        )
        .unwrap();
    let kinds: Vec<_> = hits.iter().map(|h| (h.reference.as_str(), h.hit)).collect();
    assert_eq!(
        kinds,
        vec![
            ("Sheet1!A1", SearchHit::Value),
            ("Sheet1!A2", SearchHit::Formula),
        ],
    );
}

#[test]
fn search_wildcard_anchors_full_cell() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "north*",
            SearchOptions {
                mode: SearchMode::Wildcard,
                ..Default::default()
            },
        )
        .unwrap();
    let refs: Vec<_> = hits.iter().map(|m| m.reference.as_str()).collect();
    assert_eq!(refs, vec!["Sheet1!A2", "Sheet1!A3", "Inputs!A1"]);

    let hits = wb
        .search(
            "north",
            SearchOptions {
                mode: SearchMode::Wildcard,
                ..Default::default()
            },
        )
        .unwrap();
    let refs: Vec<_> = hits.iter().map(|m| m.reference.as_str()).collect();
    assert_eq!(refs, vec!["Sheet1!A2"]);
}

#[test]
fn search_regex_mode_and_invalid_pattern_diagnosed() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            r"^N\w+$",
            SearchOptions {
                mode: SearchMode::Regex,
                case_sensitive: true,
                ..Default::default()
            },
        )
        .unwrap();
    let refs: Vec<_> = hits.iter().map(|m| m.reference.as_str()).collect();
    assert_eq!(refs, vec!["Sheet1!A2", "Sheet1!A3"]);

    let err = wb
        .search(
            "[unclosed",
            SearchOptions {
                mode: SearchMode::Regex,
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSearchQuery);
}

#[test]
fn search_matches_numbers_and_booleans_via_text() {
    let mut wb = search_fixture();
    let hits = wb.search("42", SearchOptions::default()).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A4");
    assert_eq!(hits[0].value, CellValue::Number(42.0));

    let hits = wb.search("TRUE", SearchOptions::default()).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A5");
}

#[test]
fn search_respects_sheet_and_range_scope_and_limit() {
    let mut wb = search_fixture();
    let hits = wb
        .search(
            "north",
            SearchOptions {
                sheet: Some("Sheet1".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 2);
    assert!(hits.iter().all(|h| h.sheet == "Sheet1"));

    let hits = wb
        .search(
            "north",
            SearchOptions {
                range: Some("Sheet1!A1:A2".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].reference, "Sheet1!A2");

    let hits = wb
        .search(
            "north",
            SearchOptions {
                max_results: Some(2),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(hits.len(), 2);
}

#[test]
fn search_diagnostics_empty_query_and_missing_sheet() {
    let mut wb = search_fixture();
    let err = wb.search("", SearchOptions::default()).unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSearchQuery);

    let err = wb
        .search(
            "x",
            SearchOptions {
                sheet: Some("Ghost".to_string()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}

#[test]
fn hyperlinks_add_list_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "anthropic").unwrap();
    wb.set_value("Sheet1!B2", "internal").unwrap();

    let info = wb
        .set_hyperlink(
            "Sheet1!A1",
            HyperlinkPatch {
                target: Some("https://anthropic.com".to_string()),
                tooltip: Some("home".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(info.reference, "A1:A1");
    assert_eq!(info.target.as_deref(), Some("https://anthropic.com"));

    wb.set_hyperlink(
        "Sheet1!B2:C3",
        HyperlinkPatch {
            location: Some("Sheet1!Z9".to_string()),
            display: Some("jump".to_string()),
            ..Default::default()
        },
    )
    .unwrap();

    let list = wb.hyperlinks("Sheet1").unwrap();
    assert_eq!(list.len(), 2);

    let err = wb
        .set_hyperlink("Sheet1!D1", HyperlinkPatch::default())
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidHyperlink);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.hyperlinks("Sheet1").unwrap();
    assert_eq!(after.len(), 2);
    let a1 = after.iter().find(|h| h.start_row == 1).unwrap();
    assert_eq!(a1.target.as_deref(), Some("https://anthropic.com"));
    assert_eq!(a1.tooltip.as_deref(), Some("home"));
    let b2 = after.iter().find(|h| h.start_row == 2).unwrap();
    assert_eq!(b2.location.as_deref(), Some("Sheet1!Z9"));
    assert_eq!(b2.reference, "B2:C3");

    let removed = reopened.remove_hyperlink("Sheet1!B3").unwrap();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0].reference, "B2:C3");
    assert_eq!(reopened.hyperlinks("Sheet1").unwrap().len(), 1);

    reopened
        .set_hyperlink(
            "Sheet1!A1",
            HyperlinkPatch {
                target: Some("https://example.com".to_string()),
                ..Default::default()
            },
        )
        .unwrap();
    let list = reopened.hyperlinks("Sheet1").unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].target.as_deref(), Some("https://example.com"));

    let bytes = reopened.save_bytes().unwrap();
    let rels = sheet_rels_xml(&bytes).expect("sheet1 rels present");
    assert!(
        rels.contains("https://example.com"),
        "expected current target in sheet rels: {rels}"
    );
    assert!(
        !rels.contains("https://anthropic.com"),
        "expected orphan anthropic.com rel to be cleaned: {rels}"
    );

    reopened.remove_hyperlink("Sheet1!A1").unwrap();
    assert!(reopened.hyperlinks("Sheet1").unwrap().is_empty());
    let bytes = reopened.save_bytes().unwrap();
    let rels = sheet_rels_xml(&bytes).unwrap_or_default();
    assert!(
        !rels.contains("https://example.com"),
        "expected example.com rel to be cleaned after final remove: {rels}"
    );
}

fn sheet_rels_xml(bytes: &[u8]) -> Option<String> {
    use std::io::{Cursor, Read};
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes.to_vec())).unwrap();
    let mut file = zip.by_name("xl/worksheets/_rels/sheet1.xml.rels").ok()?;
    let mut out = String::new();
    file.read_to_string(&mut out).unwrap();
    Some(out)
}

#[test]
fn defined_names_create_update_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.create_sheet("Inputs").unwrap();

    let info = wb
        .set_defined_name(DefinedNamePatch {
            name: "TaxRate".to_string(),
            formula: "Sheet1!$B$1".to_string(),
            comment: Some("effective rate".to_string()),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(info.name, "TaxRate");
    assert!(info.scope.is_none());

    wb.set_defined_name(DefinedNamePatch {
        name: "LocalRange".to_string(),
        formula: "$A$1:$B$10".to_string(),
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
        formula: "Sheet1!$C$1".to_string(),
        ..Default::default()
    })
    .unwrap();
    let updated = wb
        .defined_names()
        .unwrap()
        .into_iter()
        .find(|d| d.name == "TaxRate")
        .unwrap();
    assert_eq!(updated.formula, "Sheet1!$C$1");
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
fn defined_names_validation_errors() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "".to_string(),
            formula: "Sheet1!$A$1".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidDefinedName);

    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "A1".to_string(),
            formula: "Sheet1!$A$1".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidDefinedName);

    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "has space".to_string(),
            formula: "Sheet1!$A$1".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidDefinedName);

    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "OK".to_string(),
            formula: "   ".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidDefinedName);

    let err = wb
        .set_defined_name(DefinedNamePatch {
            name: "Scoped".to_string(),
            formula: "$A$1".to_string(),
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
        formula: "Inputs!$B$1".to_string(),
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

#[test]
fn comments_add_list_update_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Units").unwrap();
    wb.set_comment(
        "Sheet1!A1",
        CommentPatch {
            text: "Units sold this quarter".to_string(),
            author: Some("Mario".to_string()),
        },
    )
    .unwrap();
    wb.set_comment(
        "Sheet1!B2",
        CommentPatch {
            text: "double check".to_string(),
            author: None,
        },
    )
    .unwrap();

    let list = wb.comments("Sheet1").unwrap();
    assert_eq!(list.len(), 2);
    let a1 = list.iter().find(|c| c.reference == "A1").unwrap();
    assert_eq!(a1.author, "Mario");
    assert_eq!(a1.text, "Units sold this quarter");

    let updated = wb
        .set_comment(
            "Sheet1!A1",
            CommentPatch {
                text: "updated".to_string(),
                author: Some("Mario".to_string()),
            },
        )
        .unwrap();
    assert_eq!(updated.text, "updated");
    assert_eq!(wb.comments("Sheet1").unwrap().len(), 2);

    let empty = wb.set_comment("Sheet1!C3", CommentPatch::default());
    assert_eq!(empty.unwrap_err().code, ApiErrorCode::InvalidComment);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.comments("Sheet1").unwrap();
    assert_eq!(after.len(), 2);
    assert!(after.iter().any(|c| c.reference == "A1" && c.text == "updated"));

    let removed = reopened.remove_comment("Sheet1!A1:B2").unwrap();
    assert_eq!(removed.len(), 2);
    assert!(reopened.comments("Sheet1").unwrap().is_empty());

    let bytes = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes).unwrap();
    assert!(reopened2.comments("Sheet1").unwrap().is_empty());
}

#[test]
fn threaded_notes_add_reply_list_remove_and_round_trip() {
    use crate::ThreadedNotePatch;

    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Units").unwrap();
    let root = wb
        .add_threaded_note(
            "Sheet1!A1",
            ThreadedNotePatch {
                text: "check this".to_string(),
                author: Some("Mario".to_string()),
                date: None,
            },
        )
        .unwrap();
    assert_eq!(root.reference, "A1");
    assert_eq!(root.author, "Mario");
    assert!(root.parent_id.is_none());

    let reply = wb
        .reply_threaded_note(
            &root.id,
            ThreadedNotePatch {
                text: "on it".to_string(),
                author: Some("Luigi".to_string()),
                date: None,
            },
        )
        .unwrap();
    assert_eq!(reply.reference, "A1");
    assert_eq!(reply.parent_id.as_deref(), Some(root.id.as_str()));
    assert_ne!(reply.person_id, root.person_id);

    let empty = wb.add_threaded_note("Sheet1!B2", ThreadedNotePatch::default());
    assert_eq!(empty.unwrap_err().code, ApiErrorCode::InvalidThreadedNote);

    let list = wb.threaded_notes("Sheet1").unwrap();
    assert_eq!(list.len(), 2);
    assert!(list.iter().any(|n| n.text == "check this" && n.author == "Mario"));
    assert!(list.iter().any(|n| n.text == "on it" && n.author == "Luigi"));
    assert!(wb.comments("Sheet1").unwrap().is_empty());

    let bytes = wb.save_bytes().unwrap();
    {
        let cursor = std::io::Cursor::new(&bytes);
        let mut zip = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
        assert!(names.iter().any(|n| n.starts_with("xl/comments") && n.ends_with(".xml")), "classic shadow comments part missing: {names:?}");
        let mut buf = String::new();
        use std::io::Read;
        zip.by_name(names.iter().find(|n| n.starts_with("xl/comments")).unwrap()).unwrap().read_to_string(&mut buf).unwrap();
        assert!(buf.contains("tc="), "classic shadow author tc= missing in {buf}");
        assert!(buf.contains("check this"));
        assert!(!buf.contains("on it"), "replies must not produce a second legacy comment per cell");
    }

    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.threaded_notes("Sheet1").unwrap();
    assert_eq!(after.len(), 2);
    assert!(after.iter().any(|n| n.author == "Luigi" && n.parent_id.is_some()));
    assert!(reopened.comments("Sheet1").unwrap().is_empty());

    let removed = reopened.remove_threaded_thread("Sheet1!A1").unwrap();
    assert_eq!(removed.len(), 2);
    assert!(reopened.threaded_notes("Sheet1").unwrap().is_empty());

    let bytes = reopened.save_bytes().unwrap();
    {
        let cursor = std::io::Cursor::new(&bytes);
        let mut zip = zip::ZipArchive::new(cursor).unwrap();
        let names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().to_string()).collect();
        assert!(!names.iter().any(|n| n.starts_with("xl/comments") && n.ends_with(".xml")), "classic shadow comments part should be gone: {names:?}");
    }
    let mut reopened2 = Workbook::open_bytes(bytes).unwrap();
    assert!(reopened2.threaded_notes("Sheet1").unwrap().is_empty());
}

#[test]
fn threaded_note_shadow_coexists_with_classic_comment() {
    use crate::ThreadedNotePatch;

    let mut wb = Workbook::new().unwrap();
    wb.set_comment(
        "Sheet1!B2",
        CommentPatch { text: "old school".into(), author: Some("Peach".into()) },
    )
    .unwrap();
    wb.add_threaded_note(
        "Sheet1!A1",
        ThreadedNotePatch { text: "modern".into(), author: Some("Mario".into()), date: None },
    )
    .unwrap();

    let classics = wb.comments("Sheet1").unwrap();
    assert_eq!(classics.len(), 1);
    assert_eq!(classics[0].reference, "B2");
    assert_eq!(classics[0].author, "Peach");

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let classics = reopened.comments("Sheet1").unwrap();
    assert_eq!(classics.len(), 1);
    assert_eq!(reopened.threaded_notes("Sheet1").unwrap().len(), 1);

    reopened.set_comment(
        "Sheet1!B2",
        CommentPatch { text: "old school v2".into(), author: Some("Peach".into()) },
    )
    .unwrap();
    assert_eq!(reopened.threaded_notes("Sheet1").unwrap().len(), 1);
    let classics = reopened.comments("Sheet1").unwrap();
    assert_eq!(classics.len(), 1);
    assert_eq!(classics[0].text, "old school v2");

    reopened.remove_comment("Sheet1!B2").unwrap();
    assert!(reopened.comments("Sheet1").unwrap().is_empty());
    assert_eq!(reopened.threaded_notes("Sheet1").unwrap().len(), 1);
}

#[test]
fn comment_emits_vml_legacy_drawing_indicator() {
    let mut wb = Workbook::new().unwrap();
    wb.set_comment(
        "Sheet1!B3",
        CommentPatch { text: "note".into(), author: Some("Mario".into()) },
    )
    .unwrap();
    let bytes = wb.save_bytes().unwrap();

    let cursor = std::io::Cursor::new(&bytes);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();
    let vml_name = names
        .iter()
        .find(|n| n.ends_with(".vml"))
        .unwrap_or_else(|| panic!("vml drawing missing: {names:?}"))
        .clone();
    let mut buf = String::new();
    use std::io::Read;
    zip.by_name(&vml_name).unwrap().read_to_string(&mut buf).unwrap();
    assert!(buf.contains("x:ClientData ObjectType=\"Note\""), "vml missing client data: {buf}");
    assert!(buf.contains("<x:Row>2</x:Row>"), "vml missing row: {buf}");
    assert!(buf.contains("<x:Column>1</x:Column>"), "vml missing column: {buf}");

    let sheet_name = names
        .iter()
        .find(|n| n.starts_with("xl/worksheets/sheet") && n.ends_with(".xml"))
        .unwrap()
        .clone();
    let mut sheet_buf = String::new();
    zip.by_name(&sheet_name).unwrap().read_to_string(&mut sheet_buf).unwrap();
    assert!(sheet_buf.contains("legacyDrawing"), "sheet missing legacyDrawing: {sheet_buf}");

    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    assert_eq!(reopened.comments("Sheet1").unwrap().len(), 1);
}

#[test]
fn auto_filter_set_get_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_range_values(
        "Sheet1!A1:C1",
        vec![vec![
            CellValue::String("Region".into()),
            CellValue::String("Units".into()),
            CellValue::String("Rev".into()),
        ]],
    )
    .unwrap();
    wb.set_range_values(
        "Sheet1!A2:C3",
        vec![
            vec![
                CellValue::String("North".into()),
                CellValue::Number(10.0),
                CellValue::Number(99.0),
            ],
            vec![
                CellValue::String("South".into()),
                CellValue::Number(20.0),
                CellValue::Number(199.0),
            ],
        ],
    )
    .unwrap();

    assert!(wb.auto_filter("Sheet1").unwrap().is_none());

    let info = wb.set_auto_filter("Sheet1!A1:C3").unwrap();
    assert_eq!(info.sheet, "Sheet1");
    assert_eq!(info.reference, "A1:C3");
    assert_eq!(info.start_row, 1);
    assert_eq!(info.end_row, 3);
    assert_eq!(info.end_column, 3);

    let got = wb.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(got.reference, "A1:C3");

    wb.set_auto_filter("Sheet1!A1:B3").unwrap();
    let replaced = wb.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(replaced.reference, "A1:B3");

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(after.reference, "A1:B3");

    let removed = reopened.remove_auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(removed.reference, "A1:B3");
    assert!(reopened.auto_filter("Sheet1").unwrap().is_none());
    assert!(reopened.remove_auto_filter("Sheet1").unwrap().is_none());

    let err = reopened.set_auto_filter("Ghost!A1:B2").unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}

#[test]
fn auto_filter_column_criteria_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_range_values(
        "Sheet1!A1:C1",
        vec![vec![
            CellValue::String("Region".into()),
            CellValue::String("Units".into()),
            CellValue::String("Rev".into()),
        ]],
    )
    .unwrap();
    wb.set_auto_filter("Sheet1!A1:C3").unwrap();

    let info = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Values {
                    values: Vec::new(),
                    blank: true,
                },
            },
        )
        .unwrap();
    assert_eq!(info.column_offset, 0);
    assert!(matches!(info.criteria, AutoFilterCriteria::Values { .. }));

    wb.set_auto_filter_column(
        "Sheet1",
        AutoFilterColumnPatch {
            column_offset: 1,
            hidden_button: None,
            show_button: None,
            criteria: AutoFilterCriteria::Top10 {
                top: true,
                percent: false,
                val: 5.0,
            },
        },
    )
    .unwrap();

    wb.set_auto_filter_column(
        "Sheet1",
        AutoFilterColumnPatch {
            column_offset: 2,
            hidden_button: None,
            show_button: None,
            criteria: AutoFilterCriteria::Custom {
                logical_and: true,
                criteria: vec![
                    AutoFilterCustomCriterion {
                        operator: AutoFilterOperator::GreaterThanOrEqual,
                        value: "50".to_string(),
                    },
                    AutoFilterCustomCriterion {
                        operator: AutoFilterOperator::LessThan,
                        value: "500".to_string(),
                    },
                ],
            },
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(after.columns.len(), 3);
    match &after.columns[0].criteria {
        AutoFilterCriteria::Values { values, blank } => {
            assert!(values.is_empty());
            assert!(*blank);
        }
        other => panic!("expected Values, got {other:?}"),
    }
    match &after.columns[1].criteria {
        AutoFilterCriteria::Top10 {
            top,
            percent,
            val,
        } => {
            assert!(*top);
            assert!(!*percent);
            assert_eq!(*val, 5.0);
        }
        other => panic!("expected Top10, got {other:?}"),
    }
    match &after.columns[2].criteria {
        AutoFilterCriteria::Custom {
            logical_and,
            criteria,
        } => {
            assert!(*logical_and);
            assert_eq!(criteria.len(), 2);
            assert_eq!(criteria[0].operator, AutoFilterOperator::GreaterThanOrEqual);
            assert_eq!(criteria[0].value, "50");
            assert_eq!(criteria[1].operator, AutoFilterOperator::LessThan);
        }
        other => panic!("expected Custom, got {other:?}"),
    }

    let removed = reopened
        .remove_auto_filter_column("Sheet1", 1)
        .unwrap()
        .unwrap();
    assert!(matches!(removed.criteria, AutoFilterCriteria::Top10 { .. }));
    let after_remove = reopened.auto_filter("Sheet1").unwrap().unwrap();
    assert_eq!(after_remove.columns.len(), 2);
}

#[test]
fn auto_filter_column_values_multi_value_round_trip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "H").unwrap();
    wb.set_value("Sheet1!A2", "alpha").unwrap();
    wb.set_value("Sheet1!A3", "beta").unwrap();
    wb.set_value("Sheet1!A4", "gamma").unwrap();
    wb.set_auto_filter("Sheet1!A1:A4").unwrap();

    wb.set_auto_filter_column(
        "Sheet1",
        AutoFilterColumnPatch {
            column_offset: 0,
            hidden_button: None,
            show_button: None,
            criteria: AutoFilterCriteria::Values {
                values: vec!["alpha".into(), "gamma".into()],
                blank: true,
            },
        },
    )
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.auto_filter("Sheet1").unwrap().unwrap();
    match &after.columns[0].criteria {
        AutoFilterCriteria::Values { values, blank } => {
            assert_eq!(values, &vec!["alpha".to_string(), "gamma".to_string()]);
            assert!(*blank);
        }
        other => panic!("expected Values, got {other:?}"),
    }
}

#[test]
fn auto_filter_column_validation_errors() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "H").unwrap();

    let err = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Top10 {
                    top: true,
                    percent: false,
                    val: 5.0,
                },
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidAutoFilter);

    wb.set_auto_filter("Sheet1!A1:B5").unwrap();

    let err = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 5,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Top10 {
                    top: true,
                    percent: false,
                    val: 5.0,
                },
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidAutoFilter);

    let err = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Values {
                    values: vec!["".into()],
                    blank: false,
                },
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidAutoFilter);

    let err = wb
        .set_auto_filter_column(
            "Sheet1",
            AutoFilterColumnPatch {
                column_offset: 0,
                hidden_button: None,
                show_button: None,
                criteria: AutoFilterCriteria::Top10 {
                    top: true,
                    percent: false,
                    val: 0.0,
                },
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidAutoFilter);
}

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

#[test]
fn tables_create_resize_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Units").unwrap();
    wb.set_value("Sheet1!C1", "Price").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!C2", 1.5).unwrap();
    wb.set_value("Sheet1!A3", "South").unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();
    wb.set_value("Sheet1!C3", 2.5).unwrap();

    let info = wb
        .set_table(TablePatch {
            name: "Sales".to_string(),
            reference: Some("Sheet1!A1:C3".to_string()),
            style: Some(TableStylePatch {
                name: Some("TableStyleMedium2".to_string()),
                show_row_stripes: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(info.name, "Sales");
    assert_eq!(info.display_name, "Sales");
    assert_eq!(info.reference, "A1:C3");
    assert_eq!(info.header_row_count, 1);
    assert_eq!(info.totals_row_count, 0);
    assert!(info.has_auto_filter);
    assert_eq!(info.columns.len(), 3);
    assert_eq!(info.columns[0].name, "Region");
    assert_eq!(info.columns[1].name, "Units");
    assert_eq!(info.columns[2].name, "Price");
    let style = info.style.as_ref().unwrap();
    assert_eq!(style.name.as_deref(), Some("TableStyleMedium2"));
    assert!(style.show_row_stripes);

    let resized = wb
        .set_table(TablePatch {
            name: "Sales".to_string(),
            reference: Some("Sheet1!A1:C5".to_string()),
            totals_row_count: Some(1),
            columns: Some(vec![
                TableColumnPatch {
                    name: Some("Region".to_string()),
                    totals_label: Some("Total".to_string()),
                    ..Default::default()
                },
                TableColumnPatch {
                    name: Some("Units".to_string()),
                    totals_function: Some(TableTotalsFunction::Sum),
                    ..Default::default()
                },
                TableColumnPatch {
                    name: Some("Price".to_string()),
                    totals_function: Some(TableTotalsFunction::Average),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(resized.reference, "A1:C5");
    assert_eq!(resized.totals_row_count, 1);
    assert_eq!(resized.columns[1].totals_function, TableTotalsFunction::Sum);
    assert_eq!(
        resized.columns[2].totals_function,
        TableTotalsFunction::Average
    );

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let tables = reopened.tables(None).unwrap();
    assert_eq!(tables.len(), 1);
    let t = &tables[0];
    assert_eq!(t.name, "Sales");
    assert_eq!(t.sheet, "Sheet1");
    assert_eq!(t.reference, "A1:C5");
    assert_eq!(t.totals_row_count, 1);
    assert_eq!(t.columns[0].totals_label.as_deref(), Some("Total"));
    assert_eq!(t.columns[1].totals_function, TableTotalsFunction::Sum);
    assert!(t.has_auto_filter);

    let removed = reopened.remove_table("Sales").unwrap().unwrap();
    assert_eq!(removed.name, "Sales");
    assert!(reopened.tables(None).unwrap().is_empty());

    let bytes2 = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes2).unwrap();
    assert!(reopened2.tables(None).unwrap().is_empty());
}

#[test]
fn tables_reject_overlap_and_invalid_names() {
    let mut wb = Workbook::new().unwrap();
    wb.set_table(TablePatch {
        name: "T1".to_string(),
        reference: Some("Sheet1!A1:B5".to_string()),
        ..Default::default()
    })
    .unwrap();
    let err = wb
        .set_table(TablePatch {
            name: "T2".to_string(),
            reference: Some("Sheet1!B3:D8".to_string()),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidTable);

    let err = wb
        .set_table(TablePatch {
            name: "Bad Name".to_string(),
            reference: Some("Sheet1!E1:F2".to_string()),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidTable);

    let err = wb
        .set_table(TablePatch {
            name: "Other".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidTable);
}

#[test]
fn tables_shift_through_structural_edits() {
    let mut wb = Workbook::new().unwrap();
    wb.set_table(TablePatch {
        name: "T".to_string(),
        reference: Some("Sheet1!B2:D6".to_string()),
        ..Default::default()
    })
    .unwrap();
    wb.insert_rows("Sheet1", 1, 2).unwrap();
    let t = &wb.tables(None).unwrap()[0];
    assert_eq!(t.reference, "B4:D8");
}

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
    assert!(reopened.remove_sheet_protection("Sheet1").unwrap().is_none());

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

    let err = reopened.set_sheet_protection("Ghost", SheetProtectionPatch::default()).unwrap_err();
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

#[test]
fn page_setup_set_read_remove_and_round_trip() {
    let mut wb = Workbook::new().unwrap();
    let initial = wb.page_setup("Sheet1").unwrap();
    assert_eq!(initial.sheet, "Sheet1");
    assert!(initial.page.is_none());
    assert!(initial.margins.is_none());
    assert!(initial.print_options.is_none());
    assert!(initial.header_footer.is_none());

    let info = wb
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                page: Some(PageSetupSettingsPatch {
                    orientation: Some(PageOrientation::Landscape),
                    paper_size: Some(9),
                    scale: Some(85),
                    fit_to_width: Some(1),
                    fit_to_height: Some(0),
                    page_order: Some(PageOrder::OverThenDown),
                    cell_comments: Some(PrintCellComments::AtEnd),
                    errors: Some(PrintErrors::Dash),
                    copies: Some(2),
                    ..Default::default()
                }),
                margins: Some(PageMarginsPatch {
                    left: Some(0.5),
                    right: Some(0.5),
                    top: Some(0.75),
                    bottom: Some(0.75),
                    header: Some(0.3),
                    footer: Some(0.3),
                }),
                print_options: Some(PrintOptionsPatch {
                    horizontal_centered: Some(true),
                    grid_lines: Some(true),
                    headings: Some(true),
                    ..Default::default()
                }),
                header_footer: Some(HeaderFooterPatch {
                    odd_header: Some("&LLeft&CCenter&RRight".to_string()),
                    odd_footer: Some("&CPage &P of &N".to_string()),
                    different_first: Some(true),
                    first_header: Some("&CCover".to_string()),
                    scale_with_doc: Some(false),
                    ..Default::default()
                }),
            },
        )
        .unwrap();
    let page = info.page.as_ref().unwrap();
    assert_eq!(page.orientation, Some(PageOrientation::Landscape));
    assert_eq!(page.scale, Some(85));
    assert_eq!(page.fit_to_width, Some(1));
    assert_eq!(page.copies, Some(2));
    let margins = info.margins.as_ref().unwrap();
    assert!((margins.top - 0.75).abs() < 1e-9);
    let po = info.print_options.as_ref().unwrap();
    assert_eq!(po.horizontal_centered, Some(true));
    let hf = info.header_footer.as_ref().unwrap();
    assert_eq!(hf.odd_header.as_deref(), Some("&LLeft&CCenter&RRight"));
    assert_eq!(hf.different_first, Some(true));

    let updated = wb
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                page: Some(PageSetupSettingsPatch {
                    scale: Some(120),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let page = updated.page.as_ref().unwrap();
    assert_eq!(page.scale, Some(120));
    assert_eq!(page.orientation, Some(PageOrientation::Landscape));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.page_setup("Sheet1").unwrap();
    let page = after.page.as_ref().unwrap();
    assert_eq!(page.scale, Some(120));
    assert_eq!(page.orientation, Some(PageOrientation::Landscape));
    assert_eq!(page.cell_comments, Some(PrintCellComments::AtEnd));
    let hf = after.header_footer.as_ref().unwrap();
    assert_eq!(hf.first_header.as_deref(), Some("&CCover"));
    assert_eq!(hf.scale_with_doc, Some(false));

    let removed = reopened.remove_page_setup("Sheet1").unwrap();
    assert!(removed.page.is_some());
    let cleared = reopened.page_setup("Sheet1").unwrap();
    assert!(cleared.page.is_none());
    assert!(cleared.margins.is_none());
    assert!(cleared.print_options.is_none());
    assert!(cleared.header_footer.is_none());

    let err = reopened
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                page: Some(PageSetupSettingsPatch {
                    scale: Some(5),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidPageSetup);

    let err = reopened
        .set_page_setup(
            "Sheet1",
            SheetPageSetupPatch {
                margins: Some(PageMarginsPatch {
                    left: Some(-0.1),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidPageSetup);

    let err = reopened
        .set_page_setup("Ghost", SheetPageSetupPatch::default())
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::MissingSheet);
}

#[test]
fn conditional_format_add_list_remove_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook.set_value("Sheet1!A1", 5.0).unwrap();
    workbook.set_value("Sheet1!A2", 10.0).unwrap();

    workbook
        .set_conditional_format(
            "Sheet1!A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::CellIs,
                operator: Some(CfOperator::GreaterThan),
                formula1: Some("7".into()),
                dxf: Some(StylePatch {
                    fill: Some(FillPatch {
                        color: Some("#FFEB3B".into()),
                    }),
                    font: Some(FontPatch {
                        bold: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();

    workbook
        .set_conditional_format(
            "Sheet1!A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::Expression,
                formula1: Some("MOD(ROW(),2)=0".into()),
                ..Default::default()
            },
        )
        .unwrap();

    let listed = workbook.conditional_formats("Sheet1").unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].kind, CfRuleKind::CellIs);
    assert_eq!(listed[0].operator, Some(CfOperator::GreaterThan));
    assert_eq!(listed[0].formula1.as_deref(), Some("7"));
    assert!(listed[0].dxf_id.is_some());
    assert_eq!(listed[1].kind, CfRuleKind::Expression);
    assert!(listed[1].priority > listed[0].priority);

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let after = reopened.conditional_formats("Sheet1").unwrap();
    assert_eq!(after.len(), 2);
    assert_eq!(after[0].formula1.as_deref(), Some("7"));
    assert!(after[0].dxf_id.is_some());

    let removed = reopened.clear_conditional_formats("Sheet1!A1:A10").unwrap();
    assert_eq!(removed.len(), 2);
    assert!(reopened.conditional_formats("Sheet1").unwrap().is_empty());
}

#[test]
fn conditional_format_rejects_missing_formula() {
    let mut workbook = Workbook::new().unwrap();
    let err = workbook
        .set_conditional_format(
            "Sheet1!A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::CellIs,
                operator: Some(CfOperator::Equal),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidConditionalFormat);
}

#[test]
fn conditional_format_color_scale_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_conditional_format(
            "Sheet1!A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::ColorScale,
                color_scale: Some(ColorScalePatch {
                    values: vec![
                        CfValueObject { kind: CfValueObjectKind::Min, value: None },
                        CfValueObject { kind: CfValueObjectKind::Percentile, value: Some("50".into()) },
                        CfValueObject { kind: CfValueObjectKind::Max, value: None },
                    ],
                    colors: vec!["#F8696B".into(), "#FFEB84".into(), "#63BE7B".into()],
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let rules = reopened.conditional_formats("Sheet1").unwrap();
    assert_eq!(rules.len(), 1);
    let cs = rules[0].color_scale.as_ref().expect("color_scale");
    assert_eq!(cs.values.len(), 3);
    assert_eq!(cs.values[0].kind, CfValueObjectKind::Min);
    assert_eq!(cs.values[1].kind, CfValueObjectKind::Percentile);
    assert_eq!(cs.values[1].value.as_deref(), Some("50"));
    assert_eq!(cs.colors.len(), 3);
    assert!(cs.colors[0].to_uppercase().ends_with("F8696B"));
}

#[test]
fn conditional_format_data_bar_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_conditional_format(
            "Sheet1!B1:B20",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::DataBar,
                data_bar: Some(DataBarPatch {
                    min: CfValueObject { kind: CfValueObjectKind::Min, value: None },
                    max: CfValueObject { kind: CfValueObjectKind::Max, value: None },
                    color: "#638EC6".into(),
                    min_length: Some(10),
                    max_length: Some(90),
                    show_value: Some(true),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let rules = reopened.conditional_formats("Sheet1").unwrap();
    let db = rules[0].data_bar.as_ref().expect("data_bar");
    assert_eq!(db.min.kind, CfValueObjectKind::Min);
    assert_eq!(db.max.kind, CfValueObjectKind::Max);
    assert_eq!(db.min_length, Some(10));
    assert_eq!(db.max_length, Some(90));
    assert_eq!(db.show_value, Some(true));
    assert!(db.color.to_uppercase().ends_with("638EC6"));
}

#[test]
fn conditional_format_icon_set_round_trip() {
    let mut workbook = Workbook::new().unwrap();
    workbook
        .set_conditional_format(
            "Sheet1!C1:C30",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::IconSet,
                icon_set: Some(IconSetPatch {
                    icon_set: CfIconSetKind::FourTrafficLights,
                    values: vec![
                        CfValueObject { kind: CfValueObjectKind::Percent, value: Some("0".into()) },
                        CfValueObject { kind: CfValueObjectKind::Percent, value: Some("25".into()) },
                        CfValueObject { kind: CfValueObjectKind::Percent, value: Some("50".into()) },
                        CfValueObject { kind: CfValueObjectKind::Percent, value: Some("75".into()) },
                    ],
                    show_value: Some(false),
                    percent: Some(true),
                    reverse: Some(true),
                }),
                ..Default::default()
            },
        )
        .unwrap();

    let bytes = workbook.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let rules = reopened.conditional_formats("Sheet1").unwrap();
    let is = rules[0].icon_set.as_ref().expect("icon_set");
    assert_eq!(is.icon_set, CfIconSetKind::FourTrafficLights);
    assert_eq!(is.values.len(), 4);
    assert_eq!(is.show_value, Some(false));
    assert_eq!(is.percent, Some(true));
    assert_eq!(is.reverse, Some(true));
}

#[test]
fn conditional_format_color_scale_rejects_mismatched_lengths() {
    let mut workbook = Workbook::new().unwrap();
    let err = workbook
        .set_conditional_format(
            "Sheet1!A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::ColorScale,
                color_scale: Some(ColorScalePatch {
                    values: vec![
                        CfValueObject { kind: CfValueObjectKind::Min, value: None },
                        CfValueObject { kind: CfValueObjectKind::Max, value: None },
                    ],
                    colors: vec!["#FF0000".into()],
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidConditionalFormat);
}

#[test]
fn conditional_format_icon_set_rejects_wrong_arity() {
    let mut workbook = Workbook::new().unwrap();
    let err = workbook
        .set_conditional_format(
            "Sheet1!A1:A10",
            ConditionalFormatRulePatch {
                kind: CfRuleKind::IconSet,
                icon_set: Some(IconSetPatch {
                    icon_set: CfIconSetKind::ThreeTrafficLights1,
                    values: vec![
                        CfValueObject { kind: CfValueObjectKind::Percent, value: Some("0".into()) },
                        CfValueObject { kind: CfValueObjectKind::Percent, value: Some("50".into()) },
                    ],
                    show_value: None,
                    percent: None,
                    reverse: None,
                }),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidConditionalFormat);
}

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

#[test]
fn charts_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "Region").unwrap();
    wb.set_value("Sheet1!B1", "Units").unwrap();
    wb.set_value("Sheet1!A2", "North").unwrap();
    wb.set_value("Sheet1!B2", 10.0).unwrap();
    wb.set_value("Sheet1!A3", "South").unwrap();
    wb.set_value("Sheet1!B3", 20.0).unwrap();
    wb.set_value("Sheet1!A4", "East").unwrap();
    wb.set_value("Sheet1!B4", 30.0).unwrap();

    let info = wb
        .set_chart(ChartPatch {
            sheet: "Sheet1".to_string(),
            name: Some("Sales".to_string()),
            kind: ChartKind::Column,
            title: Some("Units by Region".to_string()),
            legend_position: Some(ChartLegendPosition::Bottom),
            categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            series: vec![ChartSeriesPatch {
                name: None,
                name_ref: Some("Sheet1!$B$1".to_string()),
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                ..Default::default()
            }],
            anchor: ChartAnchor {
                from_column: 3,
                from_row: 1,
                to_column: 10,
                to_row: 16,
                ..Default::default()
            },
            category_axis_title: None,
            value_axis_title: None,
            stacking: None,
            data_labels: None,
        })
        .unwrap();
    assert_eq!(info.sheet, "Sheet1");
    assert_eq!(info.kind, ChartKind::Column);
    assert_eq!(info.name, "Sales");
    assert!(!info.id.is_empty());

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 1);
    let chart = &charts[0];
    assert_eq!(chart.kind, ChartKind::Column);
    assert_eq!(chart.sheet, "Sheet1");
    assert_eq!(chart.title.as_deref(), Some("Units by Region"));
    assert_eq!(chart.legend_position, Some(ChartLegendPosition::Bottom));
    assert_eq!(chart.categories_ref.as_deref(), Some("Sheet1!$A$2:$A$4"));
    assert_eq!(chart.series.len(), 1);
    assert_eq!(chart.series[0].name_ref.as_deref(), Some("Sheet1!$B$1"));
    assert_eq!(chart.series[0].values_ref, "Sheet1!$B$2:$B$4");
    assert_eq!(chart.anchor.from_column, 3);
    assert_eq!(chart.anchor.to_row, 16);

    let removed = reopened
        .remove_chart("Sheet1", &chart.id)
        .unwrap()
        .unwrap();
    assert_eq!(removed.id, chart.id);
    assert!(reopened.charts(None).unwrap().is_empty());

    let bytes2 = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes2).unwrap();
    assert!(reopened2.charts(None).unwrap().is_empty());
}

#[test]
fn charts_supports_multiple_kinds() {
    let mut wb = Workbook::new().unwrap();
    let patch = |kind: ChartKind| ChartPatch {
        sheet: "Sheet1".to_string(),
        name: None,
        kind,
        title: None,
        legend_position: None,
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            name: Some("Series 1".to_string()),
            name_ref: None,
            values_ref: "Sheet1!$B$2:$B$4".to_string(),
            x_values_ref: matches!(kind, ChartKind::Scatter | ChartKind::Bubble)
                .then(|| "Sheet1!$A$2:$A$4".to_string()),
            bubble_sizes_ref: matches!(kind, ChartKind::Bubble)
                .then(|| "Sheet1!$C$2:$C$4".to_string()),
            color: None,
        }],
        anchor: ChartAnchor {
            from_column: 1,
            from_row: 1,
            to_column: 5,
            to_row: 10,
            ..Default::default()
        },
        category_axis_title: None,
        value_axis_title: None,
        stacking: None,
        data_labels: None,
    };
    for kind in [
        ChartKind::Column,
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Pie,
        ChartKind::Area,
        ChartKind::Scatter,
        ChartKind::Bubble,
        ChartKind::Doughnut,
    ] {
        let info = wb.set_chart(patch(kind)).unwrap();
        assert_eq!(info.kind, kind);
    }
    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 8);
    let kinds: Vec<ChartKind> = charts.iter().map(|c| c.kind).collect();
    assert!(kinds.contains(&ChartKind::Column));
    assert!(kinds.contains(&ChartKind::Bar));
    assert!(kinds.contains(&ChartKind::Line));
    assert!(kinds.contains(&ChartKind::Pie));
    assert!(kinds.contains(&ChartKind::Area));
    assert!(kinds.contains(&ChartKind::Scatter));
    assert!(kinds.contains(&ChartKind::Bubble));
    assert!(kinds.contains(&ChartKind::Doughnut));
}

#[test]
fn charts_scatter_bubble_doughnut_color_and_axis_titles_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    for r in 2..=4 {
        wb.set_value(format!("Sheet1!A{r}").as_str(), (r as f64) * 1.5).unwrap();
        wb.set_value(format!("Sheet1!B{r}").as_str(), (r as f64) * 2.0).unwrap();
        wb.set_value(format!("Sheet1!C{r}").as_str(), (r as f64) * 5.0).unwrap();
    }

    wb.set_chart(ChartPatch {
        sheet: "Sheet1".to_string(),
        name: Some("Sc".to_string()),
        kind: ChartKind::Scatter,
        title: Some("S".to_string()),
        legend_position: Some(ChartLegendPosition::Right),
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            name: Some("P".to_string()),
            name_ref: None,
            values_ref: "Sheet1!$B$2:$B$4".to_string(),
            x_values_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            bubble_sizes_ref: None,
            color: Some("FF8800".to_string()),
        }],
        anchor: ChartAnchor {
            from_column: 4, from_row: 1, to_column: 12, to_row: 16,
            ..Default::default()
        },
        category_axis_title: Some("X-Axis".to_string()),
        value_axis_title: Some("Y-Axis".to_string()),
        stacking: None,
        data_labels: None,
    })
    .unwrap();

    wb.set_chart(ChartPatch {
        sheet: "Sheet1".to_string(),
        name: Some("Bu".to_string()),
        kind: ChartKind::Bubble,
        title: None,
        legend_position: None,
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            name_ref: Some("Sheet1!$B$1".to_string()),
            values_ref: "Sheet1!$B$2:$B$4".to_string(),
            x_values_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            bubble_sizes_ref: Some("Sheet1!$C$2:$C$4".to_string()),
            ..Default::default()
        }],
        anchor: ChartAnchor { from_column: 1, from_row: 18, to_column: 8, to_row: 30, ..Default::default() },
        category_axis_title: None,
        value_axis_title: None,
        stacking: None,
        data_labels: None,
    })
    .unwrap();

    wb.set_chart(ChartPatch {
        sheet: "Sheet1".to_string(),
        name: Some("Do".to_string()),
        kind: ChartKind::Doughnut,
        title: None,
        legend_position: Some(ChartLegendPosition::None),
        categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
        series: vec![ChartSeriesPatch {
            values_ref: "Sheet1!$B$2:$B$4".to_string(),
            color: Some("#00aacc".to_string()),
            ..Default::default()
        }],
        anchor: ChartAnchor { from_column: 9, from_row: 18, to_column: 16, to_row: 30, ..Default::default() },
        category_axis_title: None,
        value_axis_title: None,
        stacking: None,
        data_labels: None,
    })
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    assert_eq!(charts.len(), 3);

    let sc = charts.iter().find(|c| c.kind == ChartKind::Scatter).unwrap();
    assert_eq!(sc.series[0].x_values_ref.as_deref(), Some("Sheet1!$A$2:$A$4"));
    assert_eq!(sc.series[0].values_ref, "Sheet1!$B$2:$B$4");
    assert_eq!(sc.series[0].color.as_deref(), Some("FF8800"));
    assert_eq!(sc.category_axis_title.as_deref(), Some("X-Axis"));
    assert_eq!(sc.value_axis_title.as_deref(), Some("Y-Axis"));

    let bu = charts.iter().find(|c| c.kind == ChartKind::Bubble).unwrap();
    assert_eq!(bu.series[0].x_values_ref.as_deref(), Some("Sheet1!$A$2:$A$4"));
    assert_eq!(bu.series[0].bubble_sizes_ref.as_deref(), Some("Sheet1!$C$2:$C$4"));

    let dn = charts.iter().find(|c| c.kind == ChartKind::Doughnut).unwrap();
    assert_eq!(dn.categories_ref.as_deref(), Some("Sheet1!$A$2:$A$4"));
    assert_eq!(dn.series[0].color.as_deref(), Some("00AACC"));
}

#[test]
fn authored_parts_emit_xml_prolog_and_bound_root_prefix() {
    use std::io::Read;
    let mut wb = Workbook::new().unwrap();
    wb.set_value("Sheet1!A1", "header").unwrap();
    wb.set_value("Sheet1!A2", "row").unwrap();
    wb.create_sheet("Fresh").unwrap();
    wb.set_value("Fresh!B2", 42.0).unwrap();
    wb.set_style(
        "Sheet1!A1",
        StylePatch {
            font: Some(FontPatch {
                bold: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
    )
    .unwrap();
    wb.set_comment(
        "Sheet1!A1",
        CommentPatch {
            author: Some("a".into()),
            text: "hello".into(),
            ..Default::default()
        },
    )
    .unwrap();
    wb.add_threaded_note(
        "Sheet1!A2",
        ThreadedNotePatch {
            author: Some("a".into()),
            text: "modern".into(),
            ..Default::default()
        },
    )
    .unwrap();
    wb.set_table(TablePatch {
        name: "T".into(),
        reference: Some("Sheet1!A1:A2".into()),
        ..Default::default()
    })
    .unwrap();
    wb.set_chart(ChartPatch {
        sheet: "Sheet1".into(),
        name: Some("C".into()),
        kind: ChartKind::Column,
        title: None,
        legend_position: None,
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            values_ref: "Sheet1!$A$1:$A$2".into(),
            ..Default::default()
        }],
        anchor: ChartAnchor {
            from_column: 3,
            from_row: 1,
            to_column: 9,
            to_row: 12,
            ..Default::default()
        },
        category_axis_title: None,
        value_axis_title: None,
        stacking: None,
        data_labels: None,
    })
    .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let cursor = std::io::Cursor::new(&bytes);
    let mut zip = zip::ZipArchive::new(cursor).unwrap();
    let names: Vec<String> = (0..zip.len())
        .map(|i| zip.by_index(i).unwrap().name().to_string())
        .collect();

    for name in &names {
        if !name.ends_with(".xml") || name.contains(".rels") {
            continue;
        }
        let mut f = zip.by_name(name).unwrap();
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        assert!(
            buf.starts_with("<?xml "),
            "part {name} missing XML prolog; head: {:?}",
            &buf[..buf.len().min(120)]
        );
        let after_prolog = buf.splitn(2, "?>").nth(1).unwrap_or("").trim_start();
        let lt = after_prolog
            .find('<')
            .expect("no root element after prolog");
        let tag = &after_prolog[lt + 1..];
        let tag_end = tag
            .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
            .unwrap_or(tag.len());
        let tag_name = &tag[..tag_end];
        if let Some(colon) = tag_name.find(':') {
            let prefix = &tag_name[..colon];
            let needle = format!("xmlns:{prefix}=");
            assert!(
                tag.contains(&needle),
                "root element <{tag_name}> in {name} uses unbound prefix {prefix:?}; root: {:?}",
                &tag[..tag.len().min(240)]
            );
        }
    }
}

#[test]
fn charts_stacking_roundtrips_for_bar_line_area() {
    let mut wb = Workbook::new().unwrap();
    let base = |kind: ChartKind, stacking: Option<ChartStacking>, row: u32| ChartPatch {
        sheet: "Sheet1".to_string(),
        name: None,
        kind,
        title: None,
        legend_position: None,
        categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
        series: vec![
            ChartSeriesPatch {
                name: Some("S1".to_string()),
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                ..Default::default()
            },
            ChartSeriesPatch {
                name: Some("S2".to_string()),
                values_ref: "Sheet1!$C$2:$C$4".to_string(),
                ..Default::default()
            },
        ],
        anchor: ChartAnchor {
            from_column: 1,
            from_row: row,
            to_column: 8,
            to_row: row + 10,
            ..Default::default()
        },
        category_axis_title: None,
        value_axis_title: None,
        stacking,
        data_labels: None,
    };

    let col_stacked = wb
        .set_chart(base(ChartKind::Column, Some(ChartStacking::Stacked), 1))
        .unwrap();
    assert_eq!(col_stacked.stacking, Some(ChartStacking::Stacked));

    let bar_pct = wb
        .set_chart(base(ChartKind::Bar, Some(ChartStacking::PercentStacked), 14))
        .unwrap();
    assert_eq!(bar_pct.stacking, Some(ChartStacking::PercentStacked));

    let line_stacked = wb
        .set_chart(base(ChartKind::Line, Some(ChartStacking::Stacked), 28))
        .unwrap();
    assert_eq!(line_stacked.stacking, Some(ChartStacking::Stacked));

    let area_pct = wb
        .set_chart(base(ChartKind::Area, Some(ChartStacking::PercentStacked), 42))
        .unwrap();
    assert_eq!(area_pct.stacking, Some(ChartStacking::PercentStacked));

    let col_clustered = wb
        .set_chart(base(ChartKind::Column, Some(ChartStacking::Clustered), 56))
        .unwrap();
    assert_eq!(col_clustered.stacking, Some(ChartStacking::Clustered));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let charts = reopened.charts(None).unwrap();
    let by_kind = |k: ChartKind, row: u32| -> ChartInfo {
        charts
            .iter()
            .find(|c| c.kind == k && c.anchor.from_row == row)
            .cloned()
            .unwrap_or_else(|| panic!("missing {:?} chart at row {row}", k))
    };
    assert_eq!(by_kind(ChartKind::Column, 1).stacking, Some(ChartStacking::Stacked));
    assert_eq!(by_kind(ChartKind::Bar, 14).stacking, Some(ChartStacking::PercentStacked));
    assert_eq!(by_kind(ChartKind::Line, 28).stacking, Some(ChartStacking::Stacked));
    assert_eq!(by_kind(ChartKind::Area, 42).stacking, Some(ChartStacking::PercentStacked));
    assert_eq!(by_kind(ChartKind::Column, 56).stacking, Some(ChartStacking::Clustered));
}

#[test]
fn charts_stacking_on_pie_emits_warning_and_drops() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_chart(ChartPatch {
            sheet: "Sheet1".to_string(),
            name: None,
            kind: ChartKind::Pie,
            title: None,
            legend_position: None,
            categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            series: vec![ChartSeriesPatch {
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                ..Default::default()
            }],
            anchor: ChartAnchor {
                from_column: 1,
                from_row: 1,
                to_column: 5,
                to_row: 10,
                ..Default::default()
            },
            category_axis_title: None,
            value_axis_title: None,
            stacking: Some(ChartStacking::Stacked),
            data_labels: None,
        })
        .unwrap();
    assert_eq!(info.stacking, None);
    let warnings = wb.take_warnings();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].code, ApiErrorCode::LossyOperation);
    assert!(warnings[0].message.contains("stacking"));
}

#[test]
fn charts_scatter_requires_x_values_and_rejects_bad_color() {
    let mut wb = Workbook::new().unwrap();
    let missing_x = wb.set_chart(ChartPatch {
        sheet: "Sheet1".to_string(),
        name: None,
        kind: ChartKind::Scatter,
        title: None,
        legend_position: None,
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            values_ref: "Sheet1!$B$2:$B$4".to_string(),
            ..Default::default()
        }],
        anchor: ChartAnchor::default(),
        category_axis_title: None,
        value_axis_title: None,
        stacking: None,
        data_labels: None,
    });
    assert!(missing_x.is_err());

    let bad_color = wb.set_chart(ChartPatch {
        sheet: "Sheet1".to_string(),
        name: None,
        kind: ChartKind::Column,
        title: None,
        legend_position: None,
        categories_ref: None,
        series: vec![ChartSeriesPatch {
            values_ref: "Sheet1!$B$2:$B$4".to_string(),
            color: Some("nope".to_string()),
            ..Default::default()
        }],
        anchor: ChartAnchor::default(),
        category_axis_title: None,
        value_axis_title: None,
        stacking: None,
        data_labels: None,
    });
    assert!(bad_color.is_err());
}

#[test]
fn charts_data_labels_roundtrip() {
    use xlcore_types::{ChartDataLabelPosition, ChartDataLabels};
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_chart(ChartPatch {
            sheet: "Sheet1".to_string(),
            name: Some("WithLabels".to_string()),
            kind: ChartKind::Column,
            title: None,
            legend_position: None,
            categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
            series: vec![ChartSeriesPatch {
                name: Some("S1".to_string()),
                values_ref: "Sheet1!$B$2:$B$4".to_string(),
                ..Default::default()
            }],
            anchor: ChartAnchor {
                from_column: 1,
                from_row: 1,
                to_column: 8,
                to_row: 12,
                ..Default::default()
            },
            category_axis_title: None,
            value_axis_title: None,
            stacking: None,
            data_labels: Some(ChartDataLabels {
                show_value: Some(true),
                show_category_name: Some(false),
                show_series_name: Some(false),
                show_percent: None,
                show_legend_key: Some(false),
                position: Some(ChartDataLabelPosition::OutsideEnd),
                separator: Some(", ".to_string()),
            }),
        })
        .unwrap();
    let dl = info.data_labels.as_ref().expect("data_labels echoed");
    assert_eq!(dl.show_value, Some(true));
    assert_eq!(dl.position, Some(ChartDataLabelPosition::OutsideEnd));

    let bytes = wb.save_bytes().unwrap();
    let mut wb2 = Workbook::open_bytes(bytes).unwrap();
    let charts = wb2.charts(Some("Sheet1")).unwrap();
    assert_eq!(charts.len(), 1);
    let dl = charts[0].data_labels.as_ref().expect("data_labels survives reopen");
    assert_eq!(dl.show_value, Some(true));
    assert_eq!(dl.show_category_name, Some(false));
    assert_eq!(dl.position, Some(ChartDataLabelPosition::OutsideEnd));
    assert_eq!(dl.separator.as_deref(), Some(", "));
}

#[test]
fn charts_data_labels_pie_show_percent_roundtrip() {
    use xlcore_types::{ChartDataLabelPosition, ChartDataLabels};
    let mut wb = Workbook::new().unwrap();
    wb.set_chart(ChartPatch {
        sheet: "Sheet1".to_string(),
        name: Some("Pie".to_string()),
        kind: ChartKind::Pie,
        title: None,
        legend_position: None,
        categories_ref: Some("Sheet1!$A$2:$A$4".to_string()),
        series: vec![ChartSeriesPatch {
            values_ref: "Sheet1!$B$2:$B$4".to_string(),
            ..Default::default()
        }],
        anchor: ChartAnchor {
            from_column: 1,
            from_row: 1,
            to_column: 6,
            to_row: 10,
            ..Default::default()
        },
        category_axis_title: None,
        value_axis_title: None,
        stacking: None,
        data_labels: Some(ChartDataLabels {
            show_percent: Some(true),
            show_category_name: Some(true),
            position: Some(ChartDataLabelPosition::Center),
            ..Default::default()
        }),
    })
    .unwrap();
    let bytes = wb.save_bytes().unwrap();
    let mut wb2 = Workbook::open_bytes(bytes).unwrap();
    let charts = wb2.charts(Some("Sheet1")).unwrap();
    let dl = charts[0].data_labels.as_ref().unwrap();
    assert_eq!(dl.show_percent, Some(true));
    assert_eq!(dl.show_category_name, Some(true));
    assert_eq!(dl.position, Some(ChartDataLabelPosition::Center));
}

const PNG_1X1: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D,
    0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00,
    0x0D, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
    0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
    0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

#[test]
fn images_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: Some("Logo".to_string()),
            anchor: ChartAnchor {
                from_column: 1,
                from_row: 1,
                to_column: 5,
                to_row: 10,
                ..Default::default()
            },
            bytes: PNG_1X1.to_vec(),
            format: None,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(info.sheet, "Sheet1");
    assert_eq!(info.name, "Logo");
    assert_eq!(info.format, ImageFormat::Png);
    assert_eq!(info.byte_len as usize, PNG_1X1.len());
    assert!(!info.id.is_empty());

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let images = reopened.images(None).unwrap();
    assert_eq!(images.len(), 1);
    assert_eq!(images[0].format, ImageFormat::Png);
    assert_eq!(images[0].name, "Logo");
    assert_eq!(images[0].anchor.from_column, 1);
    assert_eq!(images[0].anchor.to_row, 10);
    assert_eq!(images[0].byte_len as usize, PNG_1X1.len());

    let id = images[0].id.clone();
    let removed = reopened.remove_image("Sheet1", &id).unwrap().unwrap();
    assert_eq!(removed.id, id);
    assert!(reopened.images(None).unwrap().is_empty());

    let bytes2 = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes2).unwrap();
    assert!(reopened2.images(None).unwrap().is_empty());
}

#[test]
fn images_rotation_crop_flip_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: Some("Rotated".to_string()),
            anchor: ChartAnchor {
                from_column: 0,
                from_row: 0,
                to_column: 4,
                to_row: 8,
                ..Default::default()
            },
            bytes: PNG_1X1.to_vec(),
            format: None,
            rotation_degrees: Some(90.0),
            crop_left_pct: Some(10.0),
            crop_top_pct: Some(20.0),
            crop_right_pct: Some(5.0),
            crop_bottom_pct: Some(15.0),
            flip_horizontal: Some(true),
            flip_vertical: Some(false),
        })
        .unwrap();
    assert_eq!(info.rotation_degrees, 90.0);
    assert_eq!(info.crop_left_pct, 10.0);
    assert!(info.flip_horizontal);
    assert!(!info.flip_vertical);

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let images = reopened.images(None).unwrap();
    assert_eq!(images.len(), 1);
    let got = &images[0];
    assert!((got.rotation_degrees - 90.0).abs() < 1e-3);
    assert!((got.crop_left_pct - 10.0).abs() < 1e-3);
    assert!((got.crop_top_pct - 20.0).abs() < 1e-3);
    assert!((got.crop_right_pct - 5.0).abs() < 1e-3);
    assert!((got.crop_bottom_pct - 15.0).abs() < 1e-3);
    assert!(got.flip_horizontal);
    assert!(!got.flip_vertical);
}

#[test]
fn images_rejects_non_finite_rotation_and_crop() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: None,
            anchor: ChartAnchor::default(),
            bytes: PNG_1X1.to_vec(),
            format: None,
            rotation_degrees: Some(f64::NAN),
            crop_left_pct: None,
            crop_top_pct: None,
            crop_right_pct: None,
            crop_bottom_pct: None,
            flip_horizontal: None,
            flip_vertical: None,
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidImage);

    let err = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: None,
            anchor: ChartAnchor::default(),
            bytes: PNG_1X1.to_vec(),
            format: None,
            rotation_degrees: None,
            crop_left_pct: Some(f64::INFINITY),
            crop_top_pct: None,
            crop_right_pct: None,
            crop_bottom_pct: None,
            flip_horizontal: None,
            flip_vertical: None,
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidImage);
}

#[test]
fn images_rejects_empty_bytes_and_unknown_format() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: None,
            anchor: ChartAnchor::default(),
            bytes: Vec::new(),
            format: None,
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidImage);

    let err = wb
        .set_image(ImagePatch {
            sheet: "Sheet1".to_string(),
            name: None,
            anchor: ChartAnchor::default(),
            bytes: b"not an image".to_vec(),
            format: None,
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidImage);
}

#[test]
fn sparkline_groups_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_range_values(
        "Sheet1!A1:E3",
        vec![
            vec![1.0.into(), 2.0.into(), 3.0.into(), 4.0.into(), 5.0.into()],
            vec![5.0.into(), 3.0.into(), 4.0.into(), 1.0.into(), 2.0.into()],
            vec![1.0.into(), (-2.0).into(), 3.0.into(), (-4.0).into(), 5.0.into()],
        ],
    )
    .unwrap();

    let info = wb
        .set_sparkline_group(SparklineGroupPatch {
            sheet: "Sheet1".to_string(),
            kind: SparklineKind::Line,
            sparklines: vec![
                SparklineEntry { location: "F1".into(), data_ref: "Sheet1!A1:E1".into() },
                SparklineEntry { location: "F2".into(), data_ref: "Sheet1!A2:E2".into() },
            ],
            markers: Some(true),
            high: Some(true),
            low: Some(true),
            series_color: Some("4472C4".into()),
            ..Default::default()
        })
        .unwrap();
    assert!(info.id.starts_with("Sheet1:"));
    assert_eq!(info.kind, SparklineKind::Line);
    assert_eq!(info.sparklines.len(), 2);

    let _ = wb
        .set_sparkline_group(SparklineGroupPatch {
            sheet: "Sheet1".to_string(),
            kind: SparklineKind::Column,
            sparklines: vec![SparklineEntry { location: "F3".into(), data_ref: "Sheet1!A3:E3".into() }],
            negative: Some(true),
            ..Default::default()
        })
        .unwrap();

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let groups = reopened.sparkline_groups(None).unwrap();
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].kind, SparklineKind::Line);
    assert_eq!(groups[0].markers, Some(true));
    assert_eq!(groups[0].high, Some(true));
    assert_eq!(groups[0].series_color.as_deref(), Some("4472C4"));
    assert_eq!(groups[0].sparklines.len(), 2);
    assert_eq!(groups[0].sparklines[0].location, "F1");
    assert_eq!(groups[0].sparklines[0].data_ref, "Sheet1!A1:E1");
    assert_eq!(groups[1].kind, SparklineKind::Column);
    assert_eq!(groups[1].negative, Some(true));
    assert_eq!(groups[1].sparklines[0].location, "F3");

    let first_id = groups[0].id.clone();
    let removed = reopened.remove_sparkline_group("Sheet1", &first_id).unwrap().unwrap();
    assert_eq!(removed.id, first_id);

    let bytes2 = reopened.save_bytes().unwrap();
    let mut wb3 = Workbook::open_bytes(bytes2).unwrap();
    let remaining = wb3.sparkline_groups(None).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].kind, SparklineKind::Column);

    let only = remaining[0].id.clone();
    wb3.remove_sparkline_group("Sheet1", &only).unwrap();
    let bytes3 = wb3.save_bytes().unwrap();
    let mut wb4 = Workbook::open_bytes(bytes3).unwrap();
    assert!(wb4.sparkline_groups(None).unwrap().is_empty());
}

#[test]
fn sparkline_groups_validate_inputs() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_sparkline_group(SparklineGroupPatch {
            sheet: "Sheet1".into(),
            kind: SparklineKind::Line,
            sparklines: vec![],
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSparklineGroup);

    let err = wb
        .set_sparkline_group(SparklineGroupPatch {
            sheet: "Sheet1".into(),
            kind: SparklineKind::Line,
            sparklines: vec![SparklineEntry { location: "nope".into(), data_ref: "A1:E1".into() }],
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSparklineGroup);

    let err = wb
        .set_sparkline_group(SparklineGroupPatch {
            sheet: "Sheet1".into(),
            kind: SparklineKind::Line,
            sparklines: vec![SparklineEntry { location: "F1".into(), data_ref: "A1:E1".into() }],
            series_color: Some("#4472C4".into()),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSparklineGroup);
}
