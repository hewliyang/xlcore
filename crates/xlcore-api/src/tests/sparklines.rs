use crate::*;

#[test]
fn sparkline_groups_create_list_remove_roundtrip() {
    let mut wb = Workbook::new().unwrap();
    wb.set_range_values(
        "Sheet1!A1:E3",
        vec![
            vec![1.0.into(), 2.0.into(), 3.0.into(), 4.0.into(), 5.0.into()],
            vec![5.0.into(), 3.0.into(), 4.0.into(), 1.0.into(), 2.0.into()],
            vec![
                1.0.into(),
                (-2.0).into(),
                3.0.into(),
                (-4.0).into(),
                5.0.into(),
            ],
        ],
    )
    .unwrap();

    let info = wb
        .set_sparkline_group(
            "Sheet1",
            SparklineGroupPatch {
                kind: SparklineKind::Line,
                sparklines: vec![
                    SparklineEntry {
                        location: "F1".into(),
                        data_ref: "Sheet1!A1:E1".into(),
                    },
                    SparklineEntry {
                        location: "F2".into(),
                        data_ref: "Sheet1!A2:E2".into(),
                    },
                ],
                markers: Some(true),
                high: Some(true),
                low: Some(true),
                series_color: Some("4472C4".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(info.id.starts_with("Sheet1:"));
    assert_eq!(info.kind, SparklineKind::Line);
    assert_eq!(info.sparklines.len(), 2);

    let _ = wb
        .set_sparkline_group(
            "Sheet1",
            SparklineGroupPatch {
                kind: SparklineKind::Column,
                sparklines: vec![SparklineEntry {
                    location: "F3".into(),
                    data_ref: "Sheet1!A3:E3".into(),
                }],
                negative: Some(true),
                ..Default::default()
            },
        )
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
    let removed = reopened
        .remove_sparkline_group("Sheet1", &first_id)
        .unwrap()
        .unwrap();
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
        .set_sparkline_group(
            "Sheet1",
            SparklineGroupPatch {
                kind: SparklineKind::Line,
                sparklines: vec![],
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSparklineGroup);

    let err = wb
        .set_sparkline_group(
            "Sheet1",
            SparklineGroupPatch {
                kind: SparklineKind::Line,
                sparklines: vec![SparklineEntry {
                    location: "nope".into(),
                    data_ref: "A1:E1".into(),
                }],
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSparklineGroup);

    let info = wb
        .set_sparkline_group(
            "Sheet1",
            SparklineGroupPatch {
                kind: SparklineKind::Line,
                sparklines: vec![SparklineEntry {
                    location: "F1".into(),
                    data_ref: "A1:E1".into(),
                }],
                series_color: Some("#4472c4".into()),
                negative_color: Some("FF0000".into()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(info.series_color.as_deref(), Some("4472C4"));
    assert_eq!(info.negative_color.as_deref(), Some("FF0000"));

    let err = wb
        .set_sparkline_group(
            "Sheet1",
            SparklineGroupPatch {
                kind: SparklineKind::Line,
                sparklines: vec![SparklineEntry {
                    location: "F2".into(),
                    data_ref: "A2:E2".into(),
                }],
                series_color: Some("not-a-color".into()),
                ..Default::default()
            },
        )
        .unwrap_err();
    assert_eq!(err.code, ApiErrorCode::InvalidSparklineGroup);
}
