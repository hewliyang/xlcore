use crate::*;

#[test]
fn shapes_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_shape(ShapePatch {
            sheet: "Sheet1".to_string(),
            name: Some("Box".to_string()),
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 1,
                from_row: 1,
                to_column: 5,
                to_row: 6,
                ..Default::default()
            }),
            preset: "roundRect".to_string(),
            fill_color: Some("#4472C4".to_string()),
            line_color: Some("#1F3864".to_string()),
            line_width_emu: Some(19050),
            text: Some("Hello".to_string()),
            text_color: Some("#FFFFFF".to_string()),
            font_size_pt: Some(14.0),
            bold: Some(true),
            rotation_degrees: Some(15.0),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(info.name, "Box");
    assert_eq!(info.preset, "roundRect");
    assert_eq!(info.fill_color.as_deref(), Some("#4472C4"));

    let bytes = wb.save_bytes().unwrap();
    let mut reopened = Workbook::open_bytes(bytes).unwrap();
    let shapes = reopened.shapes(None).unwrap();
    assert_eq!(shapes.len(), 1);
    let s = &shapes[0];
    assert_eq!(s.name, "Box");
    assert_eq!(s.preset, "roundRect");
    assert_eq!(s.fill_color.as_deref(), Some("#4472C4"));
    assert_eq!(s.line_color.as_deref(), Some("#1F3864"));
    assert_eq!(s.text.as_deref(), Some("Hello"));
    assert_eq!(s.anchor.from_column, 1);
    assert_eq!(s.anchor.to_row, 6);
    assert!((s.rotation_degrees - 15.0).abs() < 0.01);

    let id = s.id.clone();
    let removed = reopened.remove_shape("Sheet1", &id).unwrap().unwrap();
    assert_eq!(removed.id, id);
    assert!(reopened.shapes(None).unwrap().is_empty());

    let bytes2 = reopened.save_bytes().unwrap();
    let mut reopened2 = Workbook::open_bytes(bytes2).unwrap();
    assert!(reopened2.shapes(None).unwrap().is_empty());
}

#[test]
fn shapes_emit_alignment_underline_arrow_rotation_box() {
    use std::io::Read;
    let mut wb = Workbook::new().unwrap();
    wb.set_shape(ShapePatch {
        sheet: "Sheet1".into(),
        anchor: AnchorSpec::Cells(ChartAnchor {
            from_column: 1,
            from_row: 1,
            to_column: 6,
            to_row: 8,
            ..Default::default()
        }),
        preset: "roundRect".into(),
        fill_color: Some("#4472C4".into()),
        text: Some("Quarter\nResults".into()),
        align: Some("ctr".into()),
        vertical_align: Some("ctr".into()),
        underline: Some(true),
        rotation_degrees: Some(15.0),
        ..Default::default()
    })
    .unwrap();
    wb.set_shape(ShapePatch {
        sheet: "Sheet1".into(),
        anchor: AnchorSpec::Cells(ChartAnchor {
            from_column: 1,
            from_row: 10,
            to_column: 6,
            to_row: 10,
            ..Default::default()
        }),
        preset: "line".into(),
        line_color: Some("#FF0000".into()),
        line_width_emu: Some(28575),
        tail_end: Some(ShapeLineEnd {
            r#type: Some("triangle".into()),
            w: Some("med".into()),
            len: Some("lg".into()),
        }),
        ..Default::default()
    })
    .unwrap();
    let bytes = wb.save_bytes().unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut s = String::new();
    zip.by_name("xl/drawings/drawing1.xml")
        .unwrap()
        .read_to_string(&mut s)
        .unwrap();
    assert!(s.contains("algn=\"ctr\""), "missing pPr algn: {s}");
    assert!(s.contains("anchor=\"ctr\""), "missing bodyPr anchor: {s}");
    assert!(s.contains("u=\"sng\""), "missing underline: {s}");
    assert!(s.contains("tailEnd"), "missing tailEnd: {s}");
    assert!(s.contains("type=\"triangle\""), "missing arrow type: {s}");
    assert!(s.contains("rot=\"900000\""), "missing rotation: {s}");
    assert!(
        s.contains("<a:off") && s.contains("<a:ext"),
        "rotation xfrm missing off/ext: {s}"
    );
}

#[test]
fn shapes_warn_on_offset_exceeding_referenced_cell() {
    let mut wb = Workbook::new().unwrap();
    wb.set_shape(ShapePatch {
        sheet: "Sheet1".into(),
        anchor: AnchorSpec::Cells(ChartAnchor {
            from_column: 0,
            from_row: 0,
            to_column: 0,
            to_row: 0,
            to_column_offset_emu: Some(5_000_000),
            to_row_offset_emu: Some(5_000_000),
            ..Default::default()
        }),
        preset: "rect".into(),
        fill_color: Some("#4472C4".into()),
        ..Default::default()
    })
    .unwrap();
    let warnings = wb.take_warnings();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(warnings
        .iter()
        .all(|w| w.code == ApiErrorCode::LossyOperation));
    assert!(warnings.iter().any(|w| w.message.contains("column offset")));
    assert!(warnings.iter().any(|w| w.message.contains("row offset")));
}

#[test]
fn shapes_no_warn_when_offsets_fit_cell() {
    let mut wb = Workbook::new().unwrap();
    wb.set_shape(ShapePatch {
        sheet: "Sheet1".into(),
        anchor: AnchorSpec::Cells(ChartAnchor {
            from_column: 0,
            from_row: 0,
            to_column: 3,
            to_row: 3,
            to_column_offset_emu: Some(100_000),
            to_row_offset_emu: Some(50_000),
            ..Default::default()
        }),
        preset: "rect".into(),
        fill_color: Some("#4472C4".into()),
        ..Default::default()
    })
    .unwrap();
    assert!(wb.take_warnings().is_empty());
}

#[test]
fn shapes_reject_unknown_preset() {
    let mut wb = Workbook::new().unwrap();
    let err = wb
        .set_shape(ShapePatch {
            sheet: "Sheet1".to_string(),
            anchor: AnchorSpec::Cells(ChartAnchor::default()),
            preset: "notARealShape".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidShape);
}

#[test]
fn shape_anchor_accepts_a1_range_string() {
    let mut wb = Workbook::new().unwrap();
    let info = wb
        .set_shape(ShapePatch {
            sheet: "Sheet1".to_string(),
            anchor: AnchorSpec::A1("D2:H15".to_string()),
            preset: "rect".to_string(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(info.anchor.from_column, 3);
    assert_eq!(info.anchor.from_row, 1);
    assert_eq!(info.anchor.to_column, 8);
    assert_eq!(info.anchor.to_row, 15);

    let err = wb
        .set_shape(ShapePatch {
            sheet: "Sheet1".to_string(),
            anchor: AnchorSpec::A1("D2".to_string()),
            preset: "rect".to_string(),
            ..Default::default()
        })
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidRef);
}

#[test]
fn shapes_rotation_keeps_anchor_footprint() {
    use std::io::Read;
    let off_ext = |rotation_degrees: Option<f64>| -> String {
        let mut wb = Workbook::new().unwrap();
        wb.set_shape(ShapePatch {
            sheet: "Sheet1".into(),
            anchor: AnchorSpec::Cells(ChartAnchor {
                from_column: 1,
                from_row: 1,
                to_column: 5,
                to_row: 6,
                ..Default::default()
            }),
            preset: "rect".into(),
            fill_color: Some("#4472C4".into()),
            rotation_degrees,
            ..Default::default()
        })
        .unwrap();
        let bytes = wb.save_bytes().unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut s = String::new();
        zip.by_name("xl/drawings/drawing1.xml")
            .unwrap()
            .read_to_string(&mut s)
            .unwrap();
        let start = s.find("<a:off").unwrap();
        let end = s[start..].find("/>").unwrap() + start;
        let off = s[start..=end + 1].to_string();
        let estart = s.find("<a:ext").unwrap();
        let eend = s[estart..].find("/>").unwrap() + estart;
        format!("{off}{}", &s[estart..=eend + 1])
    };

    assert_eq!(
        off_ext(Some(40.0)),
        off_ext(Some(130.0)),
        "rotation angle must not change the anchor footprint; Excel rotates geometry inside the box"
    );
}
