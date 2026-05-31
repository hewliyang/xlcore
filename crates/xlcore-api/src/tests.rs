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
