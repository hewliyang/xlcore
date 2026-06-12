use crate::*;

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
