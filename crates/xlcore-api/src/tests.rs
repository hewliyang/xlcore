    use crate::refs::{
        parse_cell_reference, parse_range_reference, ParsedCellRef, ParsedRangeRef,
    };
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
                    vec![CellValue::String("Region".into()), CellValue::String("Units".into())],
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
        assert_eq!(
            range.values[0][0],
            CellValue::String("Region".to_string())
        );
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
        assert!(cleared.values.iter().flatten().all(|v| matches!(v, CellValue::Blank)));
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

        let only_values = workbook
            .clear_with("Sheet1!B1", ClearMode::Values)
            .unwrap();
        assert_eq!(only_values.value, CellValue::Blank);
        assert_eq!(only_values.formula.as_deref(), Some("A1*2"));
        assert!(only_values.style_index.is_some());

        let only_formulas = workbook
            .clear_with("Sheet1!B1", ClearMode::Formulas)
            .unwrap();
        assert!(only_formulas.formula.is_none());
        assert!(only_formulas.style_index.is_some());

        let only_styles = workbook
            .clear_with("Sheet1!A1", ClearMode::Styles)
            .unwrap();
        assert_eq!(only_styles.value, CellValue::Number(10.0));
        assert!(only_styles.style_index.is_none());

        let all = workbook
            .clear_with("Sheet1!A1", ClearMode::All)
            .unwrap();
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
        assert!(cleared_formulas.formulas.iter().flatten().all(|f| f.is_none()));

        let cleared_styles = workbook
            .clear_range_with("Sheet1!A1:B1", ClearMode::Styles)
            .unwrap();
        assert!(matches!(cleared_styles.values[0][0], CellValue::Number(1.0)));

        let bytes = workbook.save_bytes().unwrap();
        let mut reopened = Workbook::open_bytes(bytes).unwrap();
        assert!(reopened.get_cell("Sheet1!A2").unwrap().formula.is_none());
        assert!(reopened.get_cell("Sheet1!A1").unwrap().style_index.is_none());
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
            font: Some(FontPatch { bold: Some(true), ..Default::default() }),
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
                    fill: Some(FillPatch { color: Some("notacolor".into()) }),
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
            after.iter().map(|m| m.reference.as_str()).collect::<Vec<_>>(),
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
            workbook
                .set_row_height("Sheet1", 0, 20.0)
                .unwrap_err()
                .code,
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
        assert_eq!(workbook.get_cell("Sheet1!A2").unwrap().value, CellValue::Blank);
        assert_eq!(workbook.get_cell("Sheet1!A4").unwrap().value, CellValue::Number(1.0));
        assert_eq!(workbook.get_cell("Sheet1!A5").unwrap().value, CellValue::Number(2.0));
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

        assert_eq!(workbook.get_cell("Sheet1!A1").unwrap().value, CellValue::Number(1.0));
        assert_eq!(workbook.get_cell("Sheet1!A2").unwrap().value, CellValue::Number(4.0));
        assert_eq!(workbook.get_cell("Sheet1!A3").unwrap().value, CellValue::Blank);
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
        assert_eq!(workbook.get_cell("Sheet1!A1").unwrap().value, CellValue::Number(10.0));
        assert_eq!(workbook.get_cell("Sheet1!C1").unwrap().value, CellValue::Number(20.0));
        assert_eq!(
            workbook.get_cell("Sheet1!E1").unwrap().formula.as_deref(),
            Some("A1+C1+D1")
        );

        workbook.delete_columns("Sheet1", 1, 1).unwrap();
        assert_eq!(workbook.get_cell("Sheet1!A1").unwrap().value, CellValue::Blank);
        assert_eq!(workbook.get_cell("Sheet1!B1").unwrap().value, CellValue::Number(20.0));
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
        workbook.set_formula("Sheet1!A1", "=Data!A1+Data!A2").unwrap();

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
            let wb_part = workbook.doc.workbook_part().map_err(sdk_err_to_api).unwrap().clone();
            let wb = wb_part.root_element_mut(&mut workbook.doc).map_err(sdk_err_to_api).unwrap();
            wb.defined_names = Some(x::DefinedNames {
                x_defined_name: vec![
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

        let wb_part = workbook.doc.workbook_part().map_err(sdk_err_to_api).unwrap().clone();
        let wb = wb_part.root_element(&mut workbook.doc).map_err(sdk_err_to_api).unwrap();
        let names = &wb.defined_names.as_ref().unwrap().x_defined_name;
        assert_eq!(names[0].xml_content.as_deref(), Some("Sheet1!$A$1:$B$13"));
        assert_eq!(names[1].xml_content.as_deref(), Some("$A$1:$B$13"));
        assert_eq!(names[2].xml_content.as_deref(), Some("Data!$C$5"));

        workbook.delete_rows("Sheet1", 1, 1000).unwrap();
        let wb_part = workbook.doc.workbook_part().map_err(sdk_err_to_api).unwrap().clone();
        let wb = wb_part.root_element(&mut workbook.doc).map_err(sdk_err_to_api).unwrap();
        let names = &wb.defined_names.as_ref().unwrap().x_defined_name;
        assert_eq!(names[0].xml_content.as_deref(), Some("Sheet1!#REF!"));
    }

    #[test]
    fn structural_shifts_conditional_formatting() {
        use crate::errors::sdk_err_to_api;
        use ooxmlsdk::simple_type::ListValue;
        use xlcore_io::spreadsheetml as x;

        let mut workbook = Workbook::new().unwrap();
        let ws_part = workbook.worksheet_part_for_sheet("Sheet1").unwrap();
        {
            let ws = ws_part.root_element_mut(&mut workbook.doc).map_err(sdk_err_to_api).unwrap();
            ws.x_conditional_formatting.push(x::ConditionalFormatting {
                sequence_of_references: Some(ListValue(vec![
                    "A1:B5".to_string(),
                    "D10".to_string(),
                ])),
                x_cf_rule: vec![x::ConditionalFormattingRule {
                    x_formula: vec![x::Formula {
                        xml_content: Some("A1>0".to_string()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            });
        }

        workbook.insert_rows("Sheet1", 1, 2).unwrap();

        let ws_part = workbook.worksheet_part_for_sheet("Sheet1").unwrap();
        let ws = ws_part.root_element(&mut workbook.doc).map_err(sdk_err_to_api).unwrap();
        let cf = &ws.x_conditional_formatting[0];
        let sqref = cf.sequence_of_references.as_ref().unwrap();
        assert_eq!(sqref.0, vec!["A3:B7".to_string(), "D12".to_string()]);
        assert_eq!(
            cf.x_cf_rule[0].x_formula[0].xml_content.as_deref(),
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
        let table = parts[0].root_element(&mut workbook.doc).map_err(sdk_err_to_api).unwrap();
        assert_eq!(table.reference, "A1:C15");
        assert_eq!(
            table.auto_filter.as_ref().unwrap().reference.as_deref(),
            Some("A1:C15"),
        );
    }

    #[test]
    fn layout_reflects_mutated_cells() {
        let mut workbook = Workbook::new().unwrap();
        workbook
            .batch(|tx| {
                tx.set_value("Sheet1!A1", "Label")?;
                tx.set_value("Sheet1!B1", 42.0)?;
                Ok(())
            })
            .unwrap();

        let layout = workbook.layout(LayoutOptions::default()).unwrap();
        let sheet = &layout.sheets[0];
        assert_eq!(sheet.max_row, 1);
        assert_eq!(sheet.max_col, 2);
        assert_eq!(sheet.cells.count, 2);
        assert!(sheet.value_pool.iter().any(|value| value == "Label"));
        assert!(sheet.value_pool.iter().any(|value| value == "42"));
    }
